use super::utils::{
    handle_positions, selection_bounds, unrotate_for_element, GRID_SIZE, HANDLE_MOVE_RADIUS,
    HANDLE_RESIZE_RADIUS, HANDLE_ROTATE_RADIUS, PATH_MERGE_DIST, SELECT_PADDING,
};
use super::MIN_DRAG_DIST;
use crate::canvas::state::{
    combined_bounds, hit_test_topmost, point_inside_any_element, rect_fully_contains_element,
    CanvasInputs, CanvasState, Handle,
};
use crate::model::elements::path::segment_midpoint;
use crate::model::elements::{Bounds, Offset, PathPoints, Resize, Rotate, SnapToGrid};
use crate::model::resize::ResizeContext;
use crate::model::resize::ResizeHandle;
use crate::model::{Element, ElementId, Point, Rect};
use leptos::{ev, SignalGet, SignalSet, SignalUpdate, *};

/// Enter text-edit mode on double-click of a Text element.
#[inline]
fn handle_text_double_click(
    ev: &ev::PointerEvent,
    world: (f64, f64),
    st: &mut CanvasState,
    props: &CanvasInputs,
) -> bool {
    if ev.detail() < 2 {
        return false;
    }
    let els = props.scene.get().elements().to_vec();
    let id = match hit_test_topmost(world, &els) {
        Some(id) => id,
        None => return false,
    };
    if let Some(Element::Text(text_elem)) = els.iter().find(|e| e.id() == id) {
        st.text_edit.editing_id.set(Some(id));
        st.text_edit.edit_text.set(text_elem.wrapped.raw.clone());
        return true;
    }
    false
}

/// Check if pointer is on a path-point dot or segment-midpoint ghost handle.
/// On hit, snapshot + set drag action to `PathPoint(i)` / `PathMidpoint(i)`.
#[inline]
fn handle_path_point_hit(
    world: (f64, f64),
    st: &mut CanvasState,
    props: &CanvasInputs,
    el: &Element,
) -> bool {
    let Some(points) = el.path_points() else {
        return false;
    };
    for (i, p) in points.iter().enumerate() {
        let d = ((world.0 - p.x).powi(2) + (world.1 - p.y).powi(2)).sqrt();
        if d <= HANDLE_RESIZE_RADIUS {
            (props.push_snapshot)();
            st.drag.drag_action.set(Some(Handle::PathPoint(i)));
            st.drag.moving_anchor.set(Some(world.into()));
            st.drag.last_world.set(Some(world.into()));
            return true;
        }
    }
    let cm = el.curve_mode();
    for i in 0..points.len().saturating_sub(1) {
        let Some((mx, my)) = segment_midpoint(points, cm, i) else {
            continue;
        };
        let d = ((world.0 - mx).powi(2) + (world.1 - my).powi(2)).sqrt();
        if d <= HANDLE_RESIZE_RADIUS {
            st.drag.drag_action.set(Some(Handle::PathMidpoint(i)));
            st.drag.moving_anchor.set(Some(world.into()));
            st.drag.last_world.set(Some(world.into()));
            return true;
        }
    }
    false
}

/// Check if pointer is on the move cross icon above a path-element.
#[inline]
fn handle_path_move_icon(
    world: (f64, f64),
    st: &mut CanvasState,
    props: &CanvasInputs,
    el: &Element,
) -> bool {
    if el.path_points().is_none() {
        return false;
    }
    let b = el.bounds();
    let mx = b.x + b.w / 2.0;
    let my = b.y - 25.0;
    if ((world.0 - mx).powi(2) + (world.1 - my).powi(2)).sqrt() <= HANDLE_MOVE_RADIUS {
        (props.push_snapshot)();
        st.drag.drag_action.set(Some(Handle::Move));
        st.drag.moving_anchor.set(Some(world.into()));
        st.drag.drag_bounds.set(Some(el.bounds()));
        st.drag.last_world.set(Some(world.into()));
        return true;
    }
    false
}

