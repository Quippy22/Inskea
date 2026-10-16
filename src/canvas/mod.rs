#![allow(clippy::redundant_locals)]
mod viewport;
pub use viewport::Viewport;

mod grid;
mod modes;
mod selection;
pub mod settings;
mod smart_guides;
mod state;
mod text_edit;

pub use crate::model::interaction::combined_bounds;
use modes::{draw_pointer_down, draw_pointer_up};
use selection::{select_pointer_down, select_pointer_move, select_pointer_up};
pub use state::CanvasMode;
pub use state::CropExportCallback;
pub use state::DragOperation;
use state::{hit_and_erase, CanvasInputs, CanvasState};

use crate::model::*;
use crate::ui::cursor::cursor_style;
use crate::ui::dock::Tool;
use crate::util::window_size;
use leptos::ev;
use leptos::*;
use settings::CanvasSettings;
use std::rc::Rc;

// ── Constants ──────────────────────────────────────────────────────────────
/// Minimum distance (world-space) a drag must travel before it is recognised as a drag
/// rather than a click. Prevents accidental element creation from tiny movements.
pub(crate) const MIN_DRAG_DIST: f64 = 3.0;

/// Multiplier applied per scroll tick when zooming in (1.1×). Zoom-out uses its reciprocal.
const ZOOM_FACTOR: f64 = 1.1;
/// Minimum allowed zoom level (10 %).
const ZOOM_MIN: f64 = 0.1;
/// Maximum allowed zoom level (2000 %).
const ZOOM_MAX: f64 = 20.0;
/// `stroke-dasharray` used for in-progress draw previews and selection marquee.
pub(crate) const DASH_PREVIEW: &str = "4 2";

// ────────────────────────────────────────────────────────────────────────────

/// Dispatch a pointer-down event to the handler for the active canvas mode.
/// Extracted so pointer-down logic (including the `Text` editing path, which
/// commits the active edit before dispatching) shares a single dispatch.
fn route_pointer_down(
    detail: u32,
    client: (f64, f64),
    mode: CanvasMode,
    world: Point,
    st: &mut CanvasState,
    props: &mut CanvasInputs,
) {
    if props.eraser_active.get() || props.canvas_mode.get() == CanvasMode::Erase {
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
        CanvasMode::Pan => {
            st.pan_anchor.set(Some(client));
        }
        CanvasMode::Select => {
            select_pointer_down(detail, world, st, props);
        }
        CanvasMode::Draw => {
            draw_pointer_down(world, st, props);
        }
        CanvasMode::Erase => {}
    }
}

