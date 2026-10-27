pub mod connector;
pub mod ellipse;
pub mod freehand;
pub mod image;
pub(crate) mod line;
pub mod path;
pub mod rect;
pub(crate) mod text;
pub(crate) mod utils;

pub use connector::{ConnectionEnd, Connector, ConnectorVariant};
pub use ellipse::Ellipse;
pub use freehand::Freehand;
pub use image::Image;
#[allow(unused_imports)]
pub use line::{Line, LineStyle};
pub use rect::Rectangle;
pub use text::{Text, TextAlign};

use crate::model::{Color, Point, Rect};
pub use path::CurveMode;

/// Stroke dash style.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize, Default)]
#[repr(u8)]
pub enum StrokeStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

impl StrokeStyle {
    pub fn stroke_dasharray(&self) -> &'static str {
        match self {
            StrokeStyle::Solid => "none",
            StrokeStyle::Dashed => "6 3",
            StrokeStyle::Dotted => "2 4",
        }
    }
}

/// Edge corner style for shapes.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize, Default)]
#[repr(u8)]
pub enum EdgeStyle {
    #[default]
    Sharp,
    Rounded,
}

impl EdgeStyle {
    pub fn stroke_linejoin(&self) -> &'static str {
        match self {
            EdgeStyle::Sharp => "miter",
            EdgeStyle::Rounded => "round",
        }
    }
}

/// Shared appearance properties for every element type.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[repr(C)]
pub struct ElementStyle {
    pub stroke_color: Color,
    pub fill_color: Option<Color>,
    pub stroke_width: f64,
    pub font_size: f64,
    pub stroke_style: StrokeStyle,
    pub edge_style: EdgeStyle,
    #[serde(default)]
    pub roundness: f64,
    #[serde(default = "default_opacity")]
    pub opacity: f64,
}

fn default_opacity() -> f64 {
    1.0
}

impl Default for ElementStyle {
    fn default() -> Self {
        Self {
            stroke_color: Color::default(),
            fill_color: None,
            stroke_width: 2.0,
            font_size: 18.0,
            stroke_style: StrokeStyle::default(),
            edge_style: EdgeStyle::default(),
            roundness: 6.0,
            opacity: 1.0,
        }
    }
}

/// Categorisation of an element for the styling panel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StylingKind {
    Rectangle,
    Ellipse,
    Line,
    Arrow,
    Connector,
    Text,
    Freehand,
    Image,
}

/// Unique identifier for an element in the scene.
pub type ElementId = u64;

/// Common data shared by every element type: position, size, and appearance.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[repr(C)]
pub struct ElementData {
    /// Unique identifier for this element, assigned when added to a Scene.
    pub id: ElementId,
    /// World-space position of the element's anchor (top-left for rect/ellipse/text).
    pub world_point: Point,
    /// Width of the element's bounding box in world-space.
    pub width: f64,
    /// Height of the element's bounding box in world-space.
    pub height: f64,
    /// Clockwise rotation in radians around the element's center.
    pub rotation: f64,
    /// Shared appearance properties (stroke, fill, etc.).
    pub style: ElementStyle,
}

impl ElementData {
    /// Creates an `ElementData` with defaults.
    pub fn new(id: ElementId) -> Self {
        Self {
            id,
            world_point: Point::default(),
            width: 100.0,
            height: 100.0,
            rotation: 0.0,
            style: ElementStyle::default(),
        }
    }
}

/// Every drawable shape on the canvas.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Element {
    /// A [`Rectangle`] shape.
    Rectangle(Rectangle),
    /// An [`Ellipse`] (oval) shape.
    Ellipse(Ellipse),
    /// A [`Line`] segment (optionally with an arrowhead).
    Line(Line),
    /// A [`Connector`] that can stay attached to two shapes.
    Connector(Connector),
    /// A [`Text`] label with word-wrapping.
    Text(Text),
    /// A [`Freehand`] stroke made of sampled points.
    Freehand(Freehand),
    /// An [`Image`] imported from a file and embedded as a data URL.
    Image(Image),
}

