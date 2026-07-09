use super::{Point, ResizeContext};

/// How a set of path points should be interpreted when rendering.
///
/// This is a whole-shape toggle — every segment in the path follows the
/// same curve rule. There are no per-point tangent handles or corner-vs-
/// smooth modes (those would be a materially larger feature).
#[derive(Clone, Copy, Debug, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum CurveMode {
    /// Render as straight line segments (a polyline).
    #[default]
    Straight,
    /// Render as a smooth Catmull-Rom spline converted to cubic Beziers,
    /// passing exactly through every point.
    Curved,
}

/// Build an SVG path `d` attribute string from a list of points.
///
/// * 0 points → empty string.
/// * 1 point → `M x,y` (a point with no path).
/// * 2 points → `M...L...` regardless of `mode` (a two-point Catmull-Rom
///   curve has no interior curvature, and the Bezier formula would produce
///   degenerate control points).
/// * 3+ points in `Straight` mode → `M x,y` followed by `L x,y` per point.
/// * 3+ points in `Curved` mode → Catmull-Rom-to-Bezier conversion.
pub fn path_d(points: &[Point], mode: CurveMode) -> String {
    let n = points.len();
    if n == 0 {
        return String::new();
    }
    if n == 1 {
        return format!("M{} {}", points[0].x, points[0].y);
    }

    // For 2 points, both modes reduce to a single straight segment.
    if n == 2 || mode == CurveMode::Straight {
        let mut d = format!("M{} {}", points[0].x, points[0].y);
        for p in &points[1..] {
            use std::fmt::Write;
            let _ = write!(d, " L{} {}", p.x, p.y);
        }
        return d;
    }

    // Catmull-Rom -> cubic Bezier conversion.
    // For an open path, the first and last points are duplicated as their
    // own "previous" and "next" neighbours so the formula applies everywhere.
    //
    // For each segment P[i] → P[i+1], with neighbours P[i-1] and P[i+2]:
    //   B0 = P[i]
    //   B1 = P[i] + (P[i+1] - P[i-1]) / 6
    //   B2 = P[i+1] - (P[i+2] - P[i]) / 6
    //   B3 = P[i+1]
    let mut d = format!("M{} {}", points[0].x, points[0].y);
    use std::fmt::Write;

    // synthetic neighbours: first point acts as its own previous,
    // last point acts as its own next
    let prev = &points[0];
    for i in 0..n - 1 {
        let p0 = &points[i];
        let p1 = &points[i + 1];
        let p_before = if i == 0 { prev } else { &points[i - 1] };
        let p_after = if i + 2 >= n { &points[n - 1] } else { &points[i + 2] };

        let b1x = p0.x + (p1.x - p_before.x) / 6.0;
        let b1y = p0.y + (p1.y - p_before.y) / 6.0;
        let b2x = p1.x - (p_after.x - p0.x) / 6.0;
        let b2y = p1.y - (p_after.y - p0.y) / 6.0;

        let _ = write!(d, " C{} {} {} {} {} {}", b1x, b1y, b2x, b2y, p1.x, p1.y);
    }

    d
}

/// Hit-test a point against an open polyline path.
///
/// Checks distance to each point first, then perpendicular distance to each
/// consecutive segment.
///
/// **Known limitation:** this always uses straight-line segments between
/// consecutive points as an approximation, even when the path is rendered
/// in `Curved` mode. The visual curve and the hit-test region will differ
/// slightly near bends. This is an acceptable simplification for now.
pub fn hit_test_path(points: &[Point], point: (f64, f64), tolerance: f64) -> bool {
    let (px, py) = point;
    if points.is_empty() {
        return false;
    }
    for p in points {
        if (px - p.x).hypot(py - p.y) <= tolerance {
            return true;
        }
    }
    for i in 1..points.len() {
        let a = &points[i - 1];
        let b = &points[i];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1.0 {
            continue;
        }
        let t = ((px - a.x) * dx + (py - a.y) * dy) / (len * len);
        let t = t.clamp(0.0, 1.0);
        let nx = a.x + t * dx;
        let ny = a.y + t * dy;
        if (px - nx).hypot(py - ny) <= tolerance {
            return true;
        }
    }
    false
}

