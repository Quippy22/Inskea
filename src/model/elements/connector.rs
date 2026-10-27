use crate::model::elements::path::{
    bounds_of_points, eval_cubic_bezier, hit_test_path, path_d, scale_points, segment_cubic_bezier,
    CurveMode,
};
use crate::model::elements::utils::{arrowhead_polyline, line_endpoints};
use crate::model::resize::ResizeContext;
use crate::model::*;
use leptos::IntoView;

/// How far (in screen pixels) an endpoint is attracted to a connection point.
pub const SNAP_SCREEN_PX: f64 = 14.0;

/// How many connection points a shape exposes: a 3×3 grid of corners, edge
/// midpoints, and the centre.
pub const CONNECTION_POINT_COUNT: usize = 9;

/// A point on a shape an endpoint can be attached to.
///
/// The index is stable for the shape's lifetime: it always addresses the same
/// position in the 3×3 grid, even when the shape is moved or resized.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConnectionEnd {
    pub element: ElementId,
    pub point: u8,
}

impl ConnectionEnd {
    pub fn new(element: ElementId, point: usize) -> Self {
        Self {
            element,
            point: point as u8,
        }
    }

    pub fn index(self) -> usize {
        self.point as usize
    }
}

/// Routing style for a connector.
#[derive(Clone, Copy, Debug, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum ConnectorVariant {
    /// A direct line between the two endpoints.
    #[default]
    Straight,
    /// A smoothly bowed curve between the two endpoints.
    Curved,
    /// An orthogonal elbow that runs around the shapes.
    Direct,
}

/// A line or arrow that can stay attached to connection points on two shapes.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Connector {
    pub data: ElementData,
    /// Endpoints (and optional interior bends) in world space.
    pub points: Box<[Point]>,
    /// Stroke/arrowhead appearance, shared with lines.
    pub line_style: LineStyle,
    /// Routing style.
    #[serde(default)]
    pub variant: ConnectorVariant,
    /// Shape point held by the first endpoint.
    #[serde(default)]
    pub start_binding: Option<ConnectionEnd>,
    /// Shape point held by the last endpoint.
    #[serde(default)]
    pub end_binding: Option<ConnectionEnd>,
}

impl Connector {
    pub fn from_drag(anchor: Point, current: Point, color: Color, shift: bool) -> Self {
        let (a, b) = line_endpoints(anchor, current, shift);
        Self::new(a, b, color, ConnectorVariant::default())
    }

