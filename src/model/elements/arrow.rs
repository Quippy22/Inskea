use leptos::IntoView;
use super::{ElementData, Point, ShapeColor};
use super::{
    Bounds, FromDrag, HitTest, Offset, Render, Resize, ResizeContext, Rotate, SnapToGrid,
    UpdateDrag,
};
use super::snap_angle;
use super::path::{CurveMode, path_d, bounds_of_points, hit_test_path, offset_points,
    rotate_points, scale_points, snap_points_to_grid};

const SNAP_DIVISIONS: f64 = 8.0;
const ARROW_HEAD_MULT: f64 = 4.0;

/// An arrow from tail to tip, drawn with a V-shaped head at the tip.
///
/// When first created via `FromDrag` this always has exactly 2 points
/// (tail and tip). Additional points can be inserted via the node-editing
/// UI, and the arrowhead always points along the final segment.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Arrow {
    /// Stroke appearance (width, colour).
    pub data: ElementData,
    /// Ordered path points. At minimum 2 (tail, tip).
    pub points: Vec<Point>,
    /// How the connecting path is rendered.
    pub curve_mode: CurveMode,
}

impl FromDrag for Arrow {
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
            points: vec![Point { x: ax, y: ay }, Point { x: ex, y: ey }],
            curve_mode: CurveMode::default(),
        }
    }
}

impl UpdateDrag for Arrow {
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
        if self.points.len() >= 2 {
            self.points[0].x = ax;
            self.points[0].y = ay;
            self.points[1].x = ex;
            self.points[1].y = ey;
        }
    }
}

impl Render for Arrow {
    fn render(&self, _zoom: f64) -> leptos::View {
        let sw = self.data.stroke_width;
        let hex = ShapeColor::to_hex(self.data.stroke_color);
        let d = path_d(&self.points, self.curve_mode);

        // Arrowhead: computed from the last two points so it always points
        // along the final segment regardless of bends.
        let (ux, uy) = if self.points.len() >= 2 {
            let tail = &self.points[self.points.len() - 2];
            let tip = &self.points[self.points.len() - 1];
            let dx = tip.x - tail.x;
            let dy = tip.y - tail.y;
            let len = (dx * dx + dy * dy).sqrt();
            if len > 0.0 { (dx / len, dy / len) } else { (1.0, 0.0) }
        } else {
            (1.0, 0.0)
        };

        let head_size = (sw * ARROW_HEAD_MULT).max(4.0);
        let tip = &self.points[self.points.len() - 1];
        let tip_x = tip.x;
        let tip_y = tip.y;
        let lx = tip_x - ux * head_size - uy * head_size * 0.4;
        let ly = tip_y - uy * head_size + ux * head_size * 0.4;
        let rx = tip_x - ux * head_size + uy * head_size * 0.4;
        let ry = tip_y - uy * head_size - ux * head_size * 0.4;
        let points = format!("{},{} {},{} {},{}", lx, ly, tip_x, tip_y, rx, ry);

        leptos::view! {
            <g stroke=hex stroke-width=sw fill="none" stroke-linejoin="round">
                <path d=d />
                <polyline points=points />
            </g>
        }
        .into_view()
    }
}

impl HitTest for Arrow {
    fn hit_test(&self, point: (f64, f64), margin: f64) -> bool {
        hit_test_path(&self.points, self.curve_mode, point, margin + self.data.stroke_width)
    }
}

impl Bounds for Arrow {
    fn bounds(&self) -> (f64, f64, f64, f64) {
        bounds_of_points(&self.points)
    }
}

impl Offset for Arrow {
    fn offset(&mut self, dx: f64, dy: f64) {
        self.data.x += dx;
        self.data.y += dy;
        offset_points(&mut self.points, dx, dy);
    }
}

impl SnapToGrid for Arrow {
    fn snap_to_grid(&mut self, grid: f64) {
        snap_points_to_grid(&mut self.points, grid);
    }
}

impl Rotate for Arrow {
    fn rotate_around(&mut self, cx: f64, cy: f64, delta: f64) {
        rotate_points(&mut self.points, cx, cy, delta);
    }
}

impl Resize for Arrow {
    fn resize(&mut self, ctx: &ResizeContext) {
        let orig_slice: Vec<Point> = if let super::Element::Arrow(orig) = ctx.orig {
            orig.points.clone()
        } else {
            self.points.clone()
        };
        scale_points(&mut self.points, ctx, &orig_slice);
    }
}