/// Hit-test the 8 resize corner/midpoint handles against the padded bounds.
#[inline]
fn hit_resize_handle(
    test_x: f64,
    test_y: f64,
    bounds: (f64, f64, f64, f64),
    st: &mut CanvasState,
    props: &CanvasInputs,
    world: (f64, f64),
    ids: &[ElementId],
    els: &[Element],
) -> bool {
    let (mut bx, mut by, mut bw, mut bh) = bounds;
    bx -= SELECT_PADDING;
    by -= SELECT_PADDING;
    bw += SELECT_PADDING * 2.0;
    bh += SELECT_PADDING * 2.0;
    let hpos = handle_positions(bx, by, bw, bh);

    for (i, &p) in hpos[..8].iter().enumerate() {
        if ((test_x - p.x).powi(2) + (test_y - p.y).powi(2)).sqrt() <= HANDLE_RESIZE_RADIUS {
            (props.push_snapshot)();
            st.drag
                .drag_action
                .set(Some(Handle::Resize(ResizeHandle::from(i))));
            st.drag.moving_anchor.set(Some(world.into()));
            st.drag.drag_bounds.set(Some(bounds.into()));
            st.drag.last_world.set(Some(world.into()));
            st.drag.drag_originals.set(
                els.iter()
                    .filter(|el| ids.contains(&el.id()))
                    .cloned()
                    .collect(),
            );
            return true;
        }
    }
    false
}

/// Hit-test the centre move handle.
#[inline]
fn hit_move_handle(
    test_x: f64,
    test_y: f64,
    bounds: (f64, f64, f64, f64),
    st: &mut CanvasState,
    props: &CanvasInputs,
    world: (f64, f64),
) -> bool {
    let (mut bx, mut by, mut bw, mut bh) = bounds;
    bx -= SELECT_PADDING;
    by -= SELECT_PADDING;
    bw += SELECT_PADDING * 2.0;
    bh += SELECT_PADDING * 2.0;
    let hpos = handle_positions(bx, by, bw, bh);
    let hp = hpos[8];

    if ((test_x - hp.x).powi(2) + (test_y - hp.y).powi(2)).sqrt() <= HANDLE_MOVE_RADIUS {
        (props.push_snapshot)();
        st.drag.drag_action.set(Some(Handle::Move));
        st.drag.moving_anchor.set(Some(world.into()));
        st.drag.drag_bounds.set(Some(bounds.into()));
        st.drag.last_world.set(Some(world.into()));
        return true;
    }
    false
}

/// Hit-test the rotate handle above the bbox and initialise rotation drag state.
#[inline]
fn hit_rotate_handle(
    test_x: f64,
    test_y: f64,
    bounds: (f64, f64, f64, f64),
    st: &mut CanvasState,
    props: &CanvasInputs,
    world: (f64, f64),
    ids: &[ElementId],
    els: &[Element],
) -> bool {
    let (mut bx, mut by, mut bw, mut bh) = bounds;
    bx -= SELECT_PADDING;
    by -= SELECT_PADDING;
    bw += SELECT_PADDING * 2.0;
    bh += SELECT_PADDING * 2.0;
    let hpos = handle_positions(bx, by, bw, bh);
    let hp = hpos[9];

    if ((test_x - hp.x).powi(2) + (test_y - hp.y).powi(2)).sqrt() <= HANDLE_ROTATE_RADIUS {
        (props.push_snapshot)();
        let cx = bx + bw / 2.0;
        let cy = by + bh / 2.0;
        st.drag.drag_action.set(Some(Handle::Rotate));
        st.drag.drag_angle.set(Some(
            (world.1 - cy).atan2(world.0 - cx) + std::f64::consts::FRAC_PI_2,
        ));
        st.drag.moving_anchor.set(Some(world.into()));
        st.drag.drag_bounds.set(Some(bounds.into()));
        st.drag.overlay_freeze.set(Some(bounds.into()));
        st.drag.drag_originals.set(
            els.iter()
                .filter(|el| ids.contains(&el.id()))
                .cloned()
                .collect(),
        );
        return true;
    }
    false
}

