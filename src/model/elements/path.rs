use crate::model::resize::{resize_bbox, ResizeContext};
use crate::model::Point;

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

    let mut d = format!("M{} {}", points[0].x, points[0].y);
    use std::fmt::Write;
    for i in 0..n - 1 {
        let Some((_, b1, b2, p1)) = segment_cubic_bezier(points, i) else {
            continue;
        };
        let _ = write!(d, " C{} {} {} {} {} {}", b1.x, b1.y, b2.x, b2.y, p1.x, p1.y);
    }
    d
}

/// Return the four cubic-Bezier control points for segment `i` (between
/// `points[i]` and `points[i+1]`) of a Catmull-Rom spline.
///
/// Returns `None` when `i` is out of range.
fn segment_cubic_bezier(points: &[Point], i: usize) -> Option<(Point, Point, Point, Point)> {
    if i + 1 >= points.len() {
        return None;
    }
    let p0 = &points[i];
    let p3 = &points[i + 1];
    let p_before = if i == 0 { &points[0] } else { &points[i - 1] };
    let p_after = if i + 2 >= points.len() {
        &points[points.len() - 1]
    } else {
        &points[i + 2]
    };

    let b1 = Point {
        x: p0.x + (p3.x - p_before.x) / 6.0,
        y: p0.y + (p3.y - p_before.y) / 6.0,
    };
    let b2 = Point {
        x: p3.x - (p_after.x - p0.x) / 6.0,
        y: p3.y - (p_after.y - p0.y) / 6.0,
    };

    Some((*p0, b1, b2, *p3))
}

/// Evaluate a cubic Bezier at parameter `t` (0.0 – 1.0).
pub fn eval_cubic_bezier(p0: &Point, p1: &Point, p2: &Point, p3: &Point, t: f64) -> Point {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    Point {
        x: mt3 * p0.x + 3.0 * mt2 * t * p1.x + 3.0 * mt * t2 * p2.x + t3 * p3.x,
        y: mt3 * p0.y + 3.0 * mt2 * t * p1.y + 3.0 * mt * t2 * p2.y + t3 * p3.y,
    }
}

/// Test a query point against a cubic Bezier curve by subdividing it into
/// `subdivisions` straight-line segments.
fn hit_test_cubic_bezier(
    p0: &Point,
    p1: &Point,
    p2: &Point,
    p3: &Point,
    q: (f64, f64),
    tolerance: f64,
    subdivisions: usize,
) -> bool {
    let (qx, qy) = q;
    let mut prev = eval_cubic_bezier(p0, p1, p2, p3, 0.0);
    for i in 1..=subdivisions {
        let t = i as f64 / subdivisions as f64;
        let cur = eval_cubic_bezier(p0, p1, p2, p3, t);
        if Point::dist_to_segment(Point::new(qx, qy), prev, cur) <= tolerance {
            return true;
        }
        prev = cur;
    }
    false
}

/// Hit-test a point against a (possibly curved) path.
///
/// Checks distance to each point first, then checks segments (straight line
/// in `Straight` mode; subdivided Bezier in `Curved` mode).
pub fn hit_test_path(points: &[Point], mode: CurveMode, point: (f64, f64), tolerance: f64) -> bool {
    let (px, py) = point;
    if points.is_empty() {
        return false;
    }
    for p in points {
        if (px - p.x).hypot(py - p.y) <= tolerance {
            return true;
        }
    }
    if points.len() <= 2 || mode == CurveMode::Straight {
        for i in 1..points.len() {
            let a = &points[i - 1];
            let b = &points[i];
            if Point::dist_to_segment(Point::new(px, py), *a, *b) <= tolerance {
                return true;
            }
        }
    } else {
        for i in 0..points.len() - 1 {
            if let Some((p0, p1, p2, p3)) = segment_cubic_bezier(points, i) {
                if hit_test_cubic_bezier(&p0, &p1, &p2, &p3, point, tolerance, 12) {
                    return true;
                }
            }
        }
    }
    false
}

/// Return the position along a path segment at t = 0.5 for ghost-midpoint
/// placement. In `Straight` mode this is the linear midpoint; in `Curved`
/// mode it is the midpoint of the cubic Bezier.
pub fn segment_midpoint(points: &[Point], mode: CurveMode, i: usize) -> Option<(f64, f64)> {
    if i + 1 >= points.len() {
        return None;
    }
    if points.len() <= 2 || mode == CurveMode::Straight {
        Some((
            (points[i].x + points[i + 1].x) / 2.0,
            (points[i].y + points[i + 1].y) / 2.0,
        ))
    } else {
        let (p0, p1, p2, p3) = segment_cubic_bezier(points, i)?;
        let mid = eval_cubic_bezier(&p0, &p1, &p2, &p3, 0.5);
        Some((mid.x, mid.y))
    }
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
    use crate::model::resize::MIN_ELEMENT_SIZE;
    let rctx = ctx;
    let (pos, (nw, nh)) = match resize_bbox(
        Point { x: rctx.bx, y: rctx.by },
        (rctx.bw, rctx.bh),
        rctx.pointer_world,
        rctx.handle,
        rctx.shift,
        rctx.alt,
    ) {
        Some(v) => v,
        None => return,
    };
    let obw = rctx.bw.max(MIN_ELEMENT_SIZE);
    let obh = rctx.bh.max(MIN_ELEMENT_SIZE);
    let sx = nw / obw;
    let sy = nh / obh;
    for (p, op) in points.iter_mut().zip(orig.iter()) {
        p.set(
            (op.x - rctx.bx) * sx + pos.x,
            (op.y - rctx.by) * sy + pos.y,
        );
    }
}

/// Rotate every point around the pivot `(cx, cy)` by `delta` radians.
pub fn rotate_points(points: &mut [Point], cx: f64, cy: f64, delta: f64) {
    let pivot = Point::new(cx, cy);
    for p in points {
        p.rotate_around(pivot, delta);
    }
}

/// Offset (translate) every point by `(dx, dy)`.
pub fn offset_points(points: &mut [Point], dx: f64, dy: f64) {
    for p in points {
        p.offset(dx, dy);
    }
}

/// Snap every point to the nearest grid line.
pub fn snap_points_to_grid(points: &mut [Point], grid: f64) {
    for p in points {
        p.snap_to_grid(grid);
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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Point;

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
        assert!(hit_test_path(&pts, CurveMode::Straight, (0.0, 0.0), 5.0));
        // On the line with generous tolerance
        assert!(hit_test_path(&pts, CurveMode::Straight, (50.0, 50.0), 5.0));
        // Off the line by ~2.8 units, tolerance 1.0 — should miss
        assert!(!hit_test_path(&pts, CurveMode::Straight, (50.0, 54.0), 1.0));
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

}