// ── Element dispatch methods ───────────────────────────────────────────

impl Element {
    /// Returns the unique identifier of this element.
    pub fn id(&self) -> ElementId {
        self.data().id
    }

    /// Shared reference to the element's common data.
    pub fn data(&self) -> &ElementData {
        match self {
            Element::Rectangle(e) => &e.data,
            Element::Ellipse(e) => &e.data,
            Element::Line(e) => &e.data,
            Element::Connector(e) => &e.data,
            Element::Text(e) => &e.data,
            Element::Freehand(e) => &e.data,
            Element::Image(e) => &e.data,
        }
    }

    /// Mutable reference to the element's common data.
    pub fn data_mut(&mut self) -> &mut ElementData {
        match self {
            Element::Rectangle(e) => &mut e.data,
            Element::Ellipse(e) => &mut e.data,
            Element::Line(e) => &mut e.data,
            Element::Connector(e) => &mut e.data,
            Element::Text(e) => &mut e.data,
            Element::Freehand(e) => &mut e.data,
            Element::Image(e) => &mut e.data,
        }
    }

    /// Translate every coordinate, including connector endpoints held by a
    /// shape.
    ///
    /// `Offset::offset` deliberately leaves bound endpoints where they are
    /// (they follow their shape instead), which is wrong when an element moves
    /// between scene chunks: the chunk translation has to apply to the whole
    /// geometry or the stored copy drifts by one chunk.
    pub fn translate_storage(&mut self, dx: f64, dy: f64) {
        match self {
            Element::Connector(e) => e.translate_all(dx, dy),
            other => other.offset(dx, dy),
        }
    }

    /// The kind of styling panel to show for this element.
    pub fn styling_kind(&self) -> StylingKind {
        match self {
            Element::Rectangle(_) => StylingKind::Rectangle,
            Element::Ellipse(_) => StylingKind::Ellipse,
            Element::Line(l) => {
                if l.line_style.has_end_arrowhead || l.line_style.has_start_arrowhead {
                    StylingKind::Arrow
                } else {
                    StylingKind::Line
                }
            }
            Element::Connector(_) => StylingKind::Connector,
            Element::Text(_) => StylingKind::Text,
            Element::Freehand(_) => StylingKind::Freehand,
            Element::Image(_) => StylingKind::Image,
        }
    }
}

// ── Into<Element> conversions ─────────────────────────────────────────

macro_rules! impl_into_element {
    ($variant:ident) => {
        impl From<$variant> for Element {
            fn from(e: $variant) -> Self {
                Element::$variant(e)
            }
        }
    };
}

impl_into_element!(Rectangle);
impl_into_element!(Ellipse);
impl_into_element!(Line);
impl_into_element!(Connector);
impl_into_element!(Text);
impl_into_element!(Freehand);
impl_into_element!(Image);

// ── Trait definitions ──────────────────────────────────────────────────

/// SVG rendering for an element type.
pub trait Render {
    /// Produce an SVG view for this element, scaled by `zoom`.
    fn render(&self, zoom: f64) -> leptos::View;
}

/// Hit-testing: check if a world-space point intersects the element within a margin.
pub trait HitTest {
    /// Returns `true` if `point` lies on/near this element within `margin` world-space units.
    fn hit_test(&self, point: Point, margin: f64) -> bool;
}

/// Bounding box computation in world-space: (x, y, width, height).
pub trait Bounds {
    fn bounds(&self) -> Rect;
}

/// Offset (translate) the element by a delta.
pub trait Offset {
    /// Move the element by `dx` world-space units horizontally and `dy` vertically.
    fn offset(&mut self, dx: f64, dy: f64);
}

