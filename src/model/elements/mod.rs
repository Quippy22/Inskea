pub mod ellipse;
pub mod freehand;
pub(crate) mod line;
pub mod path;
pub mod rect;
pub(crate) mod text;
pub(crate) mod utils;

pub use ellipse::Ellipse;
pub use freehand::Freehand;
#[allow(unused_imports)]
pub use line::{Line, LineStyle};
pub use rect::Rectangle;
pub use text::Text;

use super::ShapeColor;
use crate::model::Point;
pub use path::CurveMode;

/// Stroke dash style.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum StrokeStyle {
    Solid,
    Dashed,
    Dotted,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self::Solid
    }
}

impl StrokeStyle {
    pub fn stroke_dasharray(&self) -> &'static str {
        match self {
            StrokeStyle::Solid => "none",
            StrokeStyle::Dashed => "6 3",
            StrokeStyle::Dotted => "2 2",
        }
    }
}

/// Edge corner style for shapes.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EdgeStyle {
    Sharp,
    Rounded,
}

impl Default for EdgeStyle {
    fn default() -> Self {
        Self::Sharp
    }
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
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ElementStyle {
    pub stroke_color: ShapeColor,
    pub fill_color: Option<ShapeColor>,
    pub stroke_width: f64,
    pub font_size: f64,
    pub stroke_style: StrokeStyle,
    pub edge_style: EdgeStyle,
    #[serde(default)]
    pub roundness: f64,
    #[serde(default = "default_opacity")]
    pub opacity: f64,
}

fn default_opacity() -> f64 { 1.0 }

impl Default for ElementStyle {
    fn default() -> Self {
        Self {
            stroke_color: ShapeColor::default(),
            fill_color: None,
            stroke_width: 2.0,
            font_size: 24.0,
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
    Text,
    Freehand,
}

/// Unique identifier for an element in the scene.
pub type ElementId = u64;

/// Common data shared by every element type: position, size, and appearance.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
    /// A [`Text`] label with word-wrapping.
    Text(Text),
    /// A [`Freehand`] stroke made of sampled points.
    Freehand(Freehand),
}

// ── Element dispatch methods ───────────────────────────────────────────

impl Element {
    /// Returns the unique identifier of this element.
    pub fn id(&self) -> ElementId {
        match self {
            Element::Rectangle(e) => e.data.id,
            Element::Ellipse(e) => e.data.id,
            Element::Line(e) => e.data.id,
            Element::Text(e) => e.data.id,
            Element::Freehand(e) => e.data.id,
        }
    }

    /// Shared reference to the element's common data.
    pub fn data(&self) -> &ElementData {
        match self {
            Element::Rectangle(e) => &e.data,
            Element::Ellipse(e) => &e.data,
            Element::Line(e) => &e.data,
            Element::Text(e) => &e.data,
            Element::Freehand(e) => &e.data,
        }
    }

