use crate::model::elements::line::handle_positions;
use crate::model::{
    Bounds, Element, ElementId, Scene, ShapeColor, Offset, Resize, Rotate, SnapToGrid,
};
use super::state::{
    CanvasInputs, CanvasState, Handle, combined_bounds, hit_test_topmost,
    point_inside_any_element, rect_fully_contains_element,
};
use super::{DASH_PREVIEW, MIN_DRAG_DIST};

const GRID_SIZE: f64 = 40.0;
const HANDLE_RESIZE_RADIUS: f64 = 5.0;
const HANDLE_MOVE_RADIUS: f64 = 6.0;
const HANDLE_ROTATE_RADIUS: f64 = 7.0;
const ROTATE_HANDLE_OFFSET: f64 = 25.0;
const DASH_BOUNDS: &str = "3 2";
const ROTATE_SNAP_DIVISIONS: f64 = 24.0;
use crate::model::elements::{snap_angle, ResizeContext};
use leptos::{ev, SignalGet, SignalSet, SignalUpdate, *};

/// Reactive marquee rectangle shown during a box-select drag.
///
/// Drawn from the `select_anchor` (pointer-down world position) to the
/// current `cursor_world`. Only visible when `select_anchor` is set and
/// the drag is large enough (w ≥ 1 and h ≥ 1).
pub fn selection_preview_overlay(
    select_anchor: RwSignal<Option<(f64, f64)>>,
    cursor_world: RwSignal<(f64, f64)>,
) -> impl Fn() -> Option<View> {
    move || {
        let anchor = select_anchor.get()?;
        let world = cursor_world.get();
        let x = anchor.0.min(world.0);
        let y = anchor.1.min(world.1);
        let w = (world.0 - anchor.0).abs();
        let h = (world.1 - anchor.1).abs();
        if w < 1.0 || h < 1.0 { return None; }
        let hex = ShapeColor::Blue.to_hex();
        Some(view! {
            <rect x=x y=y width=w height=h fill=format!("{}33", hex) stroke=hex
                stroke-width="1" stroke-dasharray={DASH_PREVIEW} pointer-events="none" />
        }.into_view())
    }
}

/// Reactive overlay showing the selection bounding box, resize handles,
/// move handle (centre), and rotate handle (top).
///
/// Handle layout (10 positions returned by `handle_positions`):
/// - Indices 0–7: corners and edge mid-points (8 resize handles).
/// - Index 8: centre point (move handle).
/// - Index 9: above the box (rotate handle, offset by `ROTATE_HANDLE_OFFSET`).
///
/// If the selected element or group has a non-zero rotation, the entire
/// overlay (box + handles + icons) is wrapped in an SVG `rotate` transform.
/// The move and rotate icons are always positioned at the centre and at
/// the rotated handle position respectively.
pub fn selection_handle_overlay(
    selected_ids: RwSignal<Vec<ElementId>>,
    scene: RwSignal<Scene>,
) -> impl Fn() -> Option<View> {
    move || {
        let ids = selected_ids.get();
        if ids.is_empty() { return None; }
        let els = scene.get().elements;
        let (bx, by, bw, bh) = if ids.len() == 1 {
            els.iter().find(|el| el.id() == ids[0])
                .map(|el| el.bounds())
                .unwrap_or_else(|| combined_bounds(&ids, &els).unwrap_or((0.0, 0.0, 0.0, 0.0)))
        } else {
            combined_bounds(&ids, &els).unwrap_or((0.0, 0.0, 0.0, 0.0))
        };
        let hex = ShapeColor::Blue.to_hex();
        let hr = 5.0;
        let cx = bx + bw / 2.0;
        let cy = by + bh / 2.0;

        let rot: f64 = (ids.len() == 1)
            .then(|| {
                els.iter().find(|el| el.id() == ids[0])
                    .and_then(|el| { let r = el.data().rotation; if r != 0.0 { Some(r) } else { None } })
            }).flatten().unwrap_or(0.0);

        let handle_vec_x = 0.0;
        let handle_vec_y = -(bh / 2.0 + ROTATE_HANDLE_OFFSET);
        let rx = cx + handle_vec_x * rot.cos() - handle_vec_y * rot.sin();
        let ry = cy + handle_vec_x * rot.sin() + handle_vec_y * rot.cos();

        let inner = {
            let corners = [
                (bx, by), (bx + bw / 2.0, by), (bx + bw, by),
                (bx, by + bh / 2.0), (bx + bw, by + bh / 2.0),
                (bx, by + bh), (bx + bw / 2.0, by + bh), (bx + bw, by + bh),
            ];
            view! {
                <rect x=bx y=by width=bw height=bh fill="none" stroke=hex
                    stroke-width="1" stroke-dasharray={DASH_BOUNDS} pointer-events="none" />
                <line x1=cx y1=by x2=rx y2=ry stroke=hex stroke-width="1" pointer-events="none" />
                {corners.iter().map(|&(hx, hy)| {
                    view! { <circle cx=hx cy=hy r=hr fill="white" stroke=hex stroke-width="1.5" pointer-events="none" /> }.into_view()
                }).collect_view()}
            }.into_view()
        };

        let icons = {
            let move_icon = view! {
                <g stroke=hex stroke-width="1.5" fill="none"
                    transform=format!("translate({} {}) scale(0.75)", cx - 9.0, cy - 9.0)
                    pointer-events="none">
                    <circle cx="12" cy="12" r="9.25" fill="white" stroke=hex stroke-width="1.5" />
                    <path d="M12 3 L12 21 M3 12 L21 12" />
                    <path d="M9 6 L12 3 L15 6" />
                    <path d="M9 18 L12 21 L15 18" />
                    <path d="M6 9 L3 12 L6 15" />
                    <path d="M18 9 L21 12 L18 15" />
                </g>
            };
            let rotate_icon = view! {
                <g stroke=hex stroke-width="1.5" fill="none"
                    transform=format!("translate({} {}) scale(0.75)", rx - 9.0, ry - 9.0)
                    pointer-events="none">
                    <circle cx="12" cy="12" r="9.25" fill="white" stroke=hex stroke-width="1.5" />
                    <path d="M12 4 A8 8 0 1 1 4 12" />
                    <path d="M1.8 11.6 L4 9 L6.2 11.6" />
                </g>
            };
            view! { {move_icon} {rotate_icon} }.into_view()
        };

        let content = if rot != 0.0 {
            let deg = rot.to_degrees();
            view! { <g transform={format!("rotate({} {} {})", deg, cx, cy)}>{inner}{icons}</g> }.into_view()
        } else {
            view! { {inner}{icons} }.into_view()
        };

        Some(content)
    }
}