/// Root SVG canvas component.
///
/// Owns the full `<svg>` element and handles all pointer / keyboard interaction.
/// Every pointer event is **routed by mode** (`Select` / `Pan` / `Draw` / `Erase`) to a
/// dedicated handler in [`modes`] through a simple match-dispatch pattern:
///
/// ```ignore
/// on_pointer_down → match canvas_mode {
///     Select → modes::select_pointer_down(…)
///     Pan    → (inline) set pan_anchor
///     Draw   → modes::draw_pointer_down(…)
///     Erase  → hit_and_erase(…)
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
/// | `canvas_mode`   | Which interaction mode is active (Select / Pan / Draw / Erase) |
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
    cursor_world: RwSignal<Point>,
    viewport: RwSignal<Viewport>,
    selected_tool: RwSignal<Tool>,
    selected_color: RwSignal<Color>,
    canvas_mode: RwSignal<CanvasMode>,
    scene: RwSignal<Scene>,
    eraser_active: RwSignal<bool>,
    settings: RwSignal<CanvasSettings>,
    push_snapshot: Rc<dyn Fn()>,
    export_crop_active: RwSignal<bool>,
    on_crop_export: RwSignal<Option<CropExportCallback>>,
    selected_ids: RwSignal<Vec<ElementId>>,
    default_style: RwSignal<ElementStyle>,
    default_line_style: RwSignal<LineStyle>,
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
        default_style,
        default_line_style,
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

    // Esc: deselect / cancel draw / exit text edit
    // Priority: (a) textarea handles its own Esc → do nothing,
    //            (b) cancel in-progress draw, (c) clear selection.
    // Never changes canvas_mode or selected_tool.
    {
        let _st = st;
        let _props = props.clone();
        let _ = window_event_listener(ev::keydown, move |ev: ev::KeyboardEvent| {
            if ev.key() != "Escape" {
                return;
            }
            // (a) If a text element is being edited, the textarea's own
            // keydown handler will have already committed it. Do nothing.
            if _st.text_edit.editing_id.get().is_some() {
                return;
            }
            // (b) Cancel an in-progress draw. For Freehand the element was
            // added to the scene on pointer_down, so remove it.
            if _st.drawing.get().is_some() {
                let tool = _st.drawing.get().unwrap().tool;
                _st.drawing.set(None);
                if tool == Tool::Freehand {
                    _props.scene.update(|s| {
                        s.elements_mut().pop();
                    });
                }
                return;
            }
            // (c) Exit group-edit mode before clearing the selection.
            if let Some(group_id) = _st.group_editing.get_untracked() {
                let group_members = _props.scene.with_untracked(|scene| {
                    scene
                        .groups
                        .iter()
                        .find(|group| group.id == group_id)
                        .map(|group| group.members.clone())
                });
                _st.group_editing.set(None);
                if let Some(group_members) = group_members {
                    _st.selected_ids.set(group_members);
                }
                return;
            }
            // (d) Clear the current selection.
            if !_st.selected_ids.get_untracked().is_empty() {
                _st.selected_ids.set(Vec::new());
            }
        });
    }

    let commit_edit =
        text_edit::make_commit_edit(st.text_edit.editing_id, st.text_edit.edit_text, props.scene);

    // Whenever a text edit opens, focus the editing textarea so the caret is
    // visible for typing. The textarea is inserted reactively (in a later
    // flush) after `editing_id` is set, so the focus is scheduled for an
    // animation frame and retried until the node actually exists. Runs on
    // the first click that opens a text, and again when a click while
    // editing commits the old text and opens a fresh one.
    {
        let editing_id = st.text_edit.editing_id;
        let textarea_ref = st.text_edit.textarea_ref;
        create_effect(move |_| {
            if editing_id.get().is_some() {
                text_edit::focus_textarea(textarea_ref);
            }
        });
    }

    // Keep the editing textarea positioned and wrapped around the typed text,
    // and pan the canvas so the caret stays on screen. Runs on the next
    // animation frame (after the render has patched), so it cannot be
    // overwritten by a reactive re-render. Re-runs whenever the edit text,
    // the canvas pan, the screen size, or the scene changes while editing.
    {
        let editing_id = st.text_edit.editing_id;
        let edit_text = st.text_edit.edit_text;
        let textarea_ref = st.text_edit.textarea_ref;
        let scene = props.scene;
        let viewport = props.viewport;
        let screen_size = st.screen_size;
        create_effect(move |_| {
            if editing_id.get().is_some() {
                // Track everything the guide reads so typing/pans re-run it.
                let _ = edit_text.get();
                let _ = viewport.get();
                let _ = screen_size.get();
                scene.with(|_| ());
                let editing_id = editing_id;
                let textarea_ref = textarea_ref;
                let scene = scene;
                let viewport = viewport;
                let screen_size = screen_size;
                leptos::leptos_dom::helpers::request_animation_frame(move || {
                    text_edit::guide_textarea(
                        editing_id,
                        textarea_ref,
                        scene,
                        viewport,
                        screen_size,
                    );
                });
            }
        });
    }

    // Switching mode (Select/Hand/Draw) or active tool used to leave whatever
    // was in progress untouched: an active selection stayed highlighted after
    // switching to a draw tool, and a just-opened, still-empty text box stayed
    // on screen after switching away from the Text tool. Whenever the mode or
    // tool changes, clear the selection, and either commit or discard whatever
    // text edit was in progress — discard if nothing was typed (an empty Text
    // element has no reason to exist), commit if the user had actually typed
    // something (so switching tools mid-edit doesn't lose typed text).
    {
        let _st = st;
        let _props = props.clone();
        create_effect(move |prev: Option<(CanvasMode, Tool)>| {
            let mode = _props.canvas_mode.get();
            let tool = _props.selected_tool.get();
            if let Some(prev) = prev {
                if prev != (mode, tool) && _props.export_crop_active.get_untracked() {
                    _props.export_crop_active.set(false);
                    _props.on_crop_export.set(None);
                    _st.select_anchor.set(None);
                    _st.drag.last_world.set(None);
                }
            }
            (mode, tool)
        });

        let _st = st;
        let _props = props.clone();
        create_effect(move |prev: Option<(CanvasMode, Tool)>| {
            let mode = _props.canvas_mode.get();
            let tool = _props.selected_tool.get();
            if let Some(prev) = prev {
                if prev != (mode, tool) && !_st.selected_ids.get_untracked().is_empty() {
                    _st.selected_ids.set(Vec::new());
                    _st.group_editing.set(None);
                }
            }
            (mode, tool)
        });

        let _st = st;
        let _props = props.clone();
        let _commit_edit = commit_edit.clone();
        create_effect(move |prev: Option<(CanvasMode, Tool)>| {
            let mode = _props.canvas_mode.get();
            let tool = _props.selected_tool.get();
            if let Some(prev) = prev {
                if prev != (mode, tool) {
                    if let Some(id) = _st.text_edit.editing_id.get_untracked() {
                        if _st.text_edit.edit_text.get_untracked().is_empty() {
                            _props.scene.update(|s| s.remove_by_id(id));
                            _st.text_edit.editing_id.set(None);
                            _st.text_edit.edit_text.set(String::new());
                        } else {
                            _commit_edit();
                        }
                    }
                }
            }
            (mode, tool)
        });
    }

    // ── Cursor ───────────────────────────────────────────────────────────
    // Reactively compute the CSS cursor style based on the current mode,
    // tool, drag action, eraser toggle, and pan/crop state.
    {
        let st = st;
        let props = props.clone();
        create_effect(move |_| {
            let mode = props.canvas_mode.get();
            let tool = props.selected_tool.get();
            let eraser = props.eraser_active.get();
            let crop = props.export_crop_active.get();
            let pan = st.pan_anchor.get().is_some();
            let handle = st.drag.drag_action.get();
            let hovered = st.drag.hovered_handle.get();
            let cursor = cursor_style(mode, tool, handle, eraser, pan, crop, hovered);
            st.cursor_style.set(cursor);
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
    let update_world = move |ev: &ev::PointerEvent| -> Point {
        let screen = Point::new(ev.offset_x() as f64, ev.offset_y() as f64);
        props.cursor_screen.set((screen.x, screen.y));
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
    // 1. If a text edit is active (`editing_id` is set), it is committed
    //    first. The textarea's `blur` (fired by the mousedown default action)
    //    is scoped to its own edit in `text_edit_overlay`, so it cannot commit
    //    a freshly-created new text.
    // 2. Updates world coordinates via `update_world`.
    // 3. Dispatches by `CanvasMode` through `route_pointer_down`.
    //    - `Pan` → stores the initial client position for panning.
    //    - `Select` → `modes::select_pointer_down` – handles double-click text
    //      editing, handle grab (resize/move/rotate), element selection,
    //      and marquee anchor.
    //    - `Draw` → `modes::draw_pointer_down` – creates a new element
    //      (snapshot taken before creation) and enters the drawing state.
    //    - `Erase` → handled inside `route_pointer_down`, this match arm is unreachable.
    let on_pointer_down = {
        let mut st = st;
        let mut props = props.clone();
        let update_world = update_world;
        let commit_edit = commit_edit.clone();
        move |ev: ev::PointerEvent| {
            let editing = st.text_edit.editing_id.get().is_some();
            let mode = props.canvas_mode.get();
            let world = update_world(&ev);
            let detail = ev.detail() as u32;
            let client = (ev.client_x() as f64, ev.client_y() as f64);

            if editing {
                // A text edit is in progress: commit it, then handle this
                // click synchronously. The textarea's blur is scoped to its
                // own edit (see `text_edit_overlay`), so the blur fired by
                // the click's `mousedown` default action — by which point a
                // new edit may be open — is a no-op and cannot remove the
                // newly created empty text. The deferred-focus effect in
                // this file focuses the new textarea after the click has
                // fully dispatched, so the caret appears after the new
                // empty text is placed.
                commit_edit();
            }
            route_pointer_down(detail, client, mode, world, &mut st, &mut props);
        }
    };

    // ── Pointer move ─────────────────────────────────────────────────────
    // Handler for every pointer-move event on the SVG.
    //
    // Routing:
    // 1. Updates world coordinates.
    // 2. If the eraser is active (toggle or Erase mode) and the button is held,
    //    erases elements under the cursor (continuous erasing).
    // 3. Dispatches by mode:
    //    - `Pan` → pans the viewport by the delta between successive
    //      client-space positions.
    //    - `Select` → `modes::select_pointer_move` – handles resize drag
    //      (with per-frame deltas for scale), rotation drag (angle tracking
    //      with optional snap), and plain move (offset).
    //    - `Draw` → for `Freehand` tool, delegates to
    //      `UpdateDrag::update_drag` which appends the latest point to
    //      the stroke path. Other tools do nothing during move (the preview
    //      is handled reactively by the drawing preview closure).
    //    - `Erase` → the guard above catches this arm is unreachable.
    let on_pointer_move = {
        let mut st = st;
        let mut props = props.clone();
        let update_world = update_world;
        move |ev: ev::PointerEvent| {
            let mode = props.canvas_mode.get();
            let world = update_world(&ev);

            if (props.eraser_active.get() || props.canvas_mode.get() == CanvasMode::Erase)
                && st.erasing.get()
            {
                hit_and_erase(world, props.scene);
            }

            if props.export_crop_active.get() {
                st.drag.last_world.set(Some(world));
                return;
            }

            match mode {
                CanvasMode::Pan => {
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
                                    el.update_drag(
                                        world,
                                        Point::from(state.anchor),
                                        ev.shift_key(),
                                    );
                                }
                            });
                        }
                    }
                }
                CanvasMode::Erase => {}
            }
        }
    };

    // ── Pointer up ───────────────────────────────────────────────────────
    // Handler for every pointer-up event on the SVG.
    //
    // Routing:
    // 1. If the eraser is active (toggle or Erase mode), performs one final
    //    erase at the release point.
    // 2. Resets the `erasing` flag.
    // 3. Dispatches by mode:
    //    - `Pan` → clears the pan anchor (stops panning).
    //    - `Select` → `modes::select_pointer_up` – if a move/resize/rotate
    //      drag just finished, optionally snaps to grid (shift held), then
    //      clears all drag state. If a marquee drag just finished, selects
    //      all elements fully inside the rectangle.
    //    - `Draw` → `modes::draw_pointer_up` – finalises the element from
    //      the drag parameters (skipped for Freehand which is built incrementally,
    //      and skipped if the drag distance is below `MIN_DRAG_DIST`).
    //    - `Erase` → the guard above catches this; arm is unreachable.
    let on_pointer_up = {
        let mut st = st;
        let mut props = props.clone();
        let update_world = update_world;
        move |ev: ev::PointerEvent| {
            if props.eraser_active.get() || props.canvas_mode.get() == CanvasMode::Erase {
                let world = update_world(&ev);
                hit_and_erase(world, props.scene);
            }
            st.erasing.set(false);

            if props.export_crop_active.get() {
                if let Some(anchor) = st.select_anchor.get() {
                    let current = st.drag.last_world.get().unwrap_or(anchor);
                    let dx = current.x - anchor.x;
                    let dy = current.y - anchor.y;
                    if dx.hypot(dy) >= MIN_DRAG_DIST {
                        let x = anchor.x.min(current.x);
                        let y = anchor.y.min(current.y);
                        let w = (current.x - anchor.x).abs().max(1.0);
                        let h = (current.y - anchor.y).abs().max(1.0);
                        if let Some(cb) = props.on_crop_export.get() {
                            cb(Rect::new(x, y, w, h));
                            props.on_crop_export.set(None);
                        }
                    }
                }
                props.export_crop_active.set(false);
                st.select_anchor.set(None);
                st.drag.last_world.set(None);
                return;
            }

            let world = update_world(&ev);
            match props.canvas_mode.get() {
                CanvasMode::Pan => {
                    st.pan_anchor.set(None);
                }
                CanvasMode::Select => {
                    select_pointer_up(&ev, world, &mut st, &mut props);
                }
                CanvasMode::Draw => {
                    draw_pointer_up(&ev, world, &mut st, &mut props);
                }
                CanvasMode::Erase => {}
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
            let ss = st.screen_size.get();
            let factor = if ev.delta_y() < 0.0 {
                ZOOM_FACTOR
            } else {
                1.0 / ZOOM_FACTOR
            };
            props.viewport.update(|vp| {
                let world = vp.screen_to_world(Point::new(screen.0, screen.1), ss);
                vp.zoom = (vp.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
                vp.offset_x = world.x - (screen.0 - ss.x / 2.0) / vp.zoom;
                vp.offset_y = world.y - (screen.1 - ss.y / 2.0) / vp.zoom;
            });
        }
    };

    // Compute the SVG `viewBox` string from the current viewport and screen size.
    // Re-evaluated reactively whenever the viewport or window dimensions change.
    let view_box = {
        let st = st;
        let props = props.clone();
        move || {
            let ss = st.screen_size.get();
            props.viewport.get().to_view_box(ss.x, ss.y)
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
            let dx = world.x - state.anchor.0;
            let dy = world.y - state.anchor.1;
            if dx.hypot(dy) < MIN_DRAG_DIST {
                return None;
            }
            let shift = st.shift_pressed.get();
            let anchor = Point::from(state.anchor);
            let world_pt = world;
            let mut el: Element = match state.tool {
                Tool::Rectangle => {
                    Rectangle::from_drag(anchor, world_pt, state.color, shift).into()
                }
                Tool::Ellipse => Ellipse::from_drag(anchor, world_pt, state.color, shift).into(),
                Tool::Line => Line::from_drag(anchor, world_pt, state.color, shift).into(),
                Tool::Arrow => {
                    let line = Line::from_drag(anchor, world_pt, state.color, shift);
                    Element::Line(line)
                }
                Tool::Text => Text::from_drag(anchor, world_pt, state.color, shift).into(),
                Tool::Freehand => Freehand::from_drag(anchor, world_pt, state.color, shift).into(),
            };
            el.data_mut().style = props.default_style.get();
            if let Element::Line(ref mut l) = el {
                l.line_style = props.default_line_style.get();
            }
            Some(view! { <g stroke-dasharray=DASH_PREVIEW>{el.render(props.viewport.get().zoom)}</g> }.into_view())
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
            style:cursor=move || st.cursor_style.get()
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
            {
                let props = props.clone();
                let editing_id = st.text_edit.editing_id;
                move || {
                    let editing = editing_id.get();
                    props
                        .scene
                        .get()
                        .elements()
                        .iter()
                        .filter(|el| Some(el.id()) != editing)
                        .map(|el| {
                            // Elements render in world coordinates: the SVG
                            // `viewBox` applies the pan/zoom, so `render` ignores
                            // the zoom argument. Reading the viewport here would
                            // rebuild the whole scene on every pan (e.g. while the
                            // text follow-pan is typing), so stay untracked.
                            view! { <g pointer-events="none">{el.render(1.0)}</g> }
                                .into_view()
                        })
                        .collect_view()
                }
            }

            {drawing_preview}
            {smart_guides::guide_overlay(st.drag.smart_guides, props.viewport)}
            {selection::selection_preview_overlay(st.select_anchor, props.cursor_world)}
            {selection::selection_handle_overlay(
                st.selected_ids,
                props.scene,
                st.drag.overlay_freeze,
                st.drag.rotation_delta,
            )}
        </svg>

        {text_edit::text_edit_overlay(
            st.text_edit.editing_id,
            st.text_edit.edit_text,
            st.text_edit.textarea_ref,
            props.scene,
            props.viewport,
            st.screen_size,
            commit_edit,
        )}
    }
}