fn handle_selection_handles(
    world: (f64, f64),
    st: &mut CanvasState,
    props: &CanvasInputs,
    ids: &[ElementId],
    els: &[Element],
) -> bool {
    let test = if ids.len() == 1 {
        els.iter()
            .find(|e| e.id() == ids[0])
            .map(|el| unrotate_for_element(world.into(), el))
            .unwrap_or(world.into())
    } else {
        world.into()
    };
    let (test_x, test_y) = (test.x, test.y);

    let Some(b) = selection_bounds(ids, els) else {
        return false;
    };
    let bounds = (b.x, b.y, b.w, b.h);

    if hit_resize_handle(test_x, test_y, bounds, st, props, world, ids, els) {
        return true;
    }
    if hit_move_handle(test_x, test_y, bounds, st, props, world) {
        return true;
    }
    if hit_rotate_handle(test_x, test_y, bounds, st, props, world, ids, els) {
        return true;
    }

    false
}

/// Select or Shift-toggle the topmost element under the pointer.
/// Without Shift, also starts an immediate move drag on the newly selected element.
#[inline]
fn handle_element_selection(
    world: (f64, f64),
    st: &mut CanvasState,
    props: &CanvasInputs,
    els: &[Element],
) -> bool {
    let id = match hit_test_topmost(world, els) {
        Some(id) => id,
        None => return false,
    };
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
        if let Some(bounds) = combined_bounds(&[id], els) {
            (props.push_snapshot)();
            st.drag.drag_action.set(Some(Handle::Move));
            st.drag.moving_anchor.set(Some(world.into()));
            st.drag.drag_bounds.set(Some(bounds));
            st.drag.last_world.set(Some(world.into()));
        }
    }
    true
}

/// Figure out which handle (if any) the cursor is hovering over.
/// Used to set the `hovered_handle` signal for cursor style feedback.
#[inline]
fn detect_hovered_handle(
    world: (f64, f64),
    st: &CanvasState,
    props: &CanvasInputs,
) -> Option<Handle> {
    let ids = st.selected_ids.get();
    if ids.is_empty() {
        return None;
    }
    let els = props.scene.get().elements().to_vec();

    if ids.len() == 1 {
        if let Some(el) = els.iter().find(|e| e.id() == ids[0]) {
            if let Some(points) = el.path_points() {
                for (i, p) in points.iter().enumerate() {
                    if ((world.0 - p.x).powi(2) + (world.1 - p.y).powi(2)).sqrt()
                        <= HANDLE_RESIZE_RADIUS
                    {
                        return Some(Handle::PathPoint(i));
                    }
                }
                let b = el.bounds();
                let mx = b.x + b.w / 2.0;
                let my = b.y - 25.0;
                if ((world.0 - mx).powi(2) + (world.1 - my).powi(2)).sqrt() <= HANDLE_MOVE_RADIUS {
                    return Some(Handle::Move);
                }
            }
        }
    }

    if let Some(mut b) = selection_bounds(&ids, &els) {
        b = b.padded(SELECT_PADDING);
        let hpos = handle_positions(b.x, b.y, b.w, b.h);

        for (i, &p) in hpos[..8].iter().enumerate() {
            if ((world.0 - p.x).powi(2) + (world.1 - p.y).powi(2)).sqrt() <= HANDLE_RESIZE_RADIUS {
                return Some(Handle::Resize(ResizeHandle::from(i)));
            }
        }

        let p = hpos[8];
        if ((world.0 - p.x).powi(2) + (world.1 - p.y).powi(2)).sqrt() <= HANDLE_MOVE_RADIUS {
            return Some(Handle::Move);
        }

        let p = hpos[9];
        if ((world.0 - p.x).powi(2) + (world.1 - p.y).powi(2)).sqrt() <= HANDLE_ROTATE_RADIUS {
            return Some(Handle::Rotate);
        }
    }

    None
}

