use super::state::{
    combined_bounds, hit_test_topmost, point_inside_any_element, rect_fully_contains_element,
    CanvasInputs, CanvasState, Handle,
};
use super::{DASH_PREVIEW, MIN_DRAG_DIST};
use crate::model::elements::path::{handle_positions, segment_midpoint};
use crate::model::resize::{common_bounds, rotate_point_around};
use crate::model::{
    Bounds, Element, ElementId, Offset, PathPoints, Point, Resize, Rotate, Scene, ShapeColor,
    SnapToGrid,
};
use crate::model::resize::ResizeContext;

const GRID_SIZE: f64 = 40.0;
const HANDLE_RESIZE_RADIUS: f64 = 5.0;
const HANDLE_MOVE_RADIUS: f64 = 6.0;
const HANDLE_ROTATE_RADIUS: f64 = 7.0;
const ROTATE_HANDLE_OFFSET: f64 = 25.0;
const PATH_MERGE_DIST: f64 = 3.0;
const DASH_BOUNDS: &str = "3 2";
use crate::model::resize::ResizeHandle;
use leptos::{ev, SignalGet, SignalSet, SignalUpdate, *};

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
        if w < 1.0 || h < 1.0 {
            return None;
        }
        let hex = ShapeColor::Blue.to_hex();
        Some(
            view! {
                <rect x=x y=y width=w height=h fill=format!("{}33", hex) stroke=hex
                    stroke-width="1" stroke-dasharray={DASH_PREVIEW} pointer-events="none" />
            }
            .into_view(),
        )
    }
}

