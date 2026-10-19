use super::utils::{
    handle_positions, selection_bounds, unrotate_for_element, GRID_SIZE, HANDLE_MOVE_RADIUS,
    HANDLE_RESIZE_RADIUS, HANDLE_ROTATE_RADIUS, PATH_MERGE_DIST, SELECT_PADDING,
};
use super::MIN_DRAG_DIST;
use crate::canvas::smart_guides::{self, GuideBox, SmartGuideState};
use crate::canvas::state::{CanvasInputs, CanvasState, DragOperation};
use crate::model::elements::path::segment_midpoint;
use crate::model::elements::{Bounds, Offset, PathPoints, Resize, Rotate, SnapToGrid};
use crate::model::interaction::{
    guide_bounds, guide_frame, guide_union_bounds, guide_union_frame, hit_test_topmost,
    point_inside_any_element, rect_fully_contains_element, GuideFrame, CLICK_MARGIN, HIT_MARGIN,
};
use crate::model::resize::ResizeContext;
use crate::model::resize::ResizeHandle;
use crate::model::selection::{expand_ids, normalize_ids, toggle_ids};
use crate::model::{Element, ElementId, Point, Rect};
use leptos::{ev, SignalGet, SignalSet, SignalUpdate, *};

/// Enter text-edit mode on double-click of a Text element.
#[inline]
fn handle_text_double_click(
    detail: u32,
    world: Point,
    st: &mut CanvasState,
    props: &CanvasInputs,
) -> bool {
    if detail < 2 {
        return false;
    }
    let els = props.scene.get().elements().to_vec();
    let id = match hit_test_topmost(world, &els, HIT_MARGIN) {
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
    world: Point,
    st: &mut CanvasState,
    props: &CanvasInputs,
    el: &Element,
) -> bool {
    let Some(points) = el.path_points() else {
        return false;
    };
    for (i, p) in points.iter().enumerate() {
        let d = ((world.x - p.x).powi(2) + (world.y - p.y).powi(2)).sqrt();
        if d <= HANDLE_RESIZE_RADIUS {
            (props.push_snapshot)();
            st.drag.drag_originals.set(vec![el.clone()]);
            st.drag.drag_action.set(Some(DragOperation::PathPoint(i)));
            st.drag.moving_anchor.set(Some(world));
            st.drag.last_world.set(Some(world));
            return true;
        }
    }
    let cm = el.curve_mode();
    for i in 0..points.len().saturating_sub(1) {
        let Some((mx, my)) = segment_midpoint(points, cm, i) else {
            continue;
        };
        let d = ((world.x - mx).powi(2) + (world.y - my).powi(2)).sqrt();
        if d <= HANDLE_RESIZE_RADIUS {
            st.drag.drag_originals.set(vec![el.clone()]);
            st.drag
                .drag_action
                .set(Some(DragOperation::PathMidpoint(i)));
            st.drag.moving_anchor.set(Some(world));
            st.drag.last_world.set(Some(world));
            return true;
        }
    }
    false
}

/// Check if pointer is on the move cross icon above a path-element.
#[inline]
fn handle_path_move_icon(
    world: Point,
    st: &mut CanvasState,
    props: &CanvasInputs,
    els: &[Element],
    el: &Element,
) -> bool {
    if el.path_points().is_none() {
        return false;
    }
    let b = el.bounds();
    let mx = b.x + b.w / 2.0;
    let my = b.y - 25.0;
    if ((world.x - mx).powi(2) + (world.y - my).powi(2)).sqrt() <= HANDLE_MOVE_RADIUS {
        (props.push_snapshot)();
        st.drag.drag_action.set(Some(DragOperation::Move));
        st.drag.moving_anchor.set(Some(world));
        st.drag.drag_bounds.set(Some(guide_bounds(el)));
        st.drag.smart_guide_frame.set(Some(guide_frame(el)));
        st.drag.last_world.set(Some(world));
        st.drag.drag_originals.set(vec![el.clone()]);
        capture_smart_guide_targets(st, &[el.id()], els);
        return true;
    }
    false
}

/// Hit-test the 8 resize corner/midpoint handles against the padded bounds.
#[inline]
fn hit_resize_handle(
    test: Point,
    bounds: Rect,
    st: &mut CanvasState,
    props: &CanvasInputs,
    world: Point,
    ids: &[ElementId],
    els: &[Element],
) -> bool {
    let (mut bx, mut by, mut bw, mut bh) = (bounds.x, bounds.y, bounds.w, bounds.h);
    bx -= SELECT_PADDING;
    by -= SELECT_PADDING;
    bw += SELECT_PADDING * 2.0;
    bh += SELECT_PADDING * 2.0;
    let hpos = handle_positions(bx, by, bw, bh);

    for (i, &p) in hpos[..8].iter().enumerate() {
        if ((test.x - p.x).powi(2) + (test.y - p.y).powi(2)).sqrt() <= HANDLE_RESIZE_RADIUS {
            (props.push_snapshot)();
            st.drag
                .drag_action
                .set(Some(DragOperation::Resize(ResizeHandle::from(i))));
            st.drag.moving_anchor.set(Some(world));
            st.drag.drag_bounds.set(Some(bounds));
            st.drag.last_world.set(Some(world));
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
    test: Point,
    bounds: Rect,
    st: &mut CanvasState,
    props: &CanvasInputs,
    world: Point,
    ids: &[ElementId],
    els: &[Element],
) -> bool {
    let (mut bx, mut by, mut bw, mut bh) = (bounds.x, bounds.y, bounds.w, bounds.h);
    bx -= SELECT_PADDING;
    by -= SELECT_PADDING;
    bw += SELECT_PADDING * 2.0;
    bh += SELECT_PADDING * 2.0;
    let hpos = handle_positions(bx, by, bw, bh);
    let hp = hpos[8];

    if ((test.x - hp.x).powi(2) + (test.y - hp.y).powi(2)).sqrt() <= HANDLE_MOVE_RADIUS {
        let Some(drag_bounds) = guide_union_bounds(ids, els) else {
            return false;
        };
        (props.push_snapshot)();
        st.drag.drag_action.set(Some(DragOperation::Move));
        st.drag.moving_anchor.set(Some(world));
        st.drag.drag_bounds.set(Some(drag_bounds));
        st.drag.smart_guide_frame.set(guide_union_frame(ids, els));
        st.drag.last_world.set(Some(world));
        st.drag.drag_originals.set(
            els.iter()
                .filter(|el| ids.contains(&el.id()))
                .cloned()
                .collect(),
        );
        capture_smart_guide_targets(st, ids, els);
        return true;
    }
    false
}

/// Hit-test the rotate handle above the bbox and initialise rotation drag state.
#[inline]
fn hit_rotate_handle(
    test: Point,
    bounds: Rect,
    st: &mut CanvasState,
    props: &CanvasInputs,
    world: Point,
    ids: &[ElementId],
    els: &[Element],
) -> bool {
    let (mut bx, mut by, mut bw, mut bh) = (bounds.x, bounds.y, bounds.w, bounds.h);
    bx -= SELECT_PADDING;
    by -= SELECT_PADDING;
    bw += SELECT_PADDING * 2.0;
    bh += SELECT_PADDING * 2.0;
    let hpos = handle_positions(bx, by, bw, bh);
    let hp = hpos[9];

    if ((test.x - hp.x).powi(2) + (test.y - hp.y).powi(2)).sqrt() <= HANDLE_ROTATE_RADIUS {
        (props.push_snapshot)();
        let cx = bx + bw / 2.0;
        let cy = by + bh / 2.0;
        st.drag.drag_action.set(Some(DragOperation::Rotate));
        st.drag.drag_angle.set(Some(
            (world.y - cy).atan2(world.x - cx) + std::f64::consts::FRAC_PI_2,
        ));
        st.drag.moving_anchor.set(Some(world));
        st.drag.drag_bounds.set(Some(bounds));
        st.drag.overlay_freeze.set(Some(bounds));
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
    world: Point,
    st: &mut CanvasState,
    props: &CanvasInputs,
    ids: &[ElementId],
    els: &[Element],
) -> bool {
    let test = if ids.len() == 1 {
        els.iter()
            .find(|e| e.id() == ids[0])
            .map(|el| unrotate_for_element(world, el))
            .unwrap_or(world)
    } else {
        world
    };

    let Some(b) = selection_bounds(ids, els) else {
        return false;
    };

    if hit_resize_handle(test, b, st, props, world, ids, els) {
        return true;
    }
    if hit_move_handle(test, b, st, props, world, ids, els) {
        return true;
    }
    if hit_rotate_handle(test, b, st, props, world, ids, els) {
        return true;
    }

    false
}

/// Enter group-edit mode on double-click, selecting only the clicked member.
#[inline]
fn handle_group_member_double_click(
    detail: u32,
    world: Point,
    st: &mut CanvasState,
    props: &CanvasInputs,
) -> bool {
    if detail < 2 {
        return false;
    }
    let els = props.scene.get().elements().to_vec();
    let id = match hit_test_topmost(world, &els, HIT_MARGIN) {
        Some(id) => id,
        None => return false,
    };
    if let Some(group_id) = props
        .scene
        .with(|s| s.group_containing(id).map(|group| group.id))
    {
        st.group_editing.set(Some(group_id));
        st.selected_ids.set(vec![id]);
        return true;
    }
    false
}

/// Select or Shift-toggle the topmost element under the pointer.
/// Without Shift, also starts an immediate move drag on the newly selected element.
#[inline]
fn handle_element_selection(
    world: Point,
    st: &mut CanvasState,
    props: &CanvasInputs,
    els: &[Element],
) -> bool {
    let id = match hit_test_topmost(world, els, HIT_MARGIN) {
        Some(id) => id,
        None => return false,
    };

    let editing_group = st.group_editing.get();
    if let Some(group_id) = editing_group {
        let is_member = props.scene.with(|s| {
            s.group_containing(id)
                .is_some_and(|group| group.id == group_id)
        });
        if !is_member {
            st.group_editing.set(None);
        }
    }

    let shift = st.shift_pressed.get();
    if st.group_editing.get().is_some() {
        select_and_begin_move(world, st, props, els, &[id], shift);
        return true;
    }

    let expanded = props.scene.with(|s| expand_ids(s, &[id]));
    select_and_begin_move(world, st, props, els, &expanded, shift);
    true
}

/// Apply the selection change for a press on `target` and start the move drag.
/// Shift toggles selection as usual, except on an already selected element:
/// there the toggle is deferred to pointer-up so a Shift-drag moves the
/// selection (with snapping) instead of deselecting the grabbed element.
fn select_and_begin_move(
    world: Point,
    st: &mut CanvasState,
    props: &CanvasInputs,
    els: &[Element],
    target: &[ElementId],
    shift: bool,
) {
    let ids = normalize_ids(&props.scene.get_untracked(), &st.selected_ids.get());
    let moving = if shift && target.iter().all(|id| ids.contains(id)) {
        st.drag.pending_shift_toggle.set(Some(target.to_vec()));
        ids
    } else {
        let next = if shift {
            toggle_ids(&ids, target)
        } else {
            target.to_vec()
        };
        st.selected_ids.set(next.clone());
        next
    };
    begin_move_drag(world, st, props, els, &moving);
}

/// Capture the elements the smart guides may snap against when a move starts.
#[inline]
fn capture_smart_guide_targets(st: &mut CanvasState, ids: &[ElementId], els: &[Element]) {
    st.drag.smart_guide_targets.set(
        els.iter()
            .filter(|el| !ids.contains(&el.id()))
            .map(|el| GuideBox {
                id: el.id(),
                bounds: guide_bounds(el),
                frame: guide_frame(el),
            })
            .collect(),
    );
}

/// Capture the drag originals, bounds, and operation for an element move.
fn begin_move_drag(
    world: Point,
    st: &mut CanvasState,
    props: &CanvasInputs,
    els: &[Element],
    ids: &[ElementId],
) {
    let Some(bounds) = guide_union_bounds(ids, els) else {
        return;
    };
    (props.push_snapshot)();
    st.drag.drag_action.set(Some(DragOperation::Move));
    st.drag.moving_anchor.set(Some(world));
    st.drag.drag_bounds.set(Some(bounds));
    st.drag.smart_guide_frame.set(guide_union_frame(ids, els));
    st.drag.last_world.set(Some(world));
    st.drag.drag_originals.set(
        els.iter()
            .filter(|el| ids.contains(&el.id()))
            .cloned()
            .collect(),
    );
    capture_smart_guide_targets(st, ids, els);
}

/// Figure out which handle (if any) the cursor is hovering over.
/// Used to set the `hovered_handle` signal for cursor style feedback.
#[inline]
fn detect_hovered_handle(
    world: Point,
    st: &CanvasState,
    props: &CanvasInputs,
) -> Option<DragOperation> {
    let ids = st.selected_ids.get();
    if ids.is_empty() {
        return None;
    }
    // `with` instead of `get`: hover runs on every pointer move and must
    // not deep-clone the whole scene.
    props.scene.with(|s| {
        let els = s.elements();

        if ids.len() == 1 {
            if let Some(el) = els.iter().find(|e| e.id() == ids[0]) {
                if let Some(points) = el.path_points() {
                    for (i, p) in points.iter().enumerate() {
                        if ((world.x - p.x).powi(2) + (world.y - p.y).powi(2)).sqrt()
                            <= HANDLE_RESIZE_RADIUS
                        {
                            return Some(DragOperation::PathPoint(i));
                        }
                    }
                    let b = el.bounds();
                    let mx = b.x + b.w / 2.0;
                    let my = b.y - 25.0;
                    if ((world.x - mx).powi(2) + (world.y - my).powi(2)).sqrt()
                        <= HANDLE_MOVE_RADIUS
                    {
                        return Some(DragOperation::Move);
                    }
                }
            }
        }

        if let Some(mut b) = selection_bounds(&ids, els) {
            b = b.padded(SELECT_PADDING);
            let hpos = handle_positions(b.x, b.y, b.w, b.h);

            for (i, &p) in hpos[..8].iter().enumerate() {
                if ((world.x - p.x).powi(2) + (world.y - p.y).powi(2)).sqrt()
                    <= HANDLE_RESIZE_RADIUS
                {
                    return Some(DragOperation::Resize(ResizeHandle::from(i)));
                }
            }

            let p = hpos[8];
            if ((world.x - p.x).powi(2) + (world.y - p.y).powi(2)).sqrt() <= HANDLE_MOVE_RADIUS {
                return Some(DragOperation::Move);
            }

            let p = hpos[9];
            if ((world.x - p.x).powi(2) + (world.y - p.y).powi(2)).sqrt() <= HANDLE_ROTATE_RADIUS {
                return Some(DragOperation::Rotate);
            }
        }

        None
    })
}

/// Apply a resize drag to all selected elements, restoring from originals each frame.
#[inline]
fn do_resize(world: Point, st: &mut CanvasState, props: &mut CanvasInputs) {
    if let Some(b) = st.drag.drag_bounds.get() {
        let Some(start_world) = st.drag.moving_anchor.get() else {
            return;
        };
        st.drag.last_world.set(Some(world));
        let ids = st.selected_ids.get();
        let multi = ids.len() > 1;
        let originals = st.drag.drag_originals.get();
        let alt = st.alt_pressed.get();
        let shift = st.shift_pressed.get();
        let handle = match st.drag.drag_action.get() {
            Some(DragOperation::Resize(h)) => h,
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
                            start_world,
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
fn do_rotate(world: Point, st: &mut CanvasState, props: &mut CanvasInputs) {
    if let Some(b) = st.drag.drag_bounds.get() {
        let cx = b.x + b.w / 2.0;
        let cy = b.y + b.h / 2.0;
        if let Some(initial_angle) = st.drag.drag_angle.get() {
            let current_angle = (world.y - cy).atan2(world.x - cx) + std::f64::consts::FRAC_PI_2;
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
fn do_path_point_drag(world: Point, st: &mut CanvasState, props: &mut CanvasInputs, idx: usize) {
    let ids = st.selected_ids.get();
    props.scene.update(|s| {
        if let Some(el) = s.element_by_id_mut(ids[0]) {
            if let Some(pts) = el.path_points_mut() {
                if idx < pts.len() {
                    pts[idx] = world;
                }
            }
        }
    });
    st.drag.moving_anchor.set(Some(world));
}

/// Drag a segment midpoint beyond `MIN_DRAG_DIST` to insert a new path point.
#[inline]
fn do_path_midpoint_drag(world: Point, st: &mut CanvasState, props: &mut CanvasInputs, i: usize) {
    if let Some(click) = st.drag.moving_anchor.get() {
        let dist = (world.x - click.x).hypot(world.y - click.y);
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
                    .unwrap_or((world.x, world.y))
            });
            (props.push_snapshot)();
            props.scene.update(|s| {
                if let Some(target) = s.element_by_id_mut(ids[0]) {
                    if let Some(pts) = target.path_points_mut() {
                        pts.insert(new_idx, Point { x: mx, y: my });
                        pts[new_idx] = world;
                    }
                }
            });
            st.drag
                .drag_action
                .set(Some(DragOperation::PathPoint(new_idx)));
            st.drag.moving_anchor.set(Some(world));
            st.drag.last_world.set(Some(world));
        }
    }
}

/// Offset all selected elements by the delta since the drag anchor.
#[inline]
fn do_move(world: Point, anchor: Point, st: &mut CanvasState, props: &mut CanvasInputs) {
    let ids = st.selected_ids.get();
    let originals = st.drag.drag_originals.get();
    let Some(bounds) = st.drag.drag_bounds.get() else {
        return;
    };
    let raw_dx = world.x - anchor.x;
    let raw_dy = world.y - anchor.y;
    let smart_enabled =
        st.shift_pressed.get() && matches!(st.drag.drag_action.get(), Some(DragOperation::Move));
    let (snap_dx, snap_dy, next_guides) = if smart_enabled {
        let viewport = props.viewport.get_untracked();
        let visible = viewport.visible_rect(st.screen_size.get_untracked());
        st.drag.smart_guides.with(|previous| {
            st.drag.smart_guide_frame.with(|frame| {
                st.drag.smart_guide_targets.with(|targets| {
                    let proposed =
                        Rect::new(bounds.x + raw_dx, bounds.y + raw_dy, bounds.w, bounds.h);
                    let frame = frame
                        .unwrap_or_else(|| GuideFrame::rect(bounds))
                        .translated(raw_dx, raw_dy);
                    let resolution = smart_guides::resolve_move(
                        proposed,
                        frame,
                        targets,
                        visible,
                        viewport.zoom,
                        previous,
                    );
                    (resolution.snap_dx, resolution.snap_dy, resolution.state)
                })
            })
        })
    } else {
        (0.0, 0.0, SmartGuideState::default())
    };
    let dx = raw_dx + snap_dx;
    let dy = raw_dy + snap_dy;
    props.scene.update(|s| {
        for el in s.elements_mut().iter_mut() {
            if ids.contains(&el.id()) {
                if let Some(orig) = originals.iter().find(|o| o.id() == el.id()) {
                    *el = orig.clone();
                    el.offset(dx, dy);
                }
            }
        }
    });
    st.drag.smart_guides.set(next_guides);
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

/// Snap a single path point to the grid (Alt-release after a point drag).
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

/// Snap all selected elements to the grid (Alt-release after move/resize/rotate).
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
pub(crate) fn clear_drag_state(st: &mut CanvasState) {
    st.drag.moving_anchor.set(None);
    st.drag.drag_action.set(None);
    st.drag.drag_bounds.set(None);
    st.drag.drag_angle.set(None);
    st.drag.overlay_freeze.set(None);
    st.drag.rotation_delta.set(0.0);
    st.drag.last_world.set(None);
    st.drag.drag_originals.set(Vec::new());
    st.drag.hovered_handle.set(None);
    st.drag.smart_guides.set(SmartGuideState::default());
    st.drag.smart_guide_targets.set(Vec::new());
    st.drag.smart_guide_frame.set(None);
    st.drag.pending_shift_toggle.set(None);
    st.select_anchor.set(None);
}

/// Abort an in-progress selection drag (move / resize / rotate / path point)
/// or marquee: restore the captured originals, drop the drag's undo snapshot,
/// and clear all drag state so pointer-up cannot re-apply anything.
pub(crate) fn cancel_drag(st: &mut CanvasState, props: &mut CanvasInputs) {
    let action = st.drag.drag_action.get_untracked();
    let originals = st.drag.drag_originals.get_untracked();
    if !originals.is_empty() {
        props.scene.update(|s| {
            for orig in &originals {
                if let Some(el) = s.element_by_id_mut(orig.id()) {
                    *el = orig.clone();
                }
            }
        });
    }
    // A still-pending PathMidpoint has not pushed a snapshot yet; every other
    // action pushed exactly one when the drag began.
    if !matches!(action, None | Some(DragOperation::PathMidpoint(_))) {
        (props.discard_snapshot)();
    }
    clear_drag_state(st);
}

/// Finish a marquee drag-select: select all elements fully inside the dragged rect.
#[inline]
fn complete_marquee_selection(world: Point, st: &mut CanvasState, props: &mut CanvasInputs) {
    if let Some(anchor) = st.select_anchor.get() {
        let dx = world.x - anchor.x;
        let dy = world.y - anchor.y;
        if dx.hypot(dy) >= MIN_DRAG_DIST {
            let rx = anchor.x.min(world.x);
            let ry = anchor.y.min(world.y);
            let rw = (world.x - anchor.x).abs();
            let rh = (world.y - anchor.y).abs();
            let els = props.scene.get().elements().to_vec();
            let contained: Vec<ElementId> = els
                .iter()
                .filter(|el| rect_fully_contains_element(Rect::new(rx, ry, rw, rh), el))
                .map(|el| el.id())
                .collect();
            let expanded = props.scene.with(|s| expand_ids(s, &contained));
            st.selected_ids
                .set(normalize_ids(&props.scene.get_untracked(), &expanded));
        }
        st.select_anchor.set(None);
    }
}

pub fn select_pointer_down(
    detail: u32,
    world: Point,
    st: &mut CanvasState,
    props: &mut CanvasInputs,
) {
    st.drag.overlay_freeze.set(None);
    st.drag.rotation_delta.set(0.0);
    st.drag.smart_guides.set(SmartGuideState::default());

    if handle_text_double_click(detail, world, st, props) {
        return;
    }

    if handle_group_member_double_click(detail, world, st, props) {
        return;
    }

    let ids = st.selected_ids.get();
    let els = props.scene.get().elements().to_vec();

    if ids.len() == 1 {
        if let Some(el) = els.iter().find(|e| e.id() == ids[0]) {
            if handle_path_point_hit(world, st, props, el)
                || handle_path_move_icon(world, st, props, &els, el)
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

    if point_inside_any_element(world, &els, CLICK_MARGIN) {
        st.group_editing.set(None);
        if !st.shift_pressed.get() {
            st.selected_ids.set(Vec::new());
        }
        return;
    }

    st.group_editing.set(None);
    st.selected_ids.set(Vec::new());
    st.select_anchor.set(Some(world));
}

pub fn select_pointer_move(
    world: Point,
    _ev: &ev::PointerEvent,
    st: &mut CanvasState,
    props: &mut CanvasInputs,
) {
    if st.drag.moving_anchor.get().is_none() {
        let hovered = detect_hovered_handle(world, st, props);
        // Only notify when it actually changed: setting the signal on every
        // move re-runs the cursor effect and downstream work for nothing.
        if st.drag.hovered_handle.with(|h| *h != hovered) {
            st.drag.hovered_handle.set(hovered);
        }
    }

    if let Some(anchor) = st.drag.moving_anchor.get() {
        match st.drag.drag_action.get() {
            Some(DragOperation::Resize(_)) => do_resize(world, st, props),
            Some(DragOperation::Rotate) => do_rotate(world, st, props),
            Some(DragOperation::PathPoint(idx)) => do_path_point_drag(world, st, props, idx),
            Some(DragOperation::PathMidpoint(i)) => do_path_midpoint_drag(world, st, props, i),
            _ => do_move(world, anchor, st, props),
        }
    }
}

pub fn select_pointer_up(
    _ev: &ev::PointerEvent,
    world: Point,
    st: &mut CanvasState,
    props: &mut CanvasInputs,
) {
    if st.drag.moving_anchor.get().is_some() {
        let drag_action = st.drag.drag_action.get();

        if let Some(DragOperation::PathPoint(idx)) = drag_action {
            finish_path_point_drag(st, props, idx);
        }

        let travelled =
            st.drag.moving_anchor.get().is_some_and(|anchor| {
                (world.x - anchor.x).hypot(world.y - anchor.y) >= MIN_DRAG_DIST
            });
        if matches!(drag_action, Some(DragOperation::Move)) && !travelled {
            finish_shift_click(st, props);
        }

        // Alt is the grid-snap key; Shift stays reserved for smart guides.
        let smart_guide_active = !st.drag.smart_guides.get().active.is_empty();
        if st.alt_pressed.get() && travelled {
            if let Some(DragOperation::PathPoint(idx)) = drag_action {
                snap_path_point_to_grid(st, props, idx);
            } else if !smart_guide_active {
                snap_elements_to_grid(st, props);
            }
        }

        clear_drag_state(st);
        return;
    }

    complete_marquee_selection(world, st, props);
}

/// Finish a press that never became a drag. If the press was a Shift grab of an
/// already selected element, undo the sub-threshold offset and toggle it out of
/// the selection: Shift-click keeps toggling, only Shift-drags move.
fn finish_shift_click(st: &mut CanvasState, props: &mut CanvasInputs) {
    let Some(target) = st.drag.pending_shift_toggle.get() else {
        return;
    };
    let originals = st.drag.drag_originals.get();
    props.scene.update(|s| {
        for el in s.elements_mut().iter_mut() {
            if let Some(orig) = originals.iter().find(|o| o.id() == el.id()) {
                *el = orig.clone();
            }
        }
    });
    if st.shift_pressed.get() {
        let ids = normalize_ids(&props.scene.get_untracked(), &st.selected_ids.get());
        st.selected_ids.set(toggle_ids(&ids, &target));
    }
}