/// Compute the axis-aligned bounding box of a list of points.
///
/// Returns `(min_x, min_y, width, height)`. Returns `(0, 0, 0, 0)` for an
/// empty slice.
pub fn bounds_of_points(points: &[Point]) -> (f64, f64, f64, f64) {
    if points.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let min_x = points.iter().map(|p| p.x).reduce(f64::min).unwrap();
    let min_y = points.iter().map(|p| p.y).reduce(f64::min).unwrap();
    let max_x = points.iter().map(|p| p.x).reduce(f64::max).unwrap();
    // Since the path is open and has no "far side", add the
    // stroke width to the bounding box for selection purposes.
    let max_y = points.iter().map(|p| p.y).reduce(f64::max).unwrap();
    (min_x, min_y, max_x - min_x, max_y - min_y)
}

/// Scale all points using the same affine bounding-box transform as the
/// existing resize logic.
///
/// `orig` is the pre-drag point slice (from the undo snapshot) —
/// each output point position = `(orig_point - bx) * sx + nx`.
pub fn scale_points(points: &mut [Point], ctx: &ResizeContext, orig: &[Point]) {
    use super::rect::MIN_ELEMENT_SIZE;
    let rctx = ctx;
    let (nx, ny, nw, nh) = match rctx.handle {
        0 => (rctx.bx + rctx.dx, rctx.by + rctx.dy, rctx.bw - rctx.dx, rctx.bh - rctx.dy),
        1 => (rctx.bx, rctx.by + rctx.dy, rctx.bw, rctx.bh - rctx.dy),
        2 => (rctx.bx, rctx.by + rctx.dy, rctx.bw + rctx.dx, rctx.bh - rctx.dy),
        3 => (rctx.bx + rctx.dx, rctx.by, rctx.bw - rctx.dx, rctx.bh),
        4 => (rctx.bx, rctx.by, rctx.bw + rctx.dx, rctx.bh),
        5 => (rctx.bx + rctx.dx, rctx.by, rctx.bw - rctx.dx, rctx.bh + rctx.dy),
        6 => (rctx.bx, rctx.by, rctx.bw, rctx.bh + rctx.dy),
        7 => (rctx.bx, rctx.by, rctx.bw + rctx.dx, rctx.bh + rctx.dy),
        _ => return,
    };
    if nw < MIN_ELEMENT_SIZE || nh < MIN_ELEMENT_SIZE {
        return;
    }
    let obw = rctx.bw.max(MIN_ELEMENT_SIZE);
    let obh = rctx.bh.max(MIN_ELEMENT_SIZE);
    let sx = nw / obw;
    let sy = nh / obh;
    for (p, op) in points.iter_mut().zip(orig.iter()) {
        p.x = (op.x - rctx.bx) * sx + nx;
        p.y = (op.y - rctx.by) * sy + ny;
    }
}

/// Rotate every point around the pivot `(cx, cy)` by `delta` radians.
pub fn rotate_points(points: &mut [Point], cx: f64, cy: f64, delta: f64) {
    let cos = delta.cos();
    let sin = delta.sin();
    for p in points {
        let dx = p.x - cx;
        let dy = p.y - cy;
        p.x = cx + dx * cos - dy * sin;
        p.y = cy + dx * sin + dy * cos;
    }
}

/// Offset (translate) every point by `(dx, dy)`.
pub fn offset_points(points: &mut [Point], dx: f64, dy: f64) {
    for p in points {
        p.x += dx;
        p.y += dy;
    }
}

/// Snap every point to the nearest grid line.
pub fn snap_points_to_grid(points: &mut [Point], grid: f64) {
    for p in points {
        p.x = (p.x / grid).round() * grid;
        p.y = (p.y / grid).round() * grid;
    }
}

