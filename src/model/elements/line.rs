use super::path::{
    bounds_of_points, hit_test_path, offset_points, path_d, rotate_points,
    snap_points_to_grid, CurveMode,
};
use super::utils::{line_endpoints, scale_points};
use super::{
    Bounds, FromDrag, HitTest, Offset, Render, Resize, Rotate, SnapToGrid, UpdateDrag,
};
use super::{ElementData, ShapeColor};
use crate::model::resize::ResizeContext;
use crate::model::Point;
use leptos::IntoView;

/// Line/arrow-specific properties.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LineStyle {
    pub curve_mode: CurveMode,
    #[serde(default)]
    pub has_arrowhead: bool,
}

impl Default for LineStyle {
    fn default() -> Self {
        Self {
            curve_mode: CurveMode::Straight,
            has_arrowhead: false,
        }
    }
}

/// A line defined by an ordered list of path points.
///
/// When first created via `FromDrag` this always has exactly 2 points
/// (the two endpoints). Additional points can be inserted via the
/// node-editing UI (dragging a ghost midpoint handle) to create bends.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Line {
    /// Stroke appearance (width, colour).
    pub data: ElementData,
    /// Ordered path points. At minimum 2 endpoints.
    pub points: Vec<Point>,
    /// Line-specific properties (curve mode, arrowhead).
    pub line_style: LineStyle,
}

impl FromDrag for Line {
    fn from_drag(anchor: Point, current: Point, color: ShapeColor, shift: bool) -> Self {
        let (a, b) = line_endpoints(anchor, current, shift);
        Self {
            data: ElementData {
                style: super::ElementStyle {
                    stroke_color: color,
                    ..Default::default()
                },
                ..ElementData::new(0)
            },
            points: vec![a, b],
            line_style: LineStyle::default(),
        }
    }
}

impl UpdateDrag for Line {
    fn update_drag(&mut self, current: Point, anchor: Point, shift: bool) {
        let (a, b) = line_endpoints(anchor, current, shift);
        if self.points.len() >= 2 {
            self.points[0] = a;
            self.points[1] = b;
        }
    }
}

const ARROW_HEAD_MULT: f64 = 4.0;

impl Render for Line {
    fn render(&self, _zoom: f64) -> leptos::View {
        let sw = self.data.style.stroke_width;
        let stroke = ShapeColor::to_hex(self.data.style.stroke_color);
        let d = path_d(&self.points, self.line_style.curve_mode);

        if self.line_style.has_arrowhead && self.points.len() >= 2 {
            let (ux, uy) = {
                let tail = &self.points[self.points.len() - 2];
                let tip = &self.points[self.points.len() - 1];
                let dx = tip.x - tail.x;
                let dy = tip.y - tail.y;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 0.0 {
                    (dx / len, dy / len)
                } else {
                    (1.0, 0.0)
                }
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
                <g stroke=stroke stroke-width=sw fill="none" stroke-linejoin="round">
                    <path d=d />
                    <polyline points=points />
                </g>
            }
            .into_view()
        } else {
            leptos::view! {
                <path
                    d=d
                    fill="none"
                    stroke=stroke stroke-width=sw
                />
            }
            .into_view()
        }
    }
}

impl HitTest for Line {
    fn hit_test(&self, point: Point, margin: f64) -> bool {
        hit_test_path(
            &self.points,
            self.line_style.curve_mode,
            (point.x, point.y),
            margin + self.data.style.stroke_width,
        )
    }
}

impl Bounds for Line {
    fn bounds(&self) -> (f64, f64, f64, f64) {
        bounds_of_points(&self.points)
    }
}

impl Offset for Line {
    fn offset(&mut self, dx: f64, dy: f64) {
        self.data.world_point.offset(dx, dy);
        offset_points(&mut self.points, dx, dy);
    }
}

impl SnapToGrid for Line {
    fn snap_to_grid(&mut self, grid: f64) {
        snap_points_to_grid(&mut self.points, grid);
    }
}

impl Rotate for Line {
    fn rotate_around(&mut self, point: Point, delta: f64) {
        rotate_points(&mut self.points, point.x, point.y, delta);
    }
}

impl Resize for Line {
    fn resize(&mut self, ctx: &ResizeContext) {
        scale_points(&mut self.points, ctx);
    }
}
