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
    /// How the points are connected when rendering.
    pub curve_mode: CurveMode,
}

impl FromDrag for Line {
    fn from_drag(anchor: Point, current: Point, color: ShapeColor, shift: bool) -> Self {
        let (a, b) = line_endpoints(anchor, current, shift);
        Self {
            data: ElementData {
                stroke_color: color,
                ..ElementData::new(0)
            },
            points: vec![a, b],
            curve_mode: CurveMode::default(),
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

impl Render for Line {
    fn render(&self, _zoom: f64) -> leptos::View {
        let sw = self.data.stroke_width;
        let stroke = ShapeColor::to_hex(self.data.stroke_color);
        let d = path_d(&self.points, self.curve_mode);
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

impl HitTest for Line {
    fn hit_test(&self, point: Point, margin: f64) -> bool {
        hit_test_path(
            &self.points,
            self.curve_mode,
            (point.x, point.y),
            margin + self.data.stroke_width,
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