pub fn selection_handle_overlay(
    selected_ids: RwSignal<Vec<ElementId>>,
    scene: RwSignal<Scene>,
) -> impl Fn() -> Option<View> {
    move || {
        let ids = selected_ids.get();
        if ids.is_empty() {
            return None;
        }
        let els = scene.get().elements;
        let hex = ShapeColor::Blue.to_hex();

        if ids.len() == 1 {
            if let Some(el) = els.iter().find(|el| el.id() == ids[0]) {
                if let Some(points) = el.path_points() {
                    let n = points.len();
                    let (bx, by, bw, _bh) = el.bounds();
                    let mx = bx + bw / 2.0;
                    let my = by - 25.0;

                    let path_handles: Vec<_> = points.iter().map(|p| {
                        view! { <circle cx=p.x cy=p.y r="5" fill="white" stroke=hex stroke-width="1.5" pointer-events="none" /> }.into_view()
                    }).collect();

                    let cm = el.curve_mode();
                    let ghost_handles: Vec<_> = (0..n.saturating_sub(1)).filter_map(|i| {
                        let (gx, gy) = segment_midpoint(points, cm, i)?;
                        Some(view! { <circle cx=gx cy=gy r="3.5" fill="none" stroke=hex stroke-width="1" pointer-events="none" /> }.into_view())
                    }).collect();

                    let move_icon = view! {
                        <g stroke=hex stroke-width="1.5" fill="none"
                            transform=format!("translate({} {}) scale(0.9375)", mx - 11.25, my - 11.25)
                            pointer-events="none">
                            <path d="M12 3 L12 21 M3 12 L21 12" />
                            <path d="M9 6 L12 3 L15 6" />
                            <path d="M9 18 L12 21 L15 18" />
                            <path d="M6 9 L3 12 L6 15" />
                            <path d="M18 9 L21 12 L18 15" />
                        </g>
                    };

                    return Some(
                        view! {
                            {path_handles}
                            {ghost_handles}
                            {move_icon}
                        }
                        .into_view(),
                    );
                }
            }
        }

        // Use common_bounds (rotation-aware) for multi-select,
        // axis-aligned bounds for single-select (which rotates its overlay).
        let (bx, by, bw, bh) = if ids.len() == 1 {
            els.iter()
                .find(|el| el.id() == ids[0])
                .map(|el| el.bounds())
                .unwrap_or_else(|| combined_bounds(&ids, &els).unwrap_or((0.0, 0.0, 0.0, 0.0)))
        } else {
            // For multi-select, use rotation-aware common_bounds
            let data_refs: Vec<_> = els.iter()
                .filter(|el| ids.contains(&el.id()))
                .map(|el| el.data())
                .collect();
            if data_refs.is_empty() {
                (0.0, 0.0, 0.0, 0.0)
            } else {
                common_bounds(&data_refs)
            }
        };
        let hr = 5.0;
        let cx = bx + bw / 2.0;
        let cy = by + bh / 2.0;

        let rot: f64 = (ids.len() == 1)
            .then(|| {
                els.iter().find(|el| el.id() == ids[0]).and_then(|el| {
                    let r = el.data().rotation;
                    if r != 0.0 {
                        Some(r)
                    } else {
                        None
                    }
                })
            })
            .flatten()
            .unwrap_or(0.0);

        let rx = cx;
        let ry = by - ROTATE_HANDLE_OFFSET;

        let inner = {
            let corners = [
                (bx, by),
                (bx + bw / 2.0, by),
                (bx + bw, by),
                (bx, by + bh / 2.0),
                (bx + bw, by + bh / 2.0),
                (bx, by + bh),
                (bx + bw / 2.0, by + bh),
                (bx + bw, by + bh),
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

        let icons = render_move_rotate_icons(hex, cx, cy, rx, ry);

        let content = if rot != 0.0 {
            let deg = rot.to_degrees();
            view! { <g transform={format!("rotate({} {} {})", deg, cx, cy)}>{inner}{icons}</g> }
                .into_view()
        } else {
            view! { {inner}{icons} }.into_view()
        };

        Some(content)
    }
}

fn render_move_rotate_icons(hex: &'static str, cx: f64, cy: f64, rx: f64, ry: f64) -> leptos::View {
    let move_icon = view! {
        <g stroke=hex stroke-width="1.5" fill="none"
            transform=format!("translate({} {}) scale(0.9375)", cx - 11.25, cy - 11.25)
            pointer-events="none">
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
}

fn unrotate_for_element(point: (f64, f64), el: &Element) -> (f64, f64) {
    let rot = el.data().rotation;
    if rot == 0.0 {
        return point;
    }
    let (bx, by, bw, bh) = el.bounds();
    let cx = bx + bw / 2.0;
    let cy = by + bh / 2.0;
    rotate_point_around(point, (cx, cy), -rot)
}

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
                st.edit_text.set(text_elem.wrapped.raw.clone());
                return;
            }
        }
    }

    let ids = st.selected_ids.get();
    let els = props.scene.get().elements;

    if ids.len() == 1 {
        if let Some(el) = els.iter().find(|e| e.id() == ids[0]) {
            if let Some(points) = el.path_points() {
                for (i, p) in points.iter().enumerate() {
                    let d = ((world.0 - p.x).powi(2) + (world.1 - p.y).powi(2)).sqrt();
                    if d <= HANDLE_RESIZE_RADIUS {
                        (props.push_snapshot)();
                        st.drag_action.set(Some(Handle::PathPoint(i)));
                        st.moving_anchor.set(Some(world));
                        st.last_world.set(Some(world));
                        return;
                    }
                }
                let cm = el.curve_mode();
                for i in 0..points.len().saturating_sub(1) {
                    let Some((mx, my)) = segment_midpoint(points, cm, i) else {
                        continue;
                    };
                    let d = ((world.0 - mx).powi(2) + (world.1 - my).powi(2)).sqrt();
                    if d <= HANDLE_RESIZE_RADIUS {
                        st.drag_action.set(Some(Handle::PathMidpoint(i)));
                        st.moving_anchor.set(Some(world));
                        st.last_world.set(Some(world));
                        return;
                    }
                }
                let (bx, by, bw, _bh) = el.bounds();
                let mx = bx + bw / 2.0;
                let my = by - 25.0;
                if ((world.0 - mx).powi(2) + (world.1 - my).powi(2)).sqrt() <= HANDLE_MOVE_RADIUS {
                    (props.push_snapshot)();
                    st.drag_action.set(Some(Handle::Move));
                    st.moving_anchor.set(Some(world));
                    st.drag_bounds.set(Some(el.bounds()));
                    st.last_world.set(Some(world));
                    return;
                }
            }
        }
    }

    if !ids.is_empty() {
        let (test_x, test_y) = if ids.len() == 1 {
            els.iter()
                .find(|e| e.id() == ids[0])
                .map(|el| unrotate_for_element(world, el))
                .unwrap_or(world)
        } else {
            world
        };

        if let Some(bounds @ (bx, by, bw, bh)) = combined_bounds(&ids, &els) {
            let hpos = handle_positions(bx, by, bw, bh);
            for (i, &(hx, hy)) in hpos[..8].iter().enumerate() {
                if ((test_x - hx).powi(2) + (test_y - hy).powi(2)).sqrt() <= HANDLE_RESIZE_RADIUS {
                    (props.push_snapshot)();
                    st.drag_action.set(Some(Handle::Resize(ResizeHandle::from(i))));
                    st.moving_anchor.set(Some(world));
                    st.drag_bounds.set(Some(bounds));
                    st.last_world.set(Some(world));
                    st.drag_originals.set(
                        els.iter()
                            .filter(|el| ids.contains(&el.id()))
                            .cloned()
                            .collect(),
                    );
                    return;
                }
            }
            let (hx, hy) = hpos[8];
            if ((test_x - hx).powi(2) + (test_y - hy).powi(2)).sqrt() <= HANDLE_MOVE_RADIUS {
                (props.push_snapshot)();
                st.drag_action.set(Some(Handle::Move));
                st.moving_anchor.set(Some(world));
                st.drag_bounds.set(Some(bounds));
                st.last_world.set(Some(world));
                return;
            }
            let (hx, hy) = hpos[9];
            if ((test_x - hx).powi(2) + (test_y - hy).powi(2)).sqrt() <= HANDLE_ROTATE_RADIUS {
                (props.push_snapshot)();
                let cx = bx + bw / 2.0;
                let cy = by + bh / 2.0;
                st.drag_action.set(Some(Handle::Rotate));
                st.drag_angle.set(Some((world.1 - cy).atan2(world.0 - cx) + std::f64::consts::FRAC_PI_2));
                st.moving_anchor.set(Some(world));
                st.drag_bounds.set(Some(bounds));
                st.drag_originals.set(
                    els.iter()
                        .filter(|el| ids.contains(&el.id()))
                        .cloned()
                        .collect(),
                );
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
            if let Some(bounds) = combined_bounds(&[id], &els) {
                (props.push_snapshot)();
                st.drag_action.set(Some(Handle::Move));
                st.moving_anchor.set(Some(world));
                st.drag_bounds.set(Some(bounds));
                st.last_world.set(Some(world));
            }
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
            Some(Handle::Resize(handle)) => {
                if let Some((bx, by, bw, bh)) = st.drag_bounds.get() {
                    st.last_world.set(Some(world));
                    let multi = ids.len() > 1;
                    let originals = st.drag_originals.get();
                    let alt = st.alt_pressed.get();
                    let shift = st.shift_pressed.get();
                    props.scene.update(|s| {
                        for el in s.elements.iter_mut() {
                            if ids.contains(&el.id()) {
                                if let Some(orig) = originals.iter().find(|o| o.id() == el.id()) {
                                    let ctx = ResizeContext {
                                        orig,
                                        handle,
                                        pointer_world: world,
                                        shift,
                                        alt,
                                        multi,
                                        bx, by, bw, bh,
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
                    if let Some(initial_angle) = st.drag_angle.get() {
                        let current_angle = (world.1 - cy).atan2(world.0 - cx) + std::f64::consts::FRAC_PI_2;
                        let mut delta = current_angle - initial_angle;
                        if st.shift_pressed.get() {
                            let step = std::f64::consts::TAU / 24.0;
                            delta = (delta / step).round() * step;
                        }
                        // Snapshot-based: restore from originals and apply total delta
                        let originals = st.drag_originals.get();
                        props.scene.update(|s| {
                            for el in s.elements.iter_mut() {
                                if ids.contains(&el.id()) {
                                    if let Some(orig) = originals.iter().find(|o| o.id() == el.id()) {
                                        *el = orig.clone();
                                        el.rotate_around(Point { x: cx, y: cy }, delta);
                                    }
                                }
                            }
                        });
                    }
                }
            }
            Some(Handle::PathPoint(idx)) => {
                props.scene.update(|s| {
                    if let Some(el) = s.elements.iter_mut().find(|e| e.id() == ids[0]) {
                        if let Some(pts) = el.path_points_mut() {
                            if idx < pts.len() {
                                pts[idx].set(world.0, world.1);
                            }
                        }
                    }
                });
                st.moving_anchor.set(Some(world));
            }
            Some(Handle::PathMidpoint(i)) => {
                if let Some(click) = st.moving_anchor.get() {
                    let dist = (world.0 - click.0).hypot(world.1 - click.1);
                    if dist >= MIN_DRAG_DIST {
                        let new_idx = i + 1;
                        let (mx, my) = props.scene.with(|s| {
                            s.elements
                                .iter()
                                .find(|e| e.id() == ids[0])
                                .and_then(|el| {
                                    let pts = el.path_points()?;
                                    let cm = el.curve_mode();
                                    segment_midpoint(pts, cm, i)
                                })
                                .unwrap_or((world.0, world.1))
                        });
                        (props.push_snapshot)();
                        props.scene.update(|s| {
                            if let Some(target) = s.elements.iter_mut().find(|e| e.id() == ids[0]) {
                                if let Some(pts) = target.path_points_mut() {
                                    pts.insert(new_idx, Point { x: mx, y: my });
                                    pts[new_idx].set(world.0, world.1);
                                }
                            }
                        });
                        st.drag_action.set(Some(Handle::PathPoint(new_idx)));
                        st.moving_anchor.set(Some(world));
                        st.last_world.set(Some(world));
                    }
                }
            }
            _ => {
                props.scene.update(|s| {
                    for el in s.elements.iter_mut() {
                        if ids.contains(&el.id()) {
                            el.offset(dx, dy);
                        }
                    }
                });
                st.moving_anchor.set(Some(world));
            }
        }
    }
}

pub fn select_pointer_up(_ev: &ev::PointerEvent, st: &mut CanvasState, props: &mut CanvasInputs) {
    if st.moving_anchor.get().is_some() {
        let drag_action = st.drag_action.get();
        let ids = st.selected_ids.get();

        if let Some(Handle::PathPoint(idx)) = drag_action {
            let merged = props
                .scene
                .with(|s| {
                    let el = s.elements.iter().find(|e| e.id() == ids[0])?;
                    let pts = el.path_points()?;
                    if idx == 0 || idx + 1 >= pts.len() {
                        return Some(false);
                    }
                    let d = point_to_line_segment_dist(
                        pts[idx].x,
                        pts[idx].y,
                        pts[idx - 1].x,
                        pts[idx - 1].y,
                        pts[idx + 1].x,
                        pts[idx + 1].y,
                    );
                    Some(d < PATH_MERGE_DIST)
                })
                .unwrap_or(false);

            if merged {
                (props.push_snapshot)();
                props.scene.update(|s| {
                    if let Some(el) = s.elements.iter_mut().find(|e| e.id() == ids[0]) {
                        if let Some(pts) = el.path_points_mut() {
                            if idx < pts.len() {
                                pts.remove(idx);
                            }
                        }
                    }
                });
            }
        }

        if st.shift_pressed.get() {
            let drag_action = st.drag_action.get();
            let ids = st.selected_ids.get();
            if let Some(Handle::PathPoint(idx)) = drag_action {
                let exists = props.scene.with(|s| {
                    s.elements
                        .iter()
                        .find(|e| e.id() == ids[0])
                        .and_then(|el| el.path_points())
                        .is_some_and(|pts| idx < pts.len())
                });
                if exists {
                    props.scene.update(|s| {
                        if let Some(el) = s.elements.iter_mut().find(|e| e.id() == ids[0]) {
                            if let Some(pts) = el.path_points_mut() {
                                let sx = (pts[idx].x / GRID_SIZE).round() * GRID_SIZE;
                                let sy = (pts[idx].y / GRID_SIZE).round() * GRID_SIZE;
                                pts[idx].set(sx, sy);
                            }
                        }
                    });
                }
            } else {
                props.scene.update(|s| {
                    for el in s.elements.iter_mut() {
                        if ids.contains(&el.id()) {
                            el.snap_to_grid(GRID_SIZE);
                        }
                    }
                });
            }
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
            let contained: Vec<ElementId> = els
                .iter()
                .filter(|el| rect_fully_contains_element(rx, ry, rw, rh, el))
                .map(|el| el.id())
                .collect();
            st.selected_ids.set(contained);
        }
        st.select_anchor.set(None);
    }
}

fn point_to_line_segment_dist(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let abx = bx - ax;
    let aby = by - ay;
    let len2 = abx * abx + aby * aby;
    if len2 == 0.0 {
        return ((px - ax).powi(2) + (py - ay).powi(2)).sqrt();
    }
    let t = (((px - ax) * abx + (py - ay) * aby) / len2).clamp(0.0, 1.0);
    let cx = ax + t * abx;
    let cy = ay + t * aby;
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}