    pub fn new(a: Point, b: Point, color: Color, variant: ConnectorVariant) -> Self {
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
            variant,
            start_binding: None,
            end_binding: None,
        }
    }

    pub fn from_line(
        line: Line,
        variant: ConnectorVariant,
        start_binding: Option<ConnectionEnd>,
        end_binding: Option<ConnectionEnd>,
    ) -> Self {
        Self {
            data: line.data,
            points: line.points,
            line_style: line.line_style,
            variant,
            start_binding,
            end_binding,
        }
    }

    /// The point an endpoint is attached to, if it is still bound.
    pub fn binding(&self, start: bool) -> Option<ConnectionEnd> {
        if start {
            self.start_binding
        } else {
            self.end_binding
        }
    }

    pub fn set_binding(&mut self, start: bool, binding: Option<ConnectionEnd>) {
        if start {
            self.start_binding = binding;
        } else {
            self.end_binding = binding;
        }
    }

    /// Update the endpoint at `index`, detaching it unless a binding is given.
    pub fn set_endpoint(&mut self, index: usize, point: Point, binding: Option<ConnectionEnd>) {
        if index < self.points.len() {
            self.points[index] = point;
        }
        if index == 0 {
            self.start_binding = binding;
        } else if index + 1 == self.points.len() {
            self.end_binding = binding;
        }
    }

    fn first(&self) -> Option<Point> {
        self.points.first().copied()
    }

    fn last(&self) -> Option<Point> {
        self.points.last().copied()
    }

    /// Control point of the bowed curve. The bow is proportional to the
    /// endpoint distance so short connectors stay gentle and long ones do not
    /// collapse into straight lines.
    fn curve_control(&self) -> Option<Point> {
        let a = self.first()?;
        let b = self.last()?;
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let len = dx.hypot(dy);
        if len <= 1e-9 {
            return Some(Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0));
        }
        let offset = (len * 0.2).clamp(10.0, 60.0);
        let nx = -dy / len;
        let ny = dx / len;
        let mid = Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
        Some(Point::new(mid.x + nx * offset, mid.y + ny * offset))
    }

    /// Sample the curve for hit testing and bounds.
    fn sample_quadratic(&self, count: usize) -> Vec<Point> {
        let (Some(a), Some(b)) = (self.first(), self.last()) else {
            return Vec::new();
        };
        let control = self.curve_control().unwrap_or(a);
        (0..=count)
            .map(|i| {
                let t = i as f64 / count as f64;
                let mt = 1.0 - t;
                Point::new(
                    mt * mt * a.x + 2.0 * mt * t * control.x + t * t * b.x,
                    mt * mt * a.y + 2.0 * mt * t * control.y + t * t * b.y,
                )
            })
            .collect()
    }

    /// Orthogonal elbow route between the first and last point.
    fn elbow_points(&self) -> Vec<Point> {
        let (Some(a), Some(b)) = (self.first(), self.last()) else {
            return Vec::new();
        };
        if (a.x - b.x).abs() < 1e-9 || (a.y - b.y).abs() < 1e-9 {
            return vec![a, b];
        }
        if (b.x - a.x).abs() >= (b.y - a.y).abs() {
            let mx = (a.x + b.x) / 2.0;
            vec![a, Point::new(mx, a.y), Point::new(mx, b.y), b]
        } else {
            let my = (a.y + b.y) / 2.0;
            vec![a, Point::new(a.x, my), Point::new(b.x, my), b]
        }
    }

    /// A polyline approximation of the rendered route.
    pub fn route_points(&self) -> Vec<Point> {
        match self.variant {
            ConnectorVariant::Straight => self.points.to_vec(),
            ConnectorVariant::Curved => {
                if self.points.len() <= 2 {
                    self.sample_quadratic(20)
                } else {
                    // Extra bends follow the same Catmull-Rom curve the
                    // renderer draws.
                    let mut route = Vec::new();
                    for i in 0..self.points.len().saturating_sub(1) {
                        let Some((p0, p1, p2, p3)) = segment_cubic_bezier(&self.points, i) else {
                            continue;
                        };
                        for step in 0..8 {
                            route.push(eval_cubic_bezier(&p0, &p1, &p2, &p3, step as f64 / 8.0));
                        }
                    }
                    if let Some(last) = self.last() {
                        route.push(last);
                    }
                    route
                }
            }
            ConnectorVariant::Direct => self.elbow_points(),
        }
    }

    /// SVG path data for this connector.
    pub fn path_d(&self) -> String {
        match self.variant {
            ConnectorVariant::Straight => path_d(&self.points, CurveMode::Straight),
            ConnectorVariant::Curved if self.points.len() <= 2 => {
                let (Some(a), Some(b), Some(control)) =
                    (self.first(), self.last(), self.curve_control())
                else {
                    return String::new();
                };
                format!(
                    "M{} {} Q{} {} {} {}",
                    a.x, a.y, control.x, control.y, b.x, b.y
                )
            }
            ConnectorVariant::Curved => path_d(&self.points, CurveMode::Curved),
            ConnectorVariant::Direct => {
                let route = self.elbow_points();
                if route.is_empty() {
                    return String::new();
                }
                let mut d = format!("M{} {}", route[0].x, route[0].y);
                use std::fmt::Write;
                for p in &route[1..] {
                    let _ = write!(d, " L{} {}", p.x, p.y);
                }
                d
            }
        }
    }

    /// Arrowhead polylines for the start and end of the route.
    pub fn arrowheads(&self, sw: f64) -> (Option<String>, Option<String>) {
        let route = self.route_points();
        let start = self
            .line_style
            .has_start_arrowhead
            .then(|| {
                route
                    .get(1)
                    .zip(route.first())
                    .map(|(tail, tip)| arrowhead_polyline(tail, tip, sw))
            })
            .flatten();
        let end = self
            .line_style
            .has_end_arrowhead
            .then(|| {
                route
                    .len()
                    .checked_sub(2)
                    .and_then(|index| route.get(index))
                    .zip(route.last())
                    .map(|(tail, tip)| arrowhead_polyline(tail, tip, sw))
            })
            .flatten();
        (start, end)
    }

    /// Move every point, bound or not. Only used for chunk storage; see
    /// [`Element::translate_storage`].
    pub fn translate_all(&mut self, dx: f64, dy: f64) {
        self.data.world_point.offset(dx, dy);
        for point in self.points.iter_mut() {
            point.offset(dx, dy);
        }
    }
}

