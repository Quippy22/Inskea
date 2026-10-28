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
///
/// The names read backwards on purpose: files and the panel call the plain
/// line "Direct" and the elbow "Straight". Keep the serde names pinned so old
/// `.skea` files still resolve to the route they were drawn with.
#[derive(Clone, Copy, Debug, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum ConnectorVariant {
    /// A direct line between the two endpoints; shown as "Direct".
    #[default]
    #[serde(rename = "Straight")]
    Direct,
    /// A smoothly bowed curve between the two endpoints.
    Curved,
    /// An orthogonal elbow that runs around the shapes; shown as "Straight".
    #[serde(rename = "Direct")]
    Straight,
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
    /// Manual curvature, as a signed fraction of the chord. Positive bows to
    /// the left of the start → end direction. `None` uses the route's default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curve_bend: Option<f64>,
    /// Where the elbow's middle section sits between the endpoints, as a
    /// fraction of the dominant axis. `None` centres it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elbow_split: Option<f64>,
}

/// Which route handle the pointer grabbed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteHandle {
    /// The point that changes a curved connector's bow.
    Curve,
    /// A section midpoint that slides an elbow connector's bend.
    Elbow,
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
            curve_bend: None,
            elbow_split: None,
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
            curve_bend: None,
            elbow_split: None,
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

    /// Outward direction of a connection point, derived from its 3×3 grid
    /// position. The centre has no direction.
    fn connection_normal(binding: ConnectionEnd) -> Option<Point> {
        let x = (binding.index() % 3) as f64 - 1.0;
        let y = (binding.index() / 3) as f64 - 1.0;
        let len = x.hypot(y);
        (len > 0.0).then(|| Point::new(x / len, y / len))
    }

    /// Which side a curve bows to when neither connection point dictates one.
    /// The arc leans vertically toward the target: a shape below bows the route
    /// down, one above bows it up. Level routes pick the upper side, so a shape
    /// crossing the source's height flips the arc instead of keeping the same
    /// orientation.
    fn fallback_curve_side(dx: f64, dy: f64) -> f64 {
        if dy > 1e-9 {
            1.0
        } else if dy < -1e-9 || dx >= 0.0 {
            -1.0
        } else {
            1.0
        }
    }

    /// Signed control offset of the curve, as a fraction of the chord length.
    /// An explicit drag wins; otherwise the side follows the connection points
    /// (both on top bows above, both on the right bows right, and so on) and
    /// falls back to the direction the target sits in.
    fn curve_fraction(&self) -> f64 {
        let (Some(a), Some(b)) = (self.first(), self.last()) else {
            return 0.0;
        };
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len = dx.hypot(dy);
        if len <= 1e-9 {
            return 0.0;
        }
        if let Some(fraction) = self.curve_bend {
            return fraction;
        }
        let (nx, ny) = (-dy / len, dx / len);
        let normal_projection = match (
            self.start_binding.and_then(Self::connection_normal),
            self.end_binding.and_then(Self::connection_normal),
        ) {
            (Some(start), Some(end)) => Some((start.x + end.x) * nx + (start.y + end.y) * ny),
            (Some(start), None) => Some(start.x * nx + start.y * ny),
            (None, Some(end)) => Some(end.x * nx + end.y * ny),
            (None, None) => None,
        };
        let side = match normal_projection {
            Some(projection) if projection.abs() > 0.15 => projection.signum(),
            _ => Self::fallback_curve_side(dx, dy),
        };
        // Keep short connectors visibly bowed and long ones from ballooning.
        side * (len * 0.24).clamp(18.0, 110.0) / len
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
        let mid = Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
        if len <= 1e-9 {
            return Some(mid);
        }
        let offset = self.curve_fraction() * len;
        let (nx, ny) = (-dy / len, dx / len);
        Some(Point::new(mid.x + nx * offset, mid.y + ny * offset))
    }

    /// Point halfway along a plain two-point curve, where the curvature handle
    /// sits. Connectors with interior points expose those dots instead.
    pub fn curve_handle(&self) -> Option<Point> {
        if self.variant != ConnectorVariant::Curved || self.points.len() != 2 {
            return None;
        }
        let (a, b) = (self.first()?, self.last()?);
        let control = self.curve_control()?;
        Some(Point::new(
            (a.x + 2.0 * control.x + b.x) / 4.0,
            (a.y + 2.0 * control.y + b.y) / 4.0,
        ))
    }

    /// Set the bow so the curve's halfway point lands on `point`.
    pub fn set_curve_bend(&mut self, point: Point) {
        let (Some(a), Some(b)) = (self.first(), self.last()) else {
            return;
        };
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len = dx.hypot(dy);
        if len <= 1e-9 {
            return;
        }
        let mid = Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
        let (nx, ny) = (-dy / len, dx / len);
        let away = (point.x - mid.x) * nx + (point.y - mid.y) * ny;
        self.curve_bend = Some((2.0 * away / len).clamp(-1.5, 1.5));
    }

    /// Midpoints of the sections the elbow actually draws.
    pub fn elbow_handles(&self) -> Vec<Point> {
        if self.variant != ConnectorVariant::Straight {
            return Vec::new();
        }
        let route = self.elbow_points();
        if route.len() <= 2 {
            return Vec::new();
        }
        route
            .windows(2)
            .filter(|pair| (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y) > 1e-6)
            .map(|pair| Point::new((pair[0].x + pair[1].x) / 2.0, (pair[0].y + pair[1].y) / 2.0))
            .collect()
    }

    /// Slide the elbow's middle section so the grabbed `point` sets its
    /// position along the route's dominant axis.
    pub fn set_elbow_split(&mut self, point: Point) {
        let (Some(a), Some(b)) = (self.first(), self.last()) else {
            return;
        };
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let fraction = if dx.abs() >= dy.abs() {
            if dx.abs() <= 1e-9 {
                return;
            }
            (point.x - a.x) / dx
        } else {
            if dy.abs() <= 1e-9 {
                return;
            }
            (point.y - a.y) / dy
        };
        self.elbow_split = Some(fraction.clamp(0.08, 0.92));
    }

    /// Handles the overlay shows for this variant, paired with what dragging
    /// them changes.
    pub fn route_handles(&self) -> Vec<(RouteHandle, Point)> {
        match self.variant {
            ConnectorVariant::Curved => self
                .curve_handle()
                .map(|point| (RouteHandle::Curve, point))
                .into_iter()
                .collect(),
            ConnectorVariant::Straight => self
                .elbow_handles()
                .into_iter()
                .map(|point| (RouteHandle::Elbow, point))
                .collect(),
            ConnectorVariant::Direct => Vec::new(),
        }
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
            let fraction = self.elbow_split.unwrap_or(0.5).clamp(0.08, 0.92);
            let mx = a.x + (b.x - a.x) * fraction;
            vec![a, Point::new(mx, a.y), Point::new(mx, b.y), b]
        } else {
            let fraction = self.elbow_split.unwrap_or(0.5).clamp(0.08, 0.92);
            let my = a.y + (b.y - a.y) * fraction;
            vec![a, Point::new(a.x, my), Point::new(b.x, my), b]
        }
    }

    /// A polyline approximation of the rendered route.
    pub fn route_points(&self) -> Vec<Point> {
        match self.variant {
            ConnectorVariant::Direct => self.points.to_vec(),
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
            ConnectorVariant::Straight => self.elbow_points(),
        }
    }

    /// SVG path data for this connector.
    pub fn path_d(&self) -> String {
        match self.variant {
            ConnectorVariant::Direct => path_d(&self.points, CurveMode::Straight),
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
            ConnectorVariant::Straight => {
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
        Connector::new(a, b, Color::new(Color::WHITE), ConnectorVariant::default())
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

    /// The panel calls the plain line "Direct" and the elbow "Straight", but
    /// older files stored those names for the opposite routes. Both spellings
    /// must keep resolving to the route they were saved with.
    #[test]
    fn legacy_variant_names_keep_their_route() {
        let line: ConnectorVariant = serde_json::from_str("\"Straight\"").unwrap();
        let elbow: ConnectorVariant = serde_json::from_str("\"Direct\"").unwrap();
        assert_eq!(line, ConnectorVariant::Direct);
        assert_eq!(elbow, ConnectorVariant::Straight);
        assert_eq!(serde_json::to_string(&line).unwrap(), "\"Straight\"");
        assert_eq!(serde_json::to_string(&elbow).unwrap(), "\"Direct\"");

        let connector = Connector {
            variant: line,
            ..connector(Point::new(0.0, 0.0), Point::new(100.0, 100.0))
        };
        assert_eq!(connector.path_d().matches(" L").count(), 1);
    }

    #[test]
    fn elbow_route_has_two_elbows() {
        let connector = Connector {
            variant: ConnectorVariant::Straight,
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

    fn curved(a: Point, b: Point) -> Connector {
        Connector {
            variant: ConnectorVariant::Curved,
            ..connector(a, b)
        }
    }

    /// Signed distance from the chord to the curve's halfway point, positive on
    /// the same side as the chord's left normal.
    fn bow_side(connector: &Connector) -> f64 {
        let a = connector.first().unwrap();
        let b = connector.last().unwrap();
        let handle = connector.curve_handle().unwrap();
        let len = (b.x - a.x).hypot(b.y - a.y);
        let (nx, ny) = (-(b.y - a.y) / len, (b.x - a.x) / len);
        let (mx, my) = ((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
        (handle.x - mx) * nx + (handle.y - my) * ny
    }

    #[test]
    fn curve_leans_to_the_side_the_target_sits_on() {
        let level = curved(Point::new(0.0, 0.0), Point::new(400.0, 0.0));
        let below = curved(Point::new(0.0, 0.0), Point::new(60.0, 200.0));
        let above = curved(Point::new(0.0, 0.0), Point::new(60.0, -200.0));
        assert!(bow_side(&level) < -10.0, "a level route bows up");
        assert!(
            bow_side(&below) > 10.0,
            "a target below should sag the arc down"
        );
        assert!(
            bow_side(&above) < -10.0,
            "a target above should lift the arc up"
        );
    }

    #[test]
    fn curve_bows_toward_the_shared_connection_side() {
        let mut connector = curved(Point::new(0.0, 0.0), Point::new(100.0, 0.0));
        connector.start_binding = Some(ConnectionEnd::new(1, 1));
        connector.end_binding = Some(ConnectionEnd::new(2, 1));
        assert!(connector.curve_handle().unwrap().y < 0.0);

        connector.start_binding = Some(ConnectionEnd::new(1, 7));
        connector.end_binding = Some(ConnectionEnd::new(2, 7));
        assert!(connector.curve_handle().unwrap().y > 0.0);
    }

    #[test]
    fn dragging_the_curve_handle_reverses_the_bow() {
        let mut connector = curved(Point::new(0.0, 0.0), Point::new(100.0, 0.0));
        assert!(connector.curve_handle().unwrap().y < 0.0);

        connector.set_curve_bend(Point::new(50.0, 30.0));
        let handle = connector.curve_handle().unwrap();
        assert!((handle.x - 50.0).abs() < 1e-9);
        assert!((handle.y - 30.0).abs() < 1e-9);
        assert_eq!(connector.curve_bend, Some(0.6));
        assert!(connector.path_d().contains('Q'));
    }

    #[test]
    fn elbow_handles_sit_on_every_section() {
        let connector = Connector {
            variant: ConnectorVariant::Straight,
            ..connector(Point::new(0.0, 0.0), Point::new(100.0, 100.0))
        };
        let handles = connector.elbow_handles();
        assert_eq!(
            handles,
            vec![
                Point::new(25.0, 0.0),
                Point::new(50.0, 50.0),
                Point::new(75.0, 100.0),
            ]
        );
    }

    #[test]
    fn dragging_an_elbow_handle_slides_the_bend() {
        let mut connector = Connector {
            variant: ConnectorVariant::Straight,
            ..connector(Point::new(0.0, 0.0), Point::new(100.0, 100.0))
        };
        connector.set_elbow_split(Point::new(25.0, 80.0));
        let route = connector.route_points();
        assert_eq!(route[1], Point::new(25.0, 0.0));
        assert_eq!(route[2], Point::new(25.0, 100.0));
        assert_eq!(connector.elbow_handles().len(), 3);
    }

    #[test]
    fn direct_routes_offer_no_extra_handles() {
        let mut connector = connector(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
        assert!(connector.route_handles().is_empty());
        connector.variant = ConnectorVariant::Curved;
        assert_eq!(connector.route_handles().len(), 1);
        connector.variant = ConnectorVariant::Straight;
        assert_eq!(connector.route_handles().len(), 3);
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
