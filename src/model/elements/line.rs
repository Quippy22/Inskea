use crate::model::elements::path::{
    bounds_of_points, hit_test_path, offset_points, path_d, rotate_points,
};
use crate::model::elements::utils::{arrowhead_polyline, line_endpoints, scale_points};
use crate::model::resize::ResizeContext;
use crate::model::*;
use leptos::IntoView;

/// Line/arrow-specific properties.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[repr(C)]
pub struct LineStyle {
    pub curve_mode: CurveMode,
    #[serde(default)]
    pub has_start_arrowhead: bool,
    #[serde(default)]
    pub has_end_arrowhead: bool,
}

impl Default for LineStyle {
    fn default() -> Self {
        Self {
            curve_mode: CurveMode::Straight,
            has_start_arrowhead: false,
            has_end_arrowhead: true,
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
    pub points: Box<[Point]>,
    /// Line-specific properties (curve mode, arrowhead).
    pub line_style: LineStyle,
}

impl FromDrag for Line {
    fn from_drag(anchor: Point, current: Point, color: Color, shift: bool) -> Self {
        let (a, b) = line_endpoints(anchor, current, shift);
        Self {
            data: ElementData {
                style: super::ElementStyle {
                    stroke_color: color,
                    ..Default::default()
                },
                ..ElementData::new(0)
            },
            points: vec![a, b].into_boxed_slice(),
            line_style: LineStyle::default(),
        }
    }
}

impl UpdateDrag for Line {
    fn update_drag(&mut self, current: Point, anchor: Point, shift: bool, _zoom: f64) {
        let (a, b) = line_endpoints(anchor, current, shift);
        if self.points.len() >= 2 {
            self.points[0] = a;
            self.points[1] = b;
        }
    }
}

impl Render for Line {
    fn render(&self, _zoom: f64) -> leptos::View {
        let sw = self.data.style.stroke_width;
        let stroke = self.data.style.stroke_color.to_hex();
        let dash = self.data.style.stroke_style.stroke_dasharray();
        let linejoin = self.data.style.edge_style.stroke_linejoin();
        let linecap = match self.data.style.edge_style {
            super::EdgeStyle::Sharp => "butt",
            super::EdgeStyle::Rounded => "round",
        };
        let d = path_d(&self.points, self.line_style.curve_mode);
        let opacity = self.data.style.opacity;
        let sw2 = sw;

        let start_endpoints = self.points.first().zip(self.points.get(1));
        let end_endpoints = self
            .points
            .len()
            .checked_sub(2)
            .and_then(|index| self.points.get(index))
            .zip(self.points.last());
        let start_arrow = self
            .line_style
            .has_start_arrowhead
            .then(|| start_endpoints.map(|(start, end)| arrowhead_polyline(end, start, sw2)))
            .flatten();
        let end_arrow = self
            .line_style
            .has_end_arrowhead
            .then(|| end_endpoints.map(|(start, end)| arrowhead_polyline(start, end, sw2)))
            .flatten();

        let has_any_arrow = start_arrow.is_some() || end_arrow.is_some();

        if has_any_arrow {
            leptos::view! {
                <g
                    stroke=stroke
                    stroke-width=sw
                    fill="none"
                    stroke-linejoin=linejoin
                    stroke-linecap=linecap
                    stroke-dasharray=dash
                    opacity=opacity
                >
                    <path d=d />
                    {start_arrow.map(|p| leptos::view! { <polyline points=p /> })}
                    {end_arrow.map(|p| leptos::view! { <polyline points=p /> })}
                </g>
            }
            .into_view()
        } else {
            leptos::view! {
                <path
                    d=d
                    fill="none"
                    stroke=stroke
                    stroke-width=sw
                    stroke-linejoin=linejoin
                    stroke-linecap=linecap
                    stroke-dasharray=dash
                    opacity=opacity
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
    fn bounds(&self) -> Rect {
        bounds_of_points(&self.points)
    }
}

impl Offset for Line {
    fn offset(&mut self, dx: f64, dy: f64) {
        self.data.world_point.offset(dx, dy);
        offset_points(&mut self.points, dx, dy);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_editing_inserts_and_removes_points() {
        let mut line = Line::from_drag(
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Color::new(Color::BLUE),
            false,
        );
        {
            let points = line.path_points_mut().unwrap();
            points.insert(1, Point::new(5.0, 1.0));
        }
        assert_eq!(line.points.len(), 3);
        assert_eq!(line.points[1], Point::new(5.0, 1.0));

        {
            let points = line.path_points_mut().unwrap();
            assert_eq!(points.remove(1), Some(Point::new(5.0, 1.0)));
        }
        assert_eq!(line.points.len(), 2);
        assert_eq!(line.points[1], Point::new(10.0, 0.0));
    }
}