/// Apply a resize drag to all selected elements, restoring from originals each frame.
#[inline]
fn do_resize(world: (f64, f64), st: &mut CanvasState, props: &mut CanvasInputs) {
    if let Some(b) = st.drag.drag_bounds.get() {
        st.drag.last_world.set(Some(world.into()));
        let ids = st.selected_ids.get();
        let multi = ids.len() > 1;
        let originals = st.drag.drag_originals.get();
        let alt = st.alt_pressed.get();
        let shift = st.shift_pressed.get();
        let handle = match st.drag.drag_action.get() {
            Some(Handle::Resize(h)) => h,
            _ => return,
        };
        props.scene.update(|s| {
            for el in s.elements_mut().iter_mut() {
                if ids.contains(&el.id()) {
                    if let Some(orig) = originals.iter().find(|o| o.id() == el.id()) {
                        let ctx = ResizeContext {
                            orig,
                            handle,
                            pointer_world: world,
                            shift,
                            alt,
                            multi,
                            bx: b.x,
                            by: b.y,
                            bw: b.w,
                            bh: b.h,
                        };
                        el.resize(&ctx);
                    }
                }
            }
        });
    }
}

/// Apply a rotation drag: compute normalised delta from initial angle,
/// snap if Shift held, restore originals and re-apply total delta.
#[inline]
fn do_rotate(world: (f64, f64), st: &mut CanvasState, props: &mut CanvasInputs) {
    if let Some(b) = st.drag.drag_bounds.get() {
        let cx = b.x + b.w / 2.0;
        let cy = b.y + b.h / 2.0;
        if let Some(initial_angle) = st.drag.drag_angle.get() {
            let current_angle = (world.1 - cy).atan2(world.0 - cx) + std::f64::consts::FRAC_PI_2;
            let mut delta = current_angle - initial_angle;
            if delta > std::f64::consts::PI {
                delta -= std::f64::consts::TAU;
            } else if delta < -std::f64::consts::PI {
                delta += std::f64::consts::TAU;
            }
            if st.shift_pressed.get() {
                let step = std::f64::consts::TAU / 24.0;
                delta = (delta / step).round() * step;
            }
            let ids = st.selected_ids.get();
            let originals = st.drag.drag_originals.get();
            props.scene.update(|s| {
                for el in s.elements_mut().iter_mut() {
                    if ids.contains(&el.id()) {
                        if let Some(orig) = originals.iter().find(|o| o.id() == el.id()) {
                            *el = orig.clone();
                            el.rotate_around(Point { x: cx, y: cy }, delta);
                        }
                    }
                }
            });
            st.drag.rotation_delta.set(delta);
        }
    }
}

/// Drag a single path point to the current world position.
#[inline]
fn do_path_point_drag(
    world: (f64, f64),
    st: &mut CanvasState,
    props: &mut CanvasInputs,
    idx: usize,
) {
    let ids = st.selected_ids.get();
    props.scene.update(|s| {
        if let Some(el) = s.element_by_id_mut(ids[0]) {
            if let Some(pts) = el.path_points_mut() {
                if idx < pts.len() {
                    pts[idx].set(world.0, world.1);
                }
            }
        }
    });
    st.drag.moving_anchor.set(Some(world.into()));
}

