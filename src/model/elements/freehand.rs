use crate::model::elements::utils::scale_points;
use crate::model::resize::ResizeContext;
use crate::model::*;
use leptos::IntoView;
use std::fmt::Write;

/// Minimum distance (screen pixels) between consecutive sampled points.
/// Points closer than this are discarded to keep the point array lean; the
/// threshold is converted to world units with the current zoom, so the sample
/// density stays constant on screen at every zoom level.
const MIN_SAMPLE_DIST_PX: f64 = 2.0;

/// Tolerance (screen pixels) of the Ramer–Douglas–Peucker pass applied when
/// the stroke ends. Converted to world units with the zoom at release time,
/// so simplification never visibly shifts the stroke on screen.
const SIMPLIFY_EPSILON_PX: f64 = 0.5;

/// A free-hand stroke made up of a list of sampled points.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Freehand {
    /// Stroke appearance (width, colour).
    pub data: ElementData,
    /// Sampled world-space points along the stroke path.
    pub points: Vec<Point>,
}

impl Freehand {
    /// Simplify this stroke using the Ramer–Douglas–Peucker algorithm,
    /// keeping points whose perpendicular distance from the simplified
    /// line segment exceeds `epsilon`.
    pub fn simplify(&mut self, epsilon: f64) {
        if self.points.len() > 2 {
            self.points = simplify_points(&self.points, epsilon);
        }
        // Drop the growth capacity reserved while the stroke was live; the
        // finished stroke only needs one slot per surviving point.
        self.points = std::mem::take(&mut self.points)
            .into_boxed_slice()
            .into_vec();
    }

    /// Finalise a live stroke, simplifying it with a tolerance that
    /// corresponds to [`SIMPLIFY_EPSILON_PX`] screen pixels at `zoom`.
    pub fn finish(&mut self, zoom: f64) {
        self.simplify(SIMPLIFY_EPSILON_PX / zoom);
    }
}

// ── Ramer–Douglas–Peucker simplification ───────────────────────────────

fn simplify_points(points: &[Point], epsilon: f64) -> Vec<Point> {
    let n = points.len();
    if n <= 2 {
        return points.to_vec();
    }

    let (x1, y1) = (points[0].x, points[0].y);
    let (x2, y2) = (points[n - 1].x, points[n - 1].y);

    let mut max_dist = 0.0;
    let mut max_idx = 0;

    for (i, p) in points[1..n - 1].iter().enumerate() {
        let dist = perpendicular_dist(p.x, p.y, x1, y1, x2, y2);
        if dist > max_dist {
            max_dist = dist;
            max_idx = i + 1;
        }
    }

    if max_dist > epsilon {
        let mut left = simplify_points(&points[..=max_idx], epsilon);
        let right = simplify_points(&points[max_idx..], epsilon);
        left.pop();
        left.extend(right);
        left
    } else {
        vec![points[0], points[n - 1]]
    }
}

