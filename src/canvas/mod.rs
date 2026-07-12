#![allow(clippy::redundant_locals)]
mod viewport;
pub use viewport::Viewport;

mod grid;
mod modes;
mod selection;
pub mod settings;
mod state;
mod text_edit;

use modes::{draw_pointer_down, draw_pointer_up};
use selection::{select_pointer_down, select_pointer_move, select_pointer_up};
pub use state::combined_bounds;
pub use state::CanvasMode;
use state::{hit_and_erase, CanvasInputs, CanvasState};
pub use state::CropExportCallback;

use crate::model::*;
use crate::ui::dock::Tool;
use settings::CanvasSettings;
use crate::util::window_size;
use leptos::ev;
use leptos::*;
use std::rc::Rc;

// ── Constants ──────────────────────────────────────────────────────────────
/// Minimum distance (world-space) a drag must travel before it is recognised as a drag
/// rather than a click. Prevents accidental element creation from tiny movements.
const MIN_DRAG_DIST: f64 = 3.0;

/// Multiplier applied per scroll tick when zooming in (1.1×). Zoom-out uses its reciprocal.
const ZOOM_FACTOR: f64 = 1.1;
/// Minimum allowed zoom level (10 %).
const ZOOM_MIN: f64 = 0.1;
/// Maximum allowed zoom level (2000 %).
const ZOOM_MAX: f64 = 20.0;
/// `stroke-dasharray` used for in-progress draw previews and selection marquee.
pub(crate) const DASH_PREVIEW: &str = "4 2";

// ────────────────────────────────────────────────────────────────────────────