/// Drag a segment midpoint beyond `MIN_DRAG_DIST` to insert a new path point.
#[inline]
fn do_path_midpoint_drag(
    world: (f64, f64),
    st: &mut CanvasState,
    props: &mut CanvasInputs,
    i: usize,
) {
    if let Some(click) = st.drag.moving_anchor.get() {
        let dist = (world.0 - click.x).hypot(world.1 - click.y);
        if dist >= MIN_DRAG_DIST {
            let ids = st.selected_ids.get();
            let new_idx = i + 1;
            let (mx, my) = props.scene.with(|s| {
                s.element_by_id(ids[0])
                    .and_then(|el| {
                        let pts = el.path_points()?;
                        let cm = el.curve_mode();
                        segment_midpoint(pts, cm, i)
                    })
                    .unwrap_or((world.0, world.1))
            });
            (props.push_snapshot)();
            props.scene.update(|s| {
                if let Some(target) = s.element_by_id_mut(ids[0]) {
                    if let Some(pts) = target.path_points_mut() {
                        pts.insert(new_idx, Point { x: mx, y: my });
                        pts[new_idx].set(world.0, world.1);
                    }
                }
            });
            st.drag.drag_action.set(Some(Handle::PathPoint(new_idx)));
            st.drag.moving_anchor.set(Some(world.into()));
            st.drag.last_world.set(Some(world.into()));
        }
    }
}

/// Offset all selected elements by the delta since the drag anchor.
#[inline]
fn do_move(world: (f64, f64), anchor: (f64, f64), st: &mut CanvasState, props: &mut CanvasInputs) {
    let dx = world.0 - anchor.0;
    let dy = world.1 - anchor.1;
    let ids = st.selected_ids.get();
    props.scene.update(|s| {
        for el in s.elements_mut().iter_mut() {
            if ids.contains(&el.id()) {
                el.offset(dx, dy);
            }
        }
    });
    st.drag.moving_anchor.set(Some(world.into()));
}

/// On pointer-up, collapse a path point if it sits too close to its chord.
#[inline]
fn finish_path_point_drag(st: &mut CanvasState, props: &mut CanvasInputs, idx: usize) {
    let ids = st.selected_ids.get();
    let merged = props
        .scene
        .with(|s| {
            let el = s.element_by_id(ids[0])?;
            let pts = el.path_points()?;
            if idx == 0 || idx + 1 >= pts.len() {
                return Some(false);
            }
            let d = Point::dist_to_segment(pts[idx], pts[idx - 1], pts[idx + 1]);
            Some(d < PATH_MERGE_DIST)
        })
        .unwrap_or(false);

    if merged {
        (props.push_snapshot)();
        props.scene.update(|s| {
            if let Some(el) = s.element_by_id_mut(ids[0]) {
                if let Some(pts) = el.path_points_mut() {
                    if idx < pts.len() {
                        pts.remove(idx);
                    }
                }
            }
        });
    }
}

/// Snap a single path point to the grid (Shift+up after point drag).
#[inline]
fn snap_path_point_to_grid(st: &mut CanvasState, props: &mut CanvasInputs, idx: usize) {
    let ids = st.selected_ids.get();
    let exists = props.scene.with(|s| {
        s.element_by_id(ids[0])
            .and_then(|el| el.path_points())
            .is_some_and(|pts| idx < pts.len())
    });
    if exists {
        props.scene.update(|s| {
            if let Some(el) = s.element_by_id_mut(ids[0]) {
                if let Some(pts) = el.path_points_mut() {
                    let sx = (pts[idx].x / GRID_SIZE).round() * GRID_SIZE;
                    let sy = (pts[idx].y / GRID_SIZE).round() * GRID_SIZE;
                    pts[idx].set(sx, sy);
                }
            }
        });
    }
}

/// Snap all selected elements to the grid (Shift+up after move/resize/rotate).
#[inline]
fn snap_elements_to_grid(st: &mut CanvasState, props: &mut CanvasInputs) {
    let ids = st.selected_ids.get();
    props.scene.update(|s| {
        for el in s.elements_mut().iter_mut() {
            if ids.contains(&el.id()) {
                el.snap_to_grid(GRID_SIZE);
            }
        }
    });
}