fn perpendicular_dist(px: f64, py: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len_sq = dx * dx + dy * dy;
    if len_sq == 0.0 {
        return (px - x1).hypot(py - y1);
    }
    let t = ((px - x1) * dx + (py - y1) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let proj_x = x1 + t * dx;
    let proj_y = y1 + t * dy;
    (px - proj_x).hypot(py - proj_y)
}

// ── Trait implementations ──────────────────────────────────────────────

impl FromDrag for Freehand {
    fn from_drag(anchor: Point, _current: Point, color: Color, _shift: bool) -> Self {
        Self {
            data: ElementData {
                style: super::ElementStyle {
                    stroke_color: color,
                    ..Default::default()
                },
                ..ElementData::new(0)
            },
            points: vec![Point {
                x: anchor.x,
                y: anchor.y,
            }],
        }
    }
}

impl UpdateDrag for Freehand {
    fn update_drag(&mut self, current: Point, _anchor: Point, _shift: bool, zoom: f64) {
        if let Some(last) = self.points.last() {
            let dx = current.x - last.x;
            let dy = current.y - last.y;
            if dx.hypot(dy) < MIN_SAMPLE_DIST_PX / zoom {
                return;
            }
        }
        self.points.push(Point {
            x: current.x,
            y: current.y,
        });
    }
}

impl Render for Freehand {
    fn render(&self, _zoom: f64) -> leptos::View {
        let sw = self.data.style.stroke_width;
        let stroke = self.data.style.stroke_color.to_hex();
        let dash = self.data.style.stroke_style.stroke_dasharray();
        let linejoin = self.data.style.edge_style.stroke_linejoin();
        let linecap = match self.data.style.edge_style {
            super::EdgeStyle::Sharp => "butt",
            super::EdgeStyle::Rounded => "round",
        };
        let opacity = self.data.style.opacity;
        let d = build_smooth_path(&self.points);
        leptos::view! {
            <path
                d=d
                fill="none"
                stroke=stroke
                stroke-width=sw
                stroke-linecap=linecap
                stroke-linejoin=linejoin
                stroke-dasharray=dash
                opacity=opacity
            />
        }
        .into_view()
    }
}

/// Build an SVG path `d` string from a list of points using quadratic
/// bezier curves (`Q` commands) for smooth interpolation.
///
/// The technique uses midpoints between consecutive original points as
/// segment endpoints and the original points as control points, producing
/// a smooth curve that passes through every original point.
pub(crate) fn build_smooth_path(points: &[Point]) -> String {
    let n = points.len();
    if n == 0 {
        return String::new();
    }
    if n == 1 {
        return format!("M{} {}", points[0].x, points[0].y);
    }

    let mut d = format!("M{} {}", points[0].x, points[0].y);

    if n == 2 {
        let _ = write!(d, " L{} {}", points[1].x, points[1].y);
        return d;
    }

    // Lead-in: straight line from the first point to the first midpoint
    let mx = (points[0].x + points[1].x) / 2.0;
    let my = (points[0].y + points[1].y) / 2.0;
    let _ = write!(d, " L{} {}", mx, my);

    // Quadratic bezier through each intermediate point, landing at the
    // midpoint between it and the next point.
    for i in 1..n - 1 {
        let cp = &points[i];
        let mid_x = (cp.x + points[i + 1].x) / 2.0;
        let mid_y = (cp.y + points[i + 1].y) / 2.0;
        let _ = write!(d, " Q{} {} {} {}", cp.x, cp.y, mid_x, mid_y);
    }

    // Lead-out: straight line from the last midpoint to the final point
    let _ = write!(d, " L{} {}", points[n - 1].x, points[n - 1].y);

    d
}

// ── Geometry traits ────────────────────────────────────────────────────

impl HitTest for Freehand {
    fn hit_test(&self, point: Point, margin: f64) -> bool {
        crate::model::elements::path::hit_test_path(
            &self.points,
            crate::model::elements::path::CurveMode::Straight,
            (point.x, point.y),
            margin + self.data.style.stroke_width,
        )
    }
}

impl Bounds for Freehand {
    fn bounds(&self) -> Rect {
        crate::model::elements::path::bounds_of_points(&self.points)
    }
}

impl Offset for Freehand {
    fn offset(&mut self, dx: f64, dy: f64) {
        self.data.world_point.offset(dx, dy);
        crate::model::elements::path::offset_points(&mut self.points, dx, dy);
    }
}

impl Rotate for Freehand {
    fn rotate_around(&mut self, point: Point, delta: f64) {
        crate::model::elements::path::rotate_points(&mut self.points, point.x, point.y, delta);
    }
}

impl Resize for Freehand {
    fn resize(&mut self, ctx: &ResizeContext) {
        scale_points(&mut self.points, ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stroke(points: &[(f64, f64)]) -> Freehand {
        Freehand {
            data: ElementData::new(1),
            points: points.iter().map(|&(x, y)| Point { x, y }).collect(),
        }
    }

    #[test]
    fn sampling_threshold_follows_the_zoom() {
        // At 20× a 0.2-unit step is 4 screen px: sampled.
        let mut fh = stroke(&[(0.0, 0.0)]);
        fh.update_drag(
            Point { x: 0.2, y: 0.0 },
            Point { x: 0.0, y: 0.0 },
            false,
            20.0,
        );
        assert_eq!(fh.points.len(), 2);
        // At 0.1× the same step is 0.02 screen px: dropped.
        let mut fh = stroke(&[(0.0, 0.0)]);
        fh.update_drag(
            Point { x: 0.2, y: 0.0 },
            Point { x: 0.0, y: 0.0 },
            false,
            0.1,
        );
        assert_eq!(fh.points.len(), 1);
    }

    #[test]
    fn finish_keeps_detail_only_while_it_is_visible() {
        // A zigzag 0.4 world units off the straight line: at 20× that is 8
        // screen px of visible detail, at 0.1× it is 0.04 px and collapses.
        let pts = [(0.0, 0.0), (10.0, 0.4), (20.0, 0.0)];
        let mut zoomed_in = stroke(&pts);
        let mut zoomed_out = stroke(&pts);
        zoomed_in.finish(20.0);
        zoomed_out.finish(0.1);
        assert_eq!(zoomed_in.points.len(), 3);
        assert_eq!(zoomed_out.points.len(), 2);
    }

    #[test]
    fn finish_releases_growth_capacity() {
        let mut fh = stroke(&[(0.0, 0.0), (5.0, 0.0), (10.0, 0.0)]);
        fh.points.reserve(64);
        fh.finish(1.0);
        assert_eq!(fh.points.len(), 2);
        assert_eq!(fh.points.capacity(), 2);
    }
}