/// Root SVG canvas component.
///
/// Owns the full `<svg>` element and handles all pointer / keyboard interaction.
/// Every pointer event is **routed by mode** (`Select` / `Hand` / `Draw`) to a
/// dedicated handler in [`modes`] through a simple match-dispatch pattern:
///
/// ```ignore
/// on_pointer_down → match canvas_mode {
///     Select → modes::select_pointer_down(…)
///     Hand   → (inline) set pan_anchor
///     Draw   → modes::draw_pointer_down(…)
/// }
/// ```
///
/// Behaviour is driven by signals passed in from the parent (`app.rs`):
///
/// | Signal | Role |
/// |--------|------|
/// | `cursor_screen` | Latest pointer position in CSS pixels relative to the SVG |
/// | `cursor_world`  | Latest pointer position in world-space (transformed through viewport) |
/// | `viewport`      | Camera – offset + zoom |
/// | `canvas_mode`   | Which interaction mode is active (Select / Hand / Draw) |
/// | `selected_tool` | Active drawing tool (Rectangle, Ellipse, Line, Arrow, Text, Freehand) |
/// | `selected_color` | Stroke / fill colour for newly created elements |
/// | `scene`         | The document model – all elements live here |
/// | `eraser_active` | Whether the eraser toggle is pressed |
/// | `push_snapshot` | Callback: saves an undo snapshot before mutating the scene |
///
/// Internal state (drag tracking, selection, text editing, grid) lives in
/// [`state::CanvasState`] and [`state::CanvasInputs`]; reusable render helpers
/// live in the sibling modules [`grid`], [`text_edit`], [`selection`], and [`state`].
#[component]
pub fn Canvas(
    cursor_screen: RwSignal<(f64, f64)>,
    cursor_world: RwSignal<(f64, f64)>,
    viewport: RwSignal<Viewport>,
    selected_tool: RwSignal<Tool>,
    selected_color: RwSignal<ShapeColor>,
    canvas_mode: RwSignal<CanvasMode>,
    scene: RwSignal<Scene>,
    eraser_active: RwSignal<bool>,
    settings: RwSignal<CanvasSettings>,
    push_snapshot: Rc<dyn Fn()>,
    export_crop_active: RwSignal<bool>,
    on_crop_export: RwSignal<Option<CropExportCallback>>,
    selected_ids: RwSignal<Vec<ElementId>>,
) -> impl IntoView {
    let mut st = CanvasState::new();
    st.selected_ids = selected_ids;
    let props = CanvasInputs {
        cursor_screen,
        cursor_world,
        viewport,
        selected_tool,
        selected_color,
        canvas_mode,
        scene,
        eraser_active,
        settings,
        push_snapshot,
        export_crop_active,
        on_crop_export,
    };

    st.screen_size.set(window_size());
    let _ = window_event_listener(ev::resize, {
        let ss = st.screen_size;
        move |_| ss.set(window_size())
    });
    let _ = window_event_listener(ev::keydown, {
        let sp = st.shift_pressed;
        let ap = st.alt_pressed;
        move |ev: ev::KeyboardEvent| {
            if ev.key() == "Shift" {
                sp.set(true);
            }
            if ev.key() == "Alt" {
                ap.set(true);
            }
        }
    });
    let _ = window_event_listener(ev::keyup, {
        let sp = st.shift_pressed;
        let ap = st.alt_pressed;
        move |ev: ev::KeyboardEvent| {
            if ev.key() == "Shift" {
                sp.set(false);
            }
            if ev.key() == "Alt" {
                ap.set(false);
            }
        }
    });

    let commit_edit = text_edit::make_commit_edit(
        st.editing_id,
        st.edit_text,
        props.scene,
        st.textarea_ref,
        props.viewport,
    );

    // Switching mode (Select/Hand/Draw) or active tool used to leave whatever
    // was in progress untouched: an active selection stayed highlighted after
    // switching to a draw tool, and a just-opened, still-empty text box stayed
    // on screen after switching away from the Text tool. Whenever the mode or
    // tool changes, clear the selection, and either commit or discard whatever
    // text edit was in progress — discard if nothing was typed (an empty Text
    // element has no reason to exist), commit if the user had actually typed
    // something (so switching tools mid-edit doesn't lose typed text).
    {
        let st = st;
        let props = props.clone();
        let commit_edit = commit_edit.clone();
        create_effect(move |prev: Option<(CanvasMode, Tool)>| {
            let mode = props.canvas_mode.get();
            let tool = props.selected_tool.get();
            if let Some(prev) = prev {
                if prev != (mode, tool) {
                    if props.export_crop_active.get_untracked() {
                        props.export_crop_active.set(false);
                        props.on_crop_export.set(None);
                        st.select_anchor.set(None);
                        st.last_world.set(None);
                    }
                    if !st.selected_ids.get_untracked().is_empty() {
                        st.selected_ids.set(Vec::new());
                    }
                    if let Some(id) = st.editing_id.get_untracked() {
                        if st.edit_text.get_untracked().is_empty() {
                            props.scene.update(|s| s.remove_by_id(id));
                            st.editing_id.set(None);
                            st.edit_text.set(String::new());
                        } else {
                            commit_edit();
                        }
                    }
                }
            }
            (mode, tool)
        });
    }

    // ── Helper ───────────────────────────────────────────────────────────
    // Convert a DOM pointer event into world-space coordinates.
    // 1. Extracts `offset_x / offset_y` from the event (CSS pixels relative to the SVG).
    // 2. Stores them in `cursor_screen`.
    // 3. Transforms screen → world via `Viewport::screen_to_world`.
    // 4. Stores the result in `cursor_world` and returns it.
    //
    // All three pointer handlers call this as their first step so that
    // `cursor_screen` / `cursor_world` always reflect the most recent event.
    let update_world = move |ev: &ev::PointerEvent| -> (f64, f64) {
        let screen = (ev.offset_x() as f64, ev.offset_y() as f64);
        props.cursor_screen.set(screen);
        let world = props
            .viewport
            .get()
            .screen_to_world(screen, st.screen_size.get());
        props.cursor_world.set(world);
        world
    };

    // ── Pointer down ─────────────────────────────────────────────────────
    // Primary entry-point for all pointer-down events on the `<svg>` element.
    //
    // Routing:
    // 1. If a text edit is active (`editing_id` is set), the event is ignored
    //    (the textarea overlay handles its own input).
    // 2. Updates world coordinates via `update_world`.
    // 3. If the eraser toggle is on, snapshots the scene and erases the
    //    topmost element under the cursor.
    // 4. Otherwise, dispatches by `CanvasMode`:
    //    - `Hand` → stores the initial client position for panning.
    //    - `Select` → `modes::select_pointer_down` – handles double-click text
    //      editing, handle grab (resize/move/rotate), element selection,
    //      and marquee anchor.
    //    - `Draw` → `modes::draw_pointer_down` – creates a new element
    //      (snapshot taken before creation) and enters the drawing state.
    let on_pointer_down = {
        let mut st = st;
        let mut props = props.clone();
        let update_world = update_world;
        move |ev: ev::PointerEvent| {
            if st.editing_id.get().is_some() {
                return;
            }
            let mode = props.canvas_mode.get();
            let world = update_world(&ev);

            if props.eraser_active.get() {
                (props.push_snapshot)();
                st.erasing.set(true);
                hit_and_erase(world, props.scene);
                return;
            }

            if props.export_crop_active.get() {
                st.select_anchor.set(Some(world));
                return;
            }

            match mode {
                CanvasMode::Hand => {
                    st.pan_anchor
                        .set(Some((ev.client_x() as f64, ev.client_y() as f64)));
                }
                CanvasMode::Select => {
                    select_pointer_down(&ev, world, &mut st, &mut props);
                }
                CanvasMode::Draw => {
                    draw_pointer_down(&ev, world, &mut st, &mut props);
                }
            }
        }
    };

    // ── Pointer move ─────────────────────────────────────────────────────
    // Handler for every pointer-move event on the SVG.
    //
    // Routing:
    // 1. Updates world coordinates.
    // 2. If the eraser is active and the button is held, erases elements
    //    under the cursor (continuous erasing).
    // 3. Dispatches by mode:
    //    - `Hand` → pans the viewport by the delta between successive
    //      client-space positions.
    //    - `Select` → `modes::select_pointer_move` – handles resize drag
    //      (with per-frame deltas for scale), rotation drag (angle tracking
    //      with optional snap), and plain move (offset).
    //    - `Draw` → for `Freehand` tool, delegates to
    //      `UpdateDrag::update_drag` which appends the latest point to
    //      the stroke path. Other tools do nothing during move (the preview
    //      is handled reactively by the drawing preview closure).
    let on_pointer_move = {
        let mut st = st;
        let mut props = props.clone();
        let update_world = update_world;
        move |ev: ev::PointerEvent| {
            let mode = props.canvas_mode.get();
            let world = update_world(&ev);

            if props.eraser_active.get() && st.erasing.get() {
                hit_and_erase(world, props.scene);
            }

            if props.export_crop_active.get() {
                st.last_world.set(Some(world));
                return;
            }

            match mode {
                CanvasMode::Hand => {
                    if let Some((ax, ay)) = st.pan_anchor.get() {
                        let dx = ev.client_x() as f64 - ax;
                        let dy = ev.client_y() as f64 - ay;
                        props.viewport.update(|vp| {
                            vp.offset_x -= dx / vp.zoom;
                            vp.offset_y -= dy / vp.zoom;
                        });
                        st.pan_anchor
                            .set(Some((ev.client_x() as f64, ev.client_y() as f64)));
                    }
                }
                CanvasMode::Select => {
                    select_pointer_move(world, &ev, &mut st, &mut props);
                }
                CanvasMode::Draw => {
                    if let Some(ref state) = st.drawing.get() {
                        if state.tool == Tool::Freehand {
                            props.scene.update(|s| {
                                if let Some(el) = s.elements_mut().last_mut() {
                                    el.update_drag(Point::from(world), Point::from(state.anchor), ev.shift_key());
                                }
                            });
                        }
                    }
                }
            }
        }
    };

    // ── Pointer up ───────────────────────────────────────────────────────
    // Handler for every pointer-up event on the SVG.
    //
    // Routing:
    // 1. If the eraser is active, performs one final erase at the release point.
    // 2. Resets the `erasing` flag.
    // 3. Dispatches by mode:
    //    - `Hand` → clears the pan anchor (stops panning).
    //    - `Select` → `modes::select_pointer_up` – if a move/resize/rotate
    //      drag just finished, optionally snaps to grid (shift held), then
    //      clears all drag state. If a marquee drag just finished, selects
    //      all elements fully inside the rectangle.
    //    - `Draw` → `modes::draw_pointer_up` – finalises the element from
    //      the drag parameters (skipped for Freehand which is built incrementally,
    //      and skipped if the drag distance is below `MIN_DRAG_DIST`).
    let on_pointer_up = {
        let mut st = st;
        let mut props = props.clone();
        let update_world = update_world;
        move |ev: ev::PointerEvent| {
            if props.eraser_active.get() {
                let world = update_world(&ev);
                hit_and_erase(world, props.scene);
            }
            st.erasing.set(false);

            if props.export_crop_active.get() {
                if let Some(anchor) = st.select_anchor.get() {
                    let current = st.last_world.get().unwrap_or(anchor);
                    let dx = current.0 - anchor.0;
                    let dy = current.1 - anchor.1;
                    if dx.hypot(dy) >= MIN_DRAG_DIST {
                        let x = anchor.0.min(current.0);
                        let y = anchor.1.min(current.1);
                        let w = (current.0 - anchor.0).abs().max(1.0);
                        let h = (current.1 - anchor.1).abs().max(1.0);
                        if let Some(cb) = props.on_crop_export.get() {
                            cb((x, y, w, h));
                            props.on_crop_export.set(None);
                        }
                    }
                }
                props.export_crop_active.set(false);
                st.select_anchor.set(None);
                st.last_world.set(None);
                return;
            }

            let world = update_world(&ev);
            match props.canvas_mode.get() {
                CanvasMode::Hand => {
                    st.pan_anchor.set(None);
                }
                CanvasMode::Select => {
                    select_pointer_up(&ev, world, &mut st, &mut props);
                }
                CanvasMode::Draw => {
                    draw_pointer_up(&ev, world, &mut st, &mut props);
                }
            }
        }
    };

    // Zoom the viewport in/out centred on the cursor position.
    //
    // `delta_y < 0` (scroll up) → zoom in by `ZOOM_FACTOR`.
    // `delta_y > 0` (scroll down) → zoom out by `1 / ZOOM_FACTOR`.
    //
    // The zoom is clamped to `ZOOM_MIN` / `ZOOM_MAX`.
    // After changing `zoom`, the offset is adjusted so the world-space point
    // under the cursor remains stationary (pinch-zoom-at-point behaviour).
    let on_wheel = {
        let st = st;
        let props = props.clone();
        move |ev: ev::WheelEvent| {
            ev.prevent_default();
            let screen = props.cursor_screen.get();
            let (sw, sh) = st.screen_size.get();
            let factor = if ev.delta_y() < 0.0 {
                ZOOM_FACTOR
            } else {
                1.0 / ZOOM_FACTOR
            };
            props.viewport.update(|vp| {
                let world = vp.screen_to_world(screen, (sw, sh));
                vp.zoom = (vp.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
                vp.offset_x = world.0 - (screen.0 - sw / 2.0) / vp.zoom;
                vp.offset_y = world.1 - (screen.1 - sh / 2.0) / vp.zoom;
            });
        }
    };

    // Compute the SVG `viewBox` string from the current viewport and screen size.
    // Re-evaluated reactively whenever the viewport or window dimensions change.
    let view_box = {
        let st = st;
        let props = props.clone();
        move || {
            let (w, h) = st.screen_size.get();
            props.viewport.get().to_view_box(w, h)
        }
    };

    // Reactive preview of the element currently being drawn.
    //
    // Only shown when `canvas_mode == Draw` and the drag has exceeded
    // `MIN_DRAG_DIST`. Freehand elements are excluded (they are built
    // incrementally in `on_pointer_move` and don't need a preview).
    //
    // The preview element is constructed via the same `FromDrag` code path
    // as the final element, but rendered with a dashed stroke to indicate
    // it is not yet committed.
    let drawing_preview = {
        let st = st;
        let props = props.clone();
        move || {
            if props.canvas_mode.get() != CanvasMode::Draw {
                return None;
            }
            let state = st.drawing.get()?;
            if state.tool == Tool::Freehand {
                return None;
            }
            let world = props.cursor_world.get();
            let dx = world.0 - state.anchor.0;
            let dy = world.1 - state.anchor.1;
            if dx.hypot(dy) < MIN_DRAG_DIST {
                return None;
            }
            let shift = st.shift_pressed.get();
            let anchor = Point::from(state.anchor);
            let world_pt = Point::from(world);
            let el: Element = match state.tool {
                Tool::Rectangle => Rectangle::from_drag(anchor, world_pt, state.color, shift).into(),
                Tool::Ellipse => Ellipse::from_drag(anchor, world_pt, state.color, shift).into(),
                Tool::Line => Line::from_drag(anchor, world_pt, state.color, shift).into(),
                Tool::Arrow => Arrow::from_drag(anchor, world_pt, state.color, shift).into(),
                Tool::Text => Text::from_drag(anchor, world_pt, state.color, shift).into(),
                Tool::Freehand => Freehand::from_drag(anchor, world_pt, state.color, shift).into(),
            };
            Some(view! { <g stroke-dasharray={DASH_PREVIEW}>{el.render(props.viewport.get().zoom)}</g> }.into_view())
        }
    };

    // ── Template ─────────────────────────────────────────────────────────
    let svg_ref = st.svg_ref;
    view! {
        <svg
            node_ref=svg_ref
            width="100%"
            height="100%"
            style="display: block; user-select: none; -webkit-user-select: none;"
            viewBox=view_box
            on:pointerdown=on_pointer_down
            on:pointermove=on_pointer_move
            on:pointerup=on_pointer_up
            on:wheel=on_wheel
        >
            {grid::grid_overlay(props.settings)}

            // NB: this is deliberately NOT a <For> loop keyed by ElementId.
            // Leptos's <For> only re-invokes `children` for a newly-inserted
            // key, not for an existing key whose data mutated in place — and
            // every drag, resize, and text-commit in this app mutates an
            // existing element by id without changing its key. A real
            // fine-grained fix would need each element behind its own signal
            // (e.g. Vec<RwSignal<Element>> instead of RwSignal<Scene>), which
            // is a model restructuring, not a one-line change.
            {let props = props.clone(); move || {
                props.scene.get().elements().iter().map(|el| {
                    let zoom = props.viewport.get().zoom;
                    view! { <g pointer-events="none">{el.render(zoom)}</g> }.into_view()
                }).collect_view()
            }}

            {drawing_preview}
            {selection::selection_preview_overlay(st.select_anchor, props.cursor_world)}
            {selection::selection_handle_overlay(st.selected_ids, props.scene, st.overlay_freeze, st.rotation_delta)}
        </svg>

        {text_edit::text_edit_overlay(
            st.editing_id, st.edit_text, st.textarea_ref, props.scene,
            props.viewport, st.screen_size, commit_edit,
        )}

    }
}