/// Handle a pointer-down event while in `Select` mode.
///
/// Priority order: double-click text → handle grab → element click →
/// click-in-bounds → empty-space (marquee anchor).
pub fn select_pointer_down(
    _ev: &ev::PointerEvent,
    world: (f64, f64),
    st: &mut CanvasState,
    props: &mut CanvasInputs,
) {
    if _ev.detail() >= 2 {
        let els = props.scene.get().elements;
        if let Some(id) = hit_test_topmost(world, &els) {
            if let Some(Element::Text(text_elem)) = els.iter().find(|e| e.id() == id) {
                st.editing_id.set(Some(id));
                st.edit_text.set(text_elem.content.clone());
                return;
            }
        }
    }

    let ids = st.selected_ids.get();
    let els = props.scene.get().elements;

    if !ids.is_empty() {
        if let Some(bounds @ (bx, by, bw, bh)) = combined_bounds(&ids, &els) {
            let hpos = handle_positions(bx, by, bw, bh);
            for (i, &(hx, hy)) in hpos[..8].iter().enumerate() {
                if ((world.0 - hx).powi(2) + (world.1 - hy).powi(2)).sqrt() <= HANDLE_RESIZE_RADIUS {
                    (props.push_snapshot)();
                    st.drag_action.set(Some(Handle::Resize(i)));
                    st.moving_anchor.set(Some(world));
                    st.drag_bounds.set(Some(bounds));
                    st.last_world.set(Some(world));
                    st.drag_originals.set(
                        els.iter().filter(|el| ids.contains(&el.id())).cloned().collect(),
                    );
                    return;
                }
            }
            let (hx, hy) = hpos[8];
            if ((world.0 - hx).powi(2) + (world.1 - hy).powi(2)).sqrt() <= HANDLE_MOVE_RADIUS {
                (props.push_snapshot)();
                st.drag_action.set(Some(Handle::Move));
                st.moving_anchor.set(Some(world));
                st.drag_bounds.set(Some(bounds));
                st.last_world.set(Some(world));
                return;
            }
            let (hx, hy) = hpos[9];
            if ((world.0 - hx).powi(2) + (world.1 - hy).powi(2)).sqrt() <= HANDLE_ROTATE_RADIUS {
                (props.push_snapshot)();
                let cx = bx + bw / 2.0;
                let cy = by + bh / 2.0;
                st.drag_action.set(Some(Handle::Rotate));
                st.drag_angle.set(Some((world.1 - cy).atan2(world.0 - cx)));
                st.moving_anchor.set(Some(world));
                st.drag_bounds.set(Some(bounds));
                return;
            }
        }
    }

    if let Some(id) = hit_test_topmost(world, &els) {
        let mut ids = st.selected_ids.get();
        if st.shift_pressed.get() {
            if let Some(pos) = ids.iter().position(|&x| x == id) {
                ids.remove(pos);
            } else {
                ids.push(id);
            }
            st.selected_ids.set(ids);
        } else {
            st.selected_ids.set(vec![id]);
        }
        return;
    }

    if point_inside_any_element(world, &els) {
        if !st.shift_pressed.get() {
            st.selected_ids.set(Vec::new());
        }
        return;
    }

    st.selected_ids.set(Vec::new());
    st.select_anchor.set(Some(world));
}