impl FromDrag for Connector {
    fn from_drag(anchor: Point, current: Point, color: Color, shift: bool) -> Self {
        let (a, b) = line_endpoints(anchor, current, shift);
        Connector::new(a, b, color, ConnectorVariant::default())
    }
}

impl UpdateDrag for Connector {
    fn update_drag(&mut self, current: Point, anchor: Point, shift: bool, _zoom: f64) {
        let (a, b) = line_endpoints(anchor, current, shift);
        if self.points.len() >= 2 {
            self.points[0] = a;
            self.points[1] = b;
        }
    }
}

impl Render for Connector {
    fn render(&self, _zoom: f64) -> leptos::View {
        let sw = self.data.style.stroke_width;
        let stroke = self.data.style.stroke_color.to_hex();
        let dash = self.data.style.stroke_style.stroke_dasharray();
        let linejoin = self.data.style.edge_style.stroke_linejoin();
        let linecap = match self.data.style.edge_style {
            super::EdgeStyle::Sharp => "butt",
            super::EdgeStyle::Rounded => "round",
        };
        let d = self.path_d();
        let opacity = self.data.style.opacity;
        let sw2 = sw;
        let (start_arrow, end_arrow) = self.arrowheads(sw2);

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
    }
}

impl HitTest for Connector {
    fn hit_test(&self, point: Point, margin: f64) -> bool {
        let route = self.route_points();
        hit_test_path(
            &route,
            CurveMode::Straight,
            (point.x, point.y),
            margin + self.data.style.stroke_width,
        )
    }
}

impl Bounds for Connector {
    fn bounds(&self) -> Rect {
        bounds_of_points(&self.route_points())
    }
}

impl Offset for Connector {
    fn offset(&mut self, dx: f64, dy: f64) {
        self.data.world_point.offset(dx, dy);
        let last = self.points.len().saturating_sub(1);
        for (index, point) in self.points.iter_mut().enumerate() {
            let bound = (index == 0 && self.start_binding.is_some())
                || (index == last && self.end_binding.is_some());
            if !bound {
                point.offset(dx, dy);
            }
        }
    }
}

impl Rotate for Connector {
    fn rotate_around(&mut self, pivot: Point, delta: f64) {
        let last = self.points.len().saturating_sub(1);
        for (index, point) in self.points.iter_mut().enumerate() {
            let bound = (index == 0 && self.start_binding.is_some())
                || (index == last && self.end_binding.is_some());
            if !bound {
                point.rotate_around(pivot, delta);
            }
        }
    }
}

impl Resize for Connector {
    fn resize(&mut self, ctx: &ResizeContext) {
        let orig = match ctx.orig {
            Element::Connector(connector) => connector.points.to_vec(),
            _ => self.points.to_vec(),
        };
        scale_points(&mut self.points, ctx, &orig);
        let last = self.points.len().saturating_sub(1);
        if self.start_binding.is_some() && !orig.is_empty() {
            self.points[0] = orig[0];
        }
        if self.end_binding.is_some() && last < orig.len() {
            self.points[last] = orig[last];
        }
    }
}