/// Returns the 10 handle positions for the given bounding box.
///
/// Indices 0–7 are resize corners/edges, 8 is the move handle (center),
/// 9 is the rotate handle (above the box).
///
/// This is used by `selection.rs` for the generic bounding-box handle overlay
/// for every element type (Rectangle, Ellipse, Text, Freehand, and multi-
/// selections). It is NOT used for single-selection Line/Arrow node editing.
pub fn handle_positions(bx: f64, by: f64, bw: f64, bh: f64) -> [(f64, f64); 10] {
    [
        (bx, by),
        (bx + bw / 2.0, by),
        (bx + bw, by),
        (bx, by + bh / 2.0),
        (bx + bw, by + bh / 2.0),
        (bx, by + bh),
        (bx + bw / 2.0, by + bh),
        (bx + bw, by + bh),
        (bx + bw / 2.0, by + bh / 2.0),
        (bx + bw / 2.0, by - 25.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::elements::Point;

    #[test]
    fn path_d_empty() {
        assert_eq!(path_d(&[], CurveMode::Straight), "");
        assert_eq!(path_d(&[], CurveMode::Curved), "");
    }

    #[test]
    fn path_d_single_point() {
        let pts = [Point { x: 10.0, y: 20.0 }];
        assert_eq!(path_d(&pts, CurveMode::Straight), "M10 20");
        assert_eq!(path_d(&pts, CurveMode::Curved), "M10 20");
    }

    #[test]
    fn path_d_two_points_straight() {
        let pts = [Point { x: 0.0, y: 0.0 }, Point { x: 100.0, y: 100.0 }];
        assert_eq!(path_d(&pts, CurveMode::Straight), "M0 0 L100 100");
    }

    #[test]
    fn path_d_two_points_curved_reduces_to_straight() {
        let pts = [Point { x: 0.0, y: 0.0 }, Point { x: 100.0, y: 100.0 }];
        // 2-point Catmull-Rom has no interior curvature; should emit M...L...
        assert_eq!(path_d(&pts, CurveMode::Curved), "M0 0 L100 100");
    }

    #[test]
    fn path_d_three_points_straight() {
        let pts = [
            Point { x: 0.0, y: 0.0 },
            Point { x: 50.0, y: 100.0 },
            Point { x: 100.0, y: 0.0 },
        ];
        assert_eq!(path_d(&pts, CurveMode::Straight), "M0 0 L50 100 L100 0");
    }

    #[test]
    fn path_d_three_points_curved() {
        // Points: (0,0), (50,100), (100,0)
        let pts = [
            Point { x: 0.0, y: 0.0 },
            Point { x: 50.0, y: 100.0 },
            Point { x: 100.0, y: 0.0 },
        ];
        let d = path_d(&pts, CurveMode::Curved);
        // Check it starts correctly and contains the expected segments
        assert!(d.starts_with("M0 0 C"));
        assert!(d.contains("C"));
        // Verify the string has two cubic bezier segments (two 'C' commands)
        let c_count = d.matches("C").count();
        assert_eq!(c_count, 2, "expected 2 cubic bezier segments, got: {d}");
    }

    #[test]
    fn bounds_of_points_works() {
        let pts = [
            Point { x: -10.0, y: -20.0 },
            Point { x: 30.0, y: 40.0 },
            Point { x: 5.0, y: 5.0 },
        ];
        let (x, y, w, h) = bounds_of_points(&pts);
        assert_eq!(x, -10.0);
        assert_eq!(y, -20.0);
        assert_eq!(w, 40.0);
        assert_eq!(h, 60.0);
    }

    #[test]
    fn bounds_of_points_empty() {
        assert_eq!(bounds_of_points(&[]), (0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn hit_test_path_hits_point() {
        let pts = [Point { x: 0.0, y: 0.0 }, Point { x: 100.0, y: 100.0 }];
        assert!(hit_test_path(&pts, (0.0, 0.0), 5.0));
        // On the line with generous tolerance
        assert!(hit_test_path(&pts, (50.0, 50.0), 5.0));
        // Off the line by ~2.8 units, tolerance 1.0 — should miss
        assert!(!hit_test_path(&pts, (50.0, 54.0), 1.0));
    }

    #[test]
    fn offset_points_works() {
        let mut pts = [Point { x: 1.0, y: 2.0 }, Point { x: 3.0, y: 4.0 }];
        offset_points(&mut pts, 10.0, 20.0);
        assert_eq!(pts[0].x, 11.0);
        assert_eq!(pts[0].y, 22.0);
        assert_eq!(pts[1].x, 13.0);
        assert_eq!(pts[1].y, 24.0);
    }

    #[test]
    fn rotate_points_works() {
        let mut pts = [Point { x: 1.0, y: 0.0 }, Point { x: 0.0, y: 1.0 }];
        rotate_points(&mut pts, 0.0, 0.0, std::f64::consts::FRAC_PI_2);
        assert!((pts[0].x - 0.0).abs() < 1e-10);
        assert!((pts[0].y - 1.0).abs() < 1e-10);
        assert!((pts[1].x + 1.0).abs() < 1e-10);
        assert!((pts[1].y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn snap_points_to_grid_works() {
        let mut pts = [Point { x: 13.0, y: 27.0 }];
        snap_points_to_grid(&mut pts, 20.0);
        assert_eq!(pts[0].x, 20.0);
        assert_eq!(pts[0].y, 20.0);
    }

    #[test]
    fn handle_positions_count() {
        let h = handle_positions(0.0, 0.0, 100.0, 50.0);
        assert_eq!(h.len(), 10);
        assert_eq!(h[0], (0.0, 0.0));
        assert_eq!(h[8], (50.0, 25.0));
        assert_eq!(h[9], (50.0, -25.0));
    }
}