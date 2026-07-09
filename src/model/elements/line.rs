use leptos::IntoView;
use super::{ElementData, Point, ShapeColor};
use super::{
    Bounds, FromDrag, HitTest, Offset, Render, Resize, ResizeContext, Rotate, SnapToGrid,
    UpdateDrag,
};
use super::snap_angle;

const SNAP_DIVISIONS: f64 = 8.0;

/// A straight line from point A to point B.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Line {
    /// Stroke appearance (width, colour).
    pub data: ElementData,
    /// Start point of the line.
    pub a: Point,
    /// End point of the line.
    pub b: Point,
}

impl FromDrag for Line {
    fn from_drag(
        anchor: (f64, f64),
        current: (f64, f64),
        color: ShapeColor,
        shift: bool,
    ) -> Self {
        let (ax, ay) = anchor;
        let (cx, cy) = current;
        let (mut ex, mut ey) = (cx, cy);
        if shift {
            let dx = cx - ax;
            let dy = cy - ay;
            let angle = dy.atan2(dx);
            let snapped = snap_angle(angle, SNAP_DIVISIONS);
            let dist = (dx * dx + dy * dy).sqrt();
            ex = ax + dist * snapped.cos();
            ey = ay + dist * snapped.sin();
        }
        Self {
            data: ElementData {
                stroke_color: color,
                ..ElementData::new(0)
            },
            a: Point { x: ax, y: ay },
            b: Point { x: ex, y: ey },
        }
    }
}

impl UpdateDrag for Line {
    fn update_drag(&mut self, current: (f64, f64), anchor: (f64, f64), shift: bool) {
        let (ax, ay) = anchor;
        let (cx, cy) = current;
        let (mut ex, mut ey) = (cx, cy);
        if shift {
            let dx = cx - ax;
            let dy = cy - ay;
            let angle = dy.atan2(dx);
            let snapped = snap_angle(angle, SNAP_DIVISIONS);
            let dist = (dx * dx + dy * dy).sqrt();
            ex = ax + dist * snapped.cos();
            ey = ay + dist * snapped.sin();
        }
        self.a.x = ax;
        self.a.y = ay;
        self.b.x = ex;
        self.b.y = ey;
    }
}

impl Render for Line {
    fn render(&self, _zoom: f64) -> leptos::View {
        let sw = self.data.stroke_width;
        let stroke = ShapeColor::to_hex(self.data.stroke_color);
        leptos::view! {
            <line
                x1=self.a.x y1=self.a.y
                x2=self.b.x y2=self.b.y
                stroke=stroke stroke-width=sw
            />
        }
        .into_view()
    }
}

impl HitTest for Line {
    fn hit_test(&self, point: (f64, f64), margin: f64) -> bool {
        let (px, py) = point;
        let dx = self.b.x - self.a.x;
        let dy = self.b.y - self.a.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1.0 {
            return (px - self.a.x).hypot(py - self.a.y) <= margin + self.data.stroke_width;
        }
        let t = ((px - self.a.x) * dx + (py - self.a.y) * dy) / (len * len);
        let t = t.clamp(0.0, 1.0);
        let near_x = self.a.x + t * dx;
        let near_y = self.a.y + t * dy;
        (px - near_x).hypot(py - near_y) <= margin + self.data.stroke_width
    }
}

impl Bounds for Line {
    fn bounds(&self) -> (f64, f64, f64, f64) {
        let x = self.a.x.min(self.b.x);
        let y = self.a.y.min(self.b.y);
        let w = (self.b.x - self.a.x).abs();
        let h = (self.b.y - self.a.y).abs();
        (x, y, w, h)
    }
}

impl Offset for Line {
    fn offset(&mut self, dx: f64, dy: f64) {
        self.a.x += dx;
        self.a.y += dy;
        self.b.x += dx;
        self.b.y += dy;
    }
}

impl SnapToGrid for Line {
    fn snap_to_grid(&mut self, grid: f64) {
        self.a.x = (self.a.x / grid).round() * grid;
        self.a.y = (self.a.y / grid).round() * grid;
        self.b.x = (self.b.x / grid).round() * grid;
        self.b.y = (self.b.y / grid).round() * grid;
    }
}

impl Rotate for Line {
    fn rotate_around(&mut self, cx: f64, cy: f64, delta: f64) {
        let cos = delta.cos();
        let sin = delta.sin();
        let dx1 = self.a.x - cx;
        let dy1 = self.a.y - cy;
        let dx2 = self.b.x - cx;
        let dy2 = self.b.y - cy;
        self.a.x = cx + dx1 * cos - dy1 * sin;
        self.a.y = cy + dx1 * sin + dy1 * cos;
        self.b.x = cx + dx2 * cos - dy2 * sin;
        self.b.y = cy + dx2 * sin + dy2 * cos;
    }
}

impl Resize for Line {
    fn resize(&mut self, ctx: &ResizeContext) {
        let hpos = handle_positions(ctx.bx, ctx.by, ctx.bw, ctx.bh);
        let (hx, hy) = hpos[ctx.handle];
        let dist_a = (self.a.x - hx).hypot(self.a.y - hy);
        let dist_b = (self.b.x - hx).hypot(self.b.y - hy);
        if dist_a < dist_b {
            self.a.x += ctx.fdx;
            self.a.y += ctx.fdy;
        } else {
            self.b.x += ctx.fdx;
            self.b.y += ctx.fdy;
        }
    }
}