/// Reset all drag-tracking signals to their default (no-op) values.
#[inline]
fn clear_drag_state(st: &mut CanvasState) {
    st.drag.moving_anchor.set(None);
    st.drag.drag_action.set(None);
    st.drag.drag_bounds.set(None);
    st.drag.drag_angle.set(None);
    st.drag.overlay_freeze.set(None);
    st.drag.rotation_delta.set(0.0);
    st.drag.last_world.set(None);
    st.drag.drag_originals.set(Vec::new());
    st.drag.hovered_handle.set(None);
    st.select_anchor.set(None);
}

/// Finish a marquee drag-select: select all elements fully inside the dragged rect.
#[inline]
fn complete_marquee_selection(world: (f64, f64), st: &mut CanvasState, props: &mut CanvasInputs) {
    if let Some(anchor) = st.select_anchor.get() {
        let dx = world.0 - anchor.0;
        let dy = world.1 - anchor.1;
        if dx.hypot(dy) >= MIN_DRAG_DIST {
            let rx = anchor.0.min(world.0);
            let ry = anchor.1.min(world.1);
            let rw = (world.0 - anchor.0).abs();
            let rh = (world.1 - anchor.1).abs();
            let els = props.scene.get().elements().to_vec();
            let contained: Vec<ElementId> = els
                .iter()
                .filter(|el| rect_fully_contains_element(Rect::new(rx, ry, rw, rh), el))
                .map(|el| el.id())
                .collect();
            st.selected_ids.set(contained);
        }
        st.select_anchor.set(None);
    }
}

pub fn select_pointer_down(
    _ev: &ev::PointerEvent,
    world: (f64, f64),
    st: &mut CanvasState,
    props: &mut CanvasInputs,
) {
    st.drag.overlay_freeze.set(None);
    st.drag.rotation_delta.set(0.0);

    if handle_text_double_click(_ev, world, st, props) {
        return;
    }

    let ids = st.selected_ids.get();
    let els = props.scene.get().elements().to_vec();

    if ids.len() == 1 {
        if let Some(el) = els.iter().find(|e| e.id() == ids[0]) {
            if handle_path_point_hit(world, st, props, el)
                || handle_path_move_icon(world, st, props, el)
            {
                return;
            }
        }
    }

    if !ids.is_empty() && handle_selection_handles(world, st, props, &ids, &els) {
        return;
    }

    if handle_element_selection(world, st, props, &els) {
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
    if st.drag.moving_anchor.get().is_none() {
        let hovered = detect_hovered_handle(world, st, props);
        st.drag.hovered_handle.set(hovered);
    }

    if let Some(anchor) = st.drag.moving_anchor.get() {
        match st.drag.drag_action.get() {
            Some(Handle::Resize(_)) => do_resize(world, st, props),
            Some(Handle::Rotate) => do_rotate(world, st, props),
            Some(Handle::PathPoint(idx)) => do_path_point_drag(world, st, props, idx),
            Some(Handle::PathMidpoint(i)) => do_path_midpoint_drag(world, st, props, i),
            _ => do_move(world, (anchor.x, anchor.y), st, props),
        }
    }
}

pub fn select_pointer_up(
    _ev: &ev::PointerEvent,
    world: (f64, f64),
    st: &mut CanvasState,
    props: &mut CanvasInputs,
) {
    if st.drag.moving_anchor.get().is_some() {
        let drag_action = st.drag.drag_action.get();

        if let Some(Handle::PathPoint(idx)) = drag_action {
            finish_path_point_drag(st, props, idx);
        }

        if st.shift_pressed.get() {
            if let Some(Handle::PathPoint(idx)) = drag_action {
                snap_path_point_to_grid(st, props, idx);
            } else {
                snap_elements_to_grid(st, props);
            }
        }

        clear_drag_state(st);
        return;
    }

    complete_marquee_selection(world, st, props);
}