impl PathPoints for Connector {
    fn path_points(&self) -> Option<&[Point]> {
        Some(self.points.as_ref())
    }
    fn path_points_mut(&mut self) -> Option<&mut dyn EditablePathPoints> {
        Some(&mut self.points)
    }
    fn curve_mode(&self) -> CurveMode {
        CurveMode::Straight
    }
    fn editable_midpoints(&self) -> bool {
        false
    }
}

/// The 3×3 grid of attachable points on a shape.
///
/// Rectangles and images use the corners, edge midpoints, and centre of their
/// box. Ellipses use points on the outline at the same compass directions, so
/// a connector never floats away from the curve. Text uses its glyph box.
pub fn connection_points(element: &Element) -> Option<[Point; CONNECTION_POINT_COUNT]> {
    let bounds = match element {
        Element::Rectangle(_) | Element::Ellipse(_) | Element::Image(_) => element.bounds(),
        Element::Text(text) => text.visual_bounds(),
        Element::Line(_) | Element::Freehand(_) | Element::Connector(_) => return None,
    };
    let pivot = element.bounds().center();
    let rotation = element.data().rotation;
    let is_ellipse = matches!(element, Element::Ellipse(_));
    let center = bounds.center();
    let (rx, ry) = (bounds.w / 2.0, bounds.h / 2.0);
    let mut points = [Point::new(0.0, 0.0); CONNECTION_POINT_COUNT];

    for row in 0..3 {
        for col in 0..3 {
            let index = row * 3 + col;
            let fx = col as f64 / 2.0;
            let fy = row as f64 / 2.0;
            let mut point = if is_ellipse && index != 4 {
                let dx = fx * 2.0 - 1.0;
                let dy = fy * 2.0 - 1.0;
                let len = dx.hypot(dy);
                Point::new(center.x + rx * dx / len, center.y + ry * dy / len)
            } else {
                Point::new(bounds.x + bounds.w * fx, bounds.y + bounds.h * fy)
            };
            point.rotate_around(pivot, rotation);
            points[index] = point;
        }
    }
    Some(points)
}

/// Resolve one stored binding to a world-space point.
pub fn binding_point(element: &Element, binding: ConnectionEnd) -> Option<Point> {
    connection_points(element).and_then(|points| points.get(binding.index()).copied())
}

/// Nearest connection point within `threshold` world units.
pub fn nearest_connection_point(
    elements: &[Element],
    point: Point,
    threshold: f64,
    exclude: Option<ElementId>,
) -> Option<(Point, ConnectionEnd)> {
    let mut best: Option<(f64, Point, ConnectionEnd)> = None;
    for element in elements {
        let id = element.id();
        if exclude == Some(id) {
            continue;
        }
        let Some(points) = connection_points(element) else {
            continue;
        };
        for (index, candidate) in points.iter().enumerate() {
            let distance = (candidate.x - point.x).hypot(candidate.y - point.y);
            if distance > threshold {
                continue;
            }
            let closer = best
                .as_ref()
                .is_none_or(|(best_distance, _, _)| distance < *best_distance);
            if closer {
                best = Some((distance, *candidate, ConnectionEnd::new(id, index)));
            }
        }
    }
    best.map(|(_, point, end)| (point, end))
}