    /// Mutable reference to the element's common data.
    pub fn data_mut(&mut self) -> &mut ElementData {
        match self {
            Element::Rectangle(e) => &mut e.data,
            Element::Ellipse(e) => &mut e.data,
            Element::Line(e) => &mut e.data,
            Element::Text(e) => &mut e.data,
            Element::Freehand(e) => &mut e.data,
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
            Element::Text(_) => StylingKind::Text,
            Element::Freehand(_) => StylingKind::Freehand,
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
impl_into_element!(Text);
impl_into_element!(Freehand);

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
    /// Returns (x, y, width, height) of the axis-aligned bounding box.
    fn bounds(&self) -> (f64, f64, f64, f64);
}

/// Offset (translate) the element by a delta.
pub trait Offset {
    /// Move the element by `dx` world-space units horizontally and `dy` vertically.
    fn offset(&mut self, dx: f64, dy: f64);
}

/// Snap all position data to a grid of the given spacing.
pub trait SnapToGrid {
    /// Round position values to the nearest multiple of `grid`.
    fn snap_to_grid(&mut self, grid: f64);
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
    fn path_points(&self) -> Option<&Vec<Point>> {
        None
    }
    /// Mutable reference to this element's path points, if it has them.
    fn path_points_mut(&mut self) -> Option<&mut Vec<Point>> {
        None
    }
    /// How the path is rendered (curve mode).
    fn curve_mode(&self) -> CurveMode {
        CurveMode::Straight
    }
    /// Set the curve mode.
    fn set_curve_mode(&mut self, _mode: CurveMode) {}
}

impl PathPoints for Line {
    fn path_points(&self) -> Option<&Vec<Point>> {
        Some(&self.points)
    }
    fn path_points_mut(&mut self) -> Option<&mut Vec<Point>> {
        Some(&mut self.points)
    }
    fn curve_mode(&self) -> CurveMode {
        self.line_style.curve_mode
    }
    fn set_curve_mode(&mut self, mode: CurveMode) {
        self.line_style.curve_mode = mode;
    }
}

impl PathPoints for Element {
    fn path_points(&self) -> Option<&Vec<Point>> {
        match self {
            Element::Line(e) => Some(&e.points),
            _ => None,
        }
    }
    fn path_points_mut(&mut self) -> Option<&mut Vec<Point>> {
        match self {
            Element::Line(e) => Some(&mut e.points),
            _ => None,
        }
    }
    fn curve_mode(&self) -> CurveMode {
        match self {
            Element::Line(e) => e.line_style.curve_mode,
            _ => CurveMode::Straight,
        }
    }
    fn set_curve_mode(&mut self, mode: CurveMode) {
        if let Element::Line(e) = self { e.line_style.curve_mode = mode }
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
    fn from_drag(anchor: Point, current: Point, color: ShapeColor, shift: bool) -> Self;
}

/// Update an element while it is being drawn (e.g. freehand adding points).
pub trait UpdateDrag {
    /// Adjust the element in response to a mouse move during the drag that created it.
    fn update_drag(&mut self, current: Point, anchor: Point, shift: bool);
}

// ── Blanket trait implementations on Element ───────────────────────────

impl Render for Element {
    fn render(&self, zoom: f64) -> leptos::View {
        match self {
            Element::Rectangle(e) => e.render(zoom),
            Element::Ellipse(e) => e.render(zoom),
            Element::Line(e) => e.render(zoom),
            Element::Text(e) => e.render(zoom),
            Element::Freehand(e) => e.render(zoom),
        }
    }
}

impl HitTest for Element {
    fn hit_test(&self, point: Point, margin: f64) -> bool {
        match self {
            Element::Rectangle(e) => e.hit_test(point, margin),
            Element::Ellipse(e) => e.hit_test(point, margin),
            Element::Line(e) => e.hit_test(point, margin),
            Element::Text(e) => e.hit_test(point, margin),
            Element::Freehand(e) => e.hit_test(point, margin),
        }
    }
}

impl Bounds for Element {
    fn bounds(&self) -> (f64, f64, f64, f64) {
        match self {
            Element::Rectangle(e) => e.bounds(),
            Element::Ellipse(e) => e.bounds(),
            Element::Line(e) => e.bounds(),
            Element::Text(e) => e.bounds(),
            Element::Freehand(e) => e.bounds(),
        }
    }
}

impl Offset for Element {
    fn offset(&mut self, dx: f64, dy: f64) {
        match self {
            Element::Rectangle(e) => e.offset(dx, dy),
            Element::Ellipse(e) => e.offset(dx, dy),
            Element::Line(e) => e.offset(dx, dy),
            Element::Text(e) => e.offset(dx, dy),
            Element::Freehand(e) => e.offset(dx, dy),
        }
    }
}

impl SnapToGrid for Element {
    fn snap_to_grid(&mut self, grid: f64) {
        match self {
            Element::Rectangle(e) => e.snap_to_grid(grid),
            Element::Ellipse(e) => e.snap_to_grid(grid),
            Element::Line(e) => e.snap_to_grid(grid),
            Element::Text(e) => e.snap_to_grid(grid),
            Element::Freehand(e) => e.snap_to_grid(grid),
        }
    }
}

impl Rotate for Element {
    fn rotate_around(&mut self, point: Point, delta: f64) {
        match self {
            Element::Rectangle(e) => e.rotate_around(point, delta),
            Element::Ellipse(e) => e.rotate_around(point, delta),
            Element::Line(e) => e.rotate_around(point, delta),
            Element::Text(e) => e.rotate_around(point, delta),
            Element::Freehand(e) => e.rotate_around(point, delta),
        }
    }
}

impl Resize for Element {
    fn resize(&mut self, ctx: &ResizeContext) {
        match self {
            Element::Rectangle(e) => e.resize(ctx),
            Element::Ellipse(e) => e.resize(ctx),
            Element::Line(e) => e.resize(ctx),
            Element::Text(e) => e.resize(ctx),
            Element::Freehand(e) => e.resize(ctx),
        }
    }
}

impl UpdateDrag for Element {
    fn update_drag(&mut self, current: Point, anchor: Point, shift: bool) {
        match self {
            Element::Rectangle(e) => e.update_drag(current, anchor, shift),
            Element::Ellipse(e) => e.update_drag(current, anchor, shift),
            Element::Line(e) => e.update_drag(current, anchor, shift),
            Element::Text(e) => e.update_drag(current, anchor, shift),
            Element::Freehand(e) => e.update_drag(current, anchor, shift),
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Snap an angle to the nearest `divisions`-th of a full turn.
pub(crate) fn snap_angle(angle: f64, divisions: f64) -> f64 {
    (angle / (std::f64::consts::TAU / divisions)).round() * (std::f64::consts::TAU / divisions)
}