/// Rotate the element around a pivot point.
///
/// # Conventions
///
/// There are two coexisting rotation strategies in this codebase,
/// and every implementor of this trait follows exactly one of them:
///
/// **In-place rotation** (used by `Rectangle`, `Ellipse`, `Text`):
/// Accumulate `delta` into `data.rotation` and never transform the
/// element's position/size fields. The render method wraps the shape
/// in an SVG `transform="rotate(…)"` centred on the element's own
/// bounding-box centre. `data.rotation` is the single source of truth
/// and is non-zero after any rotation drag.
///
/// **Point-based rotation** (used by `Line`, `Arrow`, `Freehand`):
/// Apply the rotation matrix directly to every positional point
/// (endpoints for Line/Arrow, sample points for Freehand) around the
/// given pivot `(cx, cy)`. `data.rotation` is **never** touched and
/// stays at `0.0` — there is no single "shape transform" to accumulate
/// into, because the geometry is not a centred bounding box.
///
/// Both strategies produce the correct visual result. Code that reads
/// `data.rotation` to decide whether a selection box should be rotated
/// (e.g. `selection.rs`) must be aware of this split: it will correctly
/// detect rotation for in-place types but will always see `0.0` for
/// point-based types.
pub trait Rotate {
    /// Rotate by `delta` radians around the point (`cx`, `cy`).
    fn rotate_around(&mut self, point: Point, delta: f64);
}

/// Trait for element types that expose an editable path of points.
///
/// Used by the selection overlay to render draggable point handles and
/// ghost midpoint inserters for `Line` and `Arrow`.
///
/// **Note:** `Freehand` does **not** implement this trait — a freehand
/// stroke can have hundreds of sampled points and would be unusable with
/// per-dot handles. Freehand keeps its current render-only behavior.
pub trait PathPoints {
    /// Shared reference to this element's path points, if it has them.
    fn path_points(&self) -> Option<&[Point]> {
        None
    }
    /// Mutable reference to this element's path points, if it has them.
    fn path_points_mut(&mut self) -> Option<&mut dyn EditablePathPoints> {
        None
    }
    /// How the path is rendered (curve mode).
    fn curve_mode(&self) -> CurveMode {
        CurveMode::Straight
    }
    /// Whether the overlay may offer ghost handles for inserting new bends.
    fn editable_midpoints(&self) -> bool {
        true
    }
}

/// Path storage operations used by node editing. A line is normally just two
/// points, so it stores them as an exact boxed slice and rebuilds it on the
/// rare insert/remove.
pub trait EditablePathPoints {
    fn len(&self) -> usize;
    fn get_mut(&mut self, index: usize) -> Option<&mut Point>;
    fn insert(&mut self, index: usize, point: Point);
    fn remove(&mut self, index: usize) -> Option<Point>;
}

impl EditablePathPoints for Box<[Point]> {
    fn len(&self) -> usize {
        self.as_ref().len()
    }
    fn get_mut(&mut self, index: usize) -> Option<&mut Point> {
        self.as_mut().get_mut(index)
    }
    fn insert(&mut self, index: usize, point: Point) {
        if index <= self.len() {
            let mut points = self.to_vec();
            points.insert(index, point);
            *self = points.into_boxed_slice();
        }
    }
    fn remove(&mut self, index: usize) -> Option<Point> {
        if index >= self.len() {
            return None;
        }
        let mut points = self.to_vec();
        let removed = points.remove(index);
        *self = points.into_boxed_slice();
        Some(removed)
    }
}

impl PathPoints for Line {
    fn path_points(&self) -> Option<&[Point]> {
        Some(self.points.as_ref())
    }
    fn path_points_mut(&mut self) -> Option<&mut dyn EditablePathPoints> {
        Some(&mut self.points)
    }
    fn curve_mode(&self) -> CurveMode {
        self.line_style.curve_mode
    }
}