/// Handle a pointer-move event while in `Select` mode.
///
/// Only acts when `moving_anchor` is set. Dispatches to resize, rotate,
/// or move based on `drag_action`.
pub fn select_pointer_move(
    world: (f64, f64),
    _ev: &ev::PointerEvent,
    st: &mut CanvasState,
    props: &mut CanvasInputs,
) {
    if let Some(anchor) = st.moving_anchor.get() {
        let dx = world.0 - anchor.0;
        let dy = world.1 - anchor.1;
        let ids = st.selected_ids.get();
        match st.drag_action.get() {
            Some(Handle::Resize(idx)) => {
                if let Some((bx, by, bw, bh)) = st.drag_bounds.get() {
                    let frame_dx = world.0 - st.last_world.get().unwrap_or(world).0;
                    let frame_dy = world.1 - st.last_world.get().unwrap_or(world).1;
                    st.last_world.set(Some(world));
                    let multi = ids.len() > 1;
                    let originals = st.drag_originals.get();
                    props.scene.update(|s| {
                        for el in s.elements.iter_mut() {
                            if ids.contains(&el.id()) {
                                if let Some(orig) = originals.iter().find(|o| o.id() == el.id()) {
                                    let ctx = ResizeContext {
                                        orig,
                                        bx, by, bw, bh,
                                        dx, dy,
                                        fdx: frame_dx, fdy: frame_dy,
                                        handle: idx,
                                        shift: st.shift_pressed.get(),
                                        multi,
                                    };
                                    el.resize(&ctx);
                                }
                            }
                        }
                    });
                }
            }
            Some(Handle::Rotate) => {
                if let Some((bx, by, bw, bh)) = st.drag_bounds.get() {
                    let cx = bx + bw / 2.0;
                    let cy = by + bh / 2.0;
                    if let Some(prev) = st.drag_angle.get() {
                        let mut cur = (world.1 - cy).atan2(world.0 - cx);
                        if st.shift_pressed.get() {
                            cur = snap_angle(cur, ROTATE_SNAP_DIVISIONS);
                        }
                        let delta = cur - prev;
                        st.drag_angle.set(Some(cur));
                        props.scene.update(|s| {
                            for el in s.elements.iter_mut() {
                                if ids.contains(&el.id()) { el.rotate_around(cx, cy, delta); }
                            }
                        });
                    }
                }
            }
            _ => {
                props.scene.update(|s| {
                    for el in s.elements.iter_mut() {
                        if ids.contains(&el.id()) { el.offset(dx, dy); }
                    }
                });
                st.moving_anchor.set(Some(world));
            }
        }
    }
}

/// Handle a pointer-up event while in `Select` mode.
///
/// Finishes a drag (optionally snaps to grid) or finalises a marquee select.
pub fn select_pointer_up(
    _ev: &ev::PointerEvent,
    st: &mut CanvasState,
    props: &mut CanvasInputs,
) {
    if st.moving_anchor.get().is_some() {
        if st.shift_pressed.get() {
            let ids = st.selected_ids.get();
            props.scene.update(|s| {
                for el in s.elements.iter_mut() {
                    if ids.contains(&el.id()) { el.snap_to_grid(GRID_SIZE); }
                }
            });
        }
        st.moving_anchor.set(None);
        st.drag_action.set(None);
        st.drag_bounds.set(None);
        st.drag_angle.set(None);
        st.last_world.set(None);
        st.drag_originals.set(Vec::new());
        st.select_anchor.set(None);
        return;
    }

    if let Some(anchor) = st.select_anchor.get() {
        let world = props.cursor_world.get();
        let dx = world.0 - anchor.0;
        let dy = world.1 - anchor.1;
        if dx.hypot(dy) >= MIN_DRAG_DIST {
            let rx = anchor.0.min(world.0);
            let ry = anchor.1.min(world.1);
            let rw = (world.0 - anchor.0).abs();
            let rh = (world.1 - anchor.1).abs();
            let els = props.scene.get().elements;
            let contained: Vec<ElementId> = els.iter()
                .filter(|el| rect_fully_contains_element(rx, ry, rw, rh, el))
                .map(|el| el.id())
                .collect();
            st.selected_ids.set(contained);
        }
        st.select_anchor.set(None);
    }
}