/// Find the shape whose connection grid is nearest to `point`, for the hover
/// overlay. Returns the shape's points and the candidate within `threshold`.
pub fn connection_guide(
    elements: &[Element],
    point: Point,
    threshold: f64,
) -> Option<(ElementId, [Point; CONNECTION_POINT_COUNT], Option<usize>)> {
    let mut best: Option<(f64, ElementId, usize)> = None;
    for element in elements {
        let Some(points) = connection_points(element) else {
            continue;
        };
        for (index, candidate) in points.iter().enumerate() {
            let distance = (candidate.x - point.x).hypot(candidate.y - point.y);
            if distance > threshold {
                continue;
            }
            let closer = best
                .as_ref()
                .is_none_or(|(best_distance, _, _)| distance < *best_distance);
            if closer {
                best = Some((distance, element.id(), index));
            }
        }
    }
    let (_, id, index) = best?;
    let element = elements.iter().find(|element| element.id() == id)?;
    let points = connection_points(element)?;
    let highlight = ((points[index].x - point.x).hypot(points[index].y - point.y) <= threshold)
        .then_some(index);
    Some((id, points, highlight))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::elements::ellipse::Ellipse;
    use crate::model::elements::rect::Rectangle;

    fn rect(id: ElementId, x: f64, y: f64, w: f64, h: f64) -> Element {
        Element::Rectangle(Rectangle {
            data: ElementData {
                id,
                world_point: Point::new(x, y),
                width: w,
                height: h,
                ..ElementData::new(id)
            },
        })
    }

    fn connector(a: Point, b: Point) -> Connector {
        Connector::new(a, b, Color::new(Color::WHITE), ConnectorVariant::Straight)
    }

    #[test]
    fn connection_grid_covers_corners_edges_and_centre() {
        let points = connection_points(&rect(1, 0.0, 0.0, 100.0, 50.0)).unwrap();
        assert_eq!(points[0], Point::new(0.0, 0.0));
        assert_eq!(points[1], Point::new(50.0, 0.0));
        assert_eq!(points[2], Point::new(100.0, 0.0));
        assert_eq!(points[3], Point::new(0.0, 25.0));
        assert_eq!(points[4], Point::new(50.0, 25.0));
        assert_eq!(points[8], Point::new(100.0, 50.0));
    }

    #[test]
    fn ellipse_points_sit_on_the_outline() {
        let element = Element::Ellipse(Ellipse {
            data: ElementData {
                id: 1,
                world_point: Point::new(0.0, 0.0),
                width: 200.0,
                height: 100.0,
                ..ElementData::new(1)
            },
        });
        let points = connection_points(&element).unwrap();
        // Diagonal point uses the ellipse's radius, not the box corner.
        assert!((points[8].x - (100.0 + 100.0 / 2f64.sqrt())).abs() < 1e-9);
        assert!((points[8].y - (50.0 + 50.0 / 2f64.sqrt())).abs() < 1e-9);
    }

    #[test]
    fn connection_points_follow_rotation() {
        let mut element = rect(1, 0.0, 0.0, 100.0, 100.0);
        element.data_mut().rotation = std::f64::consts::FRAC_PI_2;
        let points = connection_points(&element).unwrap();
        // The top-left corner swings around the centre to the top-right.
        assert!((points[0].x - 100.0).abs() < 1e-9);
        assert!((points[0].y - 0.0).abs() < 1e-9);
    }

    #[test]
    fn nearest_connection_prefers_the_closest_point() {
        let elements = vec![rect(1, 0.0, 0.0, 100.0, 100.0)];
        let (point, end) =
            nearest_connection_point(&elements, Point::new(48.0, 4.0), 10.0, None).unwrap();
        assert_eq!(end, ConnectionEnd::new(1, 1));
        assert_eq!(point, Point::new(50.0, 0.0));
    }

    #[test]
    fn direct_route_has_two_elbows() {
        let connector = Connector {
            variant: ConnectorVariant::Direct,
            ..connector(Point::new(0.0, 0.0), Point::new(100.0, 100.0))
        };
        assert_eq!(connector.route_points().len(), 4);
        assert_eq!(connector.path_d().matches(" L").count(), 3);
    }

    #[test]
    fn curved_route_bows_away_from_the_chord() {
        let connector = Connector {
            variant: ConnectorVariant::Curved,
            ..connector(Point::new(0.0, 0.0), Point::new(100.0, 0.0))
        };
        let route = connector.route_points();
        let mid = route[route.len() / 2];
        assert!(mid.y.abs() > 1.0, "curve should leave the straight chord");
    }

    #[test]
    fn dragged_bound_endpoint_stays_put() {
        let mut connector = connector(Point::new(10.0, 10.0), Point::new(100.0, 100.0));
        connector.start_binding = Some(ConnectionEnd::new(7, 1));
        connector.offset(50.0, 50.0);
        assert_eq!(connector.points[0], Point::new(10.0, 10.0));
        assert_eq!(connector.points[1], Point::new(150.0, 150.0));
    }
}
