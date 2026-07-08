use leptos::IntoView;
use super::{ElementData, Point, ShapeColor};
use super::{
    Bounds, FromDrag, HitTest, Offset, Render, Resize, ResizeContext, Rotate, SnapToGrid,
    UpdateDrag,
};
use super::rect::MIN_ELEMENT_SIZE;
use std::fmt::Write;

/// Minimum distance (world-space) between consecutive sampled points.
/// Points closer than this are discarded to keep the point array lean.
const MIN_SAMPLE_DIST: f64 = 2.0;

/// Epsilon for Ramer–Douglas–Peucker simplification applied on pointer-up.
const SIMPLIFY_EPSILON: f64 = 0.5;

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
        if self.points.len() <= 2 {
            return;
        }
        self.points = simplify_points(&self.points, epsilon);
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

    for i in 1..n - 1 {
        let dist = perpendicular_dist(points[i].x, points[i].y, x1, y1, x2, y2);
        if dist > max_dist {
            max_dist = dist;
            max_idx = i;
        }
    }

    if max_dist > epsilon {
        let mut left = simplify_points(&points[..=max_idx], epsilon);
        let right = simplify_points(&points[max_idx..], epsilon);
        left.pop();
        left.extend(right);
        left
    } else {
        vec![points[0].clone(), points[n - 1].clone()]
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
    fn from_drag(
        anchor: (f64, f64),
        _current: (f64, f64),
        color: ShapeColor,
        _shift: bool,
    ) -> Self {
        let mut fh = Self {
            data: ElementData {
                stroke_color: color,
                ..ElementData::new(0)
            },
            points: vec![Point { x: anchor.0, y: anchor.1 }],
        };
        fh.simplify(SIMPLIFY_EPSILON);
        fh
    }
}

impl UpdateDrag for Freehand {
    fn update_drag(&mut self, current: (f64, f64), _anchor: (f64, f64), _shift: bool) {
        if let Some(last) = self.points.last() {
            let dx = current.0 - last.x;
            let dy = current.1 - last.y;
            if dx.hypot(dy) < MIN_SAMPLE_DIST {
                return;
            }
        }
        self.points.push(Point {
            x: current.0,
            y: current.1,
        });
    }
}

impl Render for Freehand {
    fn render(&self, _zoom: f64) -> leptos::View {
        let sw = self.data.stroke_width;
        let stroke = ShapeColor::to_hex(self.data.stroke_color);
        let d = build_smooth_path(&self.points);
        leptos::view! {
            <path d=d fill="none" stroke=stroke stroke-width=sw stroke-linecap="round" stroke-linejoin="round" />
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
fn build_smooth_path(points: &[Point]) -> String {
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
    fn hit_test(&self, point: (f64, f64), margin: f64) -> bool {
        let (px, py) = point;
        let tolerance = margin + self.data.stroke_width;
        if self.points.is_empty() {
            return false;
        }
        for p in &self.points {
            if (px - p.x).hypot(py - p.y) <= tolerance {
                return true;
            }
        }
        for i in 1..self.points.len() {
            let a = &self.points[i - 1];
            let b = &self.points[i];
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
}

impl Bounds for Freehand {
    fn bounds(&self) -> (f64, f64, f64, f64) {
        if self.points.is_empty() {
            return (0.0, 0.0, 0.0, 0.0);
        }
        let min_x = self.points.iter().map(|p| p.x).reduce(f64::min).unwrap();
        let min_y = self.points.iter().map(|p| p.y).reduce(f64::min).unwrap();
        let max_x = self.points.iter().map(|p| p.x).reduce(f64::max).unwrap();
        let max_y = self.points.iter().map(|p| p.y).reduce(f64::max).unwrap();
        (min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

impl Offset for Freehand {
    fn offset(&mut self, dx: f64, dy: f64) {
        self.data.x += dx;
        self.data.y += dy;
        for p in &mut self.points {
            p.x += dx;
            p.y += dy;
        }
    }
}

impl SnapToGrid for Freehand {
    fn snap_to_grid(&mut self, grid: f64) {
        for p in &mut self.points {
            p.x = (p.x / grid).round() * grid;
            p.y = (p.y / grid).round() * grid;
        }
    }
}

impl Rotate for Freehand {
    fn rotate_around(&mut self, cx: f64, cy: f64, delta: f64) {
        let cos = delta.cos();
        let sin = delta.sin();
        for p in &mut self.points {
            let dx = p.x - cx;
            let dy = p.y - cy;
            p.x = cx + dx * cos - dy * sin;
            p.y = cy + dx * sin + dy * cos;
        }
    }
}

impl Resize for Freehand {
    fn resize(&mut self, ctx: &ResizeContext) {
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
        if rctx.multi {
            if let super::Element::Freehand(orig) = rctx.orig {
                for (p, op) in self.points.iter_mut().zip(orig.points.iter()) {
                    p.x = (op.x - rctx.bx) * sx + nx;
                    p.y = (op.y - rctx.by) * sy + ny;
                }
            }
        } else {
            for p in &mut self.points {
                p.x = (p.x - rctx.bx) * sx + nx;
                p.y = (p.y - rctx.by) * sy + ny;
            }
        }
    }
}
