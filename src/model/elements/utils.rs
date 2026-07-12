use crate::model::Point;

use super::rect::MIN_DIMENSION;

pub(crate) fn snap_bbox_to_grid(world_point: &mut Point, width: f64, height: f64, grid: f64) {
    let cx = world_point.x + width / 2.0;
    let cy = world_point.y + height / 2.0;
    let snapped_cx = (cx / grid).round() * grid;
    let snapped_cy = (cy / grid).round() * grid;
    world_point.x += snapped_cx - cx;
    world_point.y += snapped_cy - cy;
}

pub(crate) fn rect_from_drag(
    anchor: (f64, f64),
    current: (f64, f64),
    shift: bool,
) -> (Point, f64, f64) {
    let (ax, ay) = anchor;
    let (cx, cy) = current;
    let mut x = ax.min(cx);
    let mut y = ay.min(cy);
    let mut w = (cx - ax).abs();
    let mut h = (cy - ay).abs();
    if shift {
        let s = w.max(h);
        w = s;
        h = s;
        if cx < ax {
            x = ax - s;
        }
        if cy < ay {
            y = ay - s;
        }
    }
    if w < MIN_DIMENSION {
        w = MIN_DIMENSION;
    }
    if h < MIN_DIMENSION {
        h = MIN_DIMENSION;
    }
    (Point { x, y }, w, h)
}