impl PathPoints for Element {
    fn path_points(&self) -> Option<&[Point]> {
        match self {
            Element::Line(e) => Some(e.points.as_ref()),
            Element::Connector(e) => Some(e.points.as_ref()),
            _ => None,
        }
    }
    fn path_points_mut(&mut self) -> Option<&mut dyn EditablePathPoints> {
        match self {
            Element::Line(e) => Some(&mut e.points),
            Element::Connector(e) => Some(&mut e.points),
            _ => None,
        }
    }
    fn curve_mode(&self) -> CurveMode {
        match self {
            Element::Line(e) => e.line_style.curve_mode,
            _ => CurveMode::Straight,
        }
    }
    fn editable_midpoints(&self) -> bool {
        !matches!(self, Element::Connector(_))
    }
}

use crate::model::resize::ResizeContext;

/// Resize the element using the given context.
pub trait Resize {
    /// Mutate this element's geometry based on `ctx` (drag deltas, handle index, etc.).
    fn resize(&mut self, ctx: &ResizeContext);
}

/// Construct an element from a mouse-drag operation (anchor → current position).
pub trait FromDrag: Sized {
    /// Create a new element of this type given the drag start and current world-space points.
    fn from_drag(anchor: Point, current: Point, color: Color, shift: bool) -> Self;
}

/// Update an element while it is being drawn (e.g. freehand adding points).
pub trait UpdateDrag {
    /// Adjust the element in response to a mouse move during the drag that
    /// created it. `zoom` is the viewport zoom, so screen-space sampling
    /// density (freehand) stays constant at every zoom level.
    fn update_drag(&mut self, current: Point, anchor: Point, shift: bool, zoom: f64);
}

// ── Blanket trait implementations on Element ───────────────────────────

macro_rules! dispatch_element {
    ($trait:ident, $method:ident(&self $(, $arg:ident: $ty:ty)*) -> $ret:ty) => {
        impl $trait for Element {
            fn $method(&self, $($arg: $ty),*) -> $ret {
                match self {
                    Element::Rectangle(e) => e.$method($($arg),*),
                    Element::Ellipse(e) => e.$method($($arg),*),
                    Element::Line(e) => e.$method($($arg),*),
                    Element::Connector(e) => e.$method($($arg),*),
                    Element::Text(e) => e.$method($($arg),*),
                    Element::Freehand(e) => e.$method($($arg),*),
                    Element::Image(e) => e.$method($($arg),*),
                }
            }
        }
    };
    ($trait:ident, $method:ident(&mut self $(, $arg:ident: $ty:ty)*)) => {
        impl $trait for Element {
            fn $method(&mut self, $($arg: $ty),*) {
                match self {
                    Element::Rectangle(e) => e.$method($($arg),*),
                    Element::Ellipse(e) => e.$method($($arg),*),
                    Element::Line(e) => e.$method($($arg),*),
                    Element::Connector(e) => e.$method($($arg),*),
                    Element::Text(e) => e.$method($($arg),*),
                    Element::Freehand(e) => e.$method($($arg),*),
                    Element::Image(e) => e.$method($($arg),*),
                }
            }
        }
    };
}

dispatch_element!(Render, render(&self, zoom: f64) -> leptos::View);
dispatch_element!(HitTest, hit_test(&self, point: Point, margin: f64) -> bool);
dispatch_element!(Bounds, bounds(&self) -> Rect);
dispatch_element!(Offset, offset(&mut self, dx: f64, dy: f64));
dispatch_element!(Rotate, rotate_around(&mut self, point: Point, delta: f64));
dispatch_element!(Resize, resize(&mut self, ctx: &ResizeContext));
dispatch_element!(UpdateDrag, update_drag(&mut self, current: Point, anchor: Point, shift: bool, zoom: f64));

// ── Helpers ────────────────────────────────────────────────────────────

/// Snap an angle to the nearest `divisions`-th of a full turn.
pub(crate) fn snap_angle(angle: f64, divisions: f64) -> f64 {
    (angle / (std::f64::consts::TAU / divisions)).round() * (std::f64::consts::TAU / divisions)
}

#[cfg(test)]
mod tests {
    use super::StrokeStyle;

    #[test]
    fn dotted_strokes_leave_a_clear_gap() {
        assert_eq!(StrokeStyle::Dotted.stroke_dasharray(), "2 4");
    }
}
