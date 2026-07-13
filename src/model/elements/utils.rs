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
        Element::Freehand(orig) => orig.points.clone(),
        _ => points.to_vec(),
    };
    super::path::scale_points(points, ctx, &orig_slice);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_bbox_to_grid_moves_center() {
        let mut pt = Point::new(3.0, 7.0);
        let w = 10.0;
        let h = 10.0; // center at (8, 12)
        snap_bbox_to_grid(&mut pt, w, h, 10.0);
        // snapped center: (10, 10), so world_point shifts by (+2, -2)
        assert!((pt.x - 5.0).abs() < 1e-10);
        assert!((pt.y - 5.0).abs() < 1e-10);
    }

    #[test]
    fn rotate_bbox_accumulates_rotation() {
        let mut data = ElementData::new(0);
        data.world_point = Point::new(0.0, 0.0);
        data.width = 100.0;
        data.height = 50.0;
        let pivot = Point::new(50.0, 25.0); // center of element
        rotate_bbox(&mut data, pivot, std::f64::consts::FRAC_PI_2);
        assert!((data.rotation - std::f64::consts::FRAC_PI_2).abs() < 1e-10);
        // Center rotates around itself → no movement of world_point
        assert!((data.world_point.x - 0.0).abs() < 1e-10);
        assert!((data.world_point.y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn line_endpoints_no_shift() {
        let (a, b) = line_endpoints(Point::new(0.0, 0.0), Point::new(10.0, 20.0), false);
        assert_eq!(a, Point::new(0.0, 0.0));
        assert_eq!(b, Point::new(10.0, 20.0));
    }

    #[test]
    fn line_endpoints_shift_snaps_45_degrees() {
        let (a, b) = line_endpoints(Point::new(0.0, 0.0), Point::new(10.0, 5.0), true);
        assert_eq!(a, Point::new(0.0, 0.0));
        // With 8 divisions, snap angle steps are 45°. (10,5) has angle ~26.6°,
        // so it snaps to 45°. The distance is sqrt(125) ≈ 11.18, so endpoint
        // should be at roughly (11.18*cos45, 11.18*sin45) ≈ (7.9, 7.9)
        assert!((b.x - 7.905694150420948).abs() < 1e-10);
        assert!((b.y - 7.905694150420948).abs() < 1e-10);
    }

    #[test]
    fn rect_from_drag_no_shift() {
        let (pt, w, h) = rect_from_drag(Point::new(0.0, 0.0), Point::new(10.0, 20.0), false);
        assert_eq!(pt, Point::new(0.0, 0.0));
        assert_eq!(w, 10.0);
        assert_eq!(h, 20.0);
    }

    #[test]
    fn rect_from_drag_shift_squares() {
        let (pt, w, h) = rect_from_drag(Point::new(0.0, 0.0), Point::new(10.0, 20.0), true);
        // max(10,20) = 20, so square of 20. Since cx > ax, x stays at 0.
        assert_eq!(pt, Point::new(0.0, 0.0));
        assert_eq!(w, 20.0);
        assert_eq!(h, 20.0);
    }

    #[test]
    fn rect_from_drag_anchor_reversed() {
        let (pt, w, h) = rect_from_drag(Point::new(10.0, 20.0), Point::new(0.0, 0.0), false);
        // Should normalize to top-left
        assert_eq!(pt, Point::new(0.0, 0.0));
        assert_eq!(w, 10.0);
        assert_eq!(h, 20.0);
    }
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
