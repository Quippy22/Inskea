use crate::model::resize::ResizeContext;
use crate::model::Point;

use super::rect::MIN_DIMENSION;
use super::Element;
use super::ElementData;

pub(crate) fn snap_bbox_to_grid(world_point: &mut Point, width: f64, height: f64, grid: f64) {
    let cx = world_point.x + width / 2.0;
    let cy = world_point.y + height / 2.0;
    let snapped_cx = (cx / grid).round() * grid;
    let snapped_cy = (cy / grid).round() * grid;
    world_point.x += snapped_cx - cx;
    world_point.y += snapped_cy - cy;
}

pub(crate) fn rotate_bbox(data: &mut ElementData, pivot: Point, delta: f64) {
    data.rotation += delta;
    let cx = data.world_point.x + data.width / 2.0;
    let cy = data.world_point.y + data.height / 2.0;
    let mut center = Point { x: cx, y: cy };
    center.rotate_around(pivot, delta);
    data.world_point.x = center.x - data.width / 2.0;
    data.world_point.y = center.y - data.height / 2.0;
}

/// Compute the two endpoints of a line/arrow from drag anchor and current position.
/// When `shift` is held, the endpoint angle is snapped to the nearest 45° division.
pub(crate) fn line_endpoints(anchor: Point, current: Point, shift: bool) -> (Point, Point) {
    let ax = anchor.x;
    let ay = anchor.y;
    let cx = current.x;
    let cy = current.y;
    let (mut ex, mut ey) = (cx, cy);
    if shift {
        let dx = cx - ax;
        let dy = cy - ay;
        let angle = dy.atan2(dx);
        let divisions = 8.0;
        let snapped = crate::model::elements::snap_angle(angle, divisions);
        let dist = (dx * dx + dy * dy).sqrt();
        ex = ax + dist * snapped.cos();
        ey = ay + dist * snapped.sin();
    }
    (Point { x: ax, y: ay }, Point { x: ex, y: ey })
}

pub(crate) fn scale_points(points: &mut [Point], ctx: &ResizeContext) {
    let orig_slice: Vec<Point> = match ctx.orig {
        Element::Line(orig) => orig.points.clone(),
        Element::Freehand(orig) if ctx.multi => orig.points.clone(),
        _ => points.to_vec(),
    };
    super::path::scale_points(points, ctx, &orig_slice);
}

pub(crate) fn rect_from_drag(anchor: Point, current: Point, shift: bool) -> (Point, f64, f64) {
    let ax = anchor.x;
    let ay = anchor.y;
    let cx = current.x;
    let cy = current.y;
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
