pub mod rect;
pub mod ellipse;
pub(crate) mod line;
pub mod arrow;
pub(crate) mod text;
pub mod freehand;
pub mod path;

pub use rect::Rectangle;
pub use ellipse::Ellipse;
pub use line::Line;
pub use arrow::Arrow;
pub use text::Text;
pub use freehand::Freehand;

use super::ShapeColor;

/// Unique identifier for an element in the scene.
pub type ElementId = u64;

/// A 2-D point in world space.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Point {
    /// Horizontal coordinate (positive right).
    pub x: f64,
    /// Vertical coordinate (positive down).
    pub y: f64,
}

/// Common data shared by every element type: position, size, and appearance.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ElementData {
    /// Unique identifier for this element, assigned when added to a Scene.
    pub id: ElementId,
    /// World-space X of the element's anchor (top-left for rect/ellipse/text).
    pub x: f64,
    /// World-space Y of the element's anchor.
    pub y: f64,
    /// Width of the element's bounding box in world-space.
    pub width: f64,
    /// Height of the element's bounding box in world-space.
    pub height: f64,
    /// Clockwise rotation in radians around the element's center.
    pub rotation: f64,
    /// Font size in world-space units (used by [`Text`]).
    pub font_size: f64,
    /// Stroke (outline) colour.
    pub stroke_color: ShapeColor,
    /// Optional fill colour. `None` means transparent.
    pub fill_color: Option<ShapeColor>,
    /// Stroke width in world-space units.
    pub stroke_width: f64,
}

impl ElementData {
    /// Creates an `ElementData` with defaults: position (0,0), size 100×100,
    /// no rotation, font size 24, white stroke, no fill, stroke width 2.
    pub fn new(id: ElementId) -> Self {
        Self {
            id,
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
            rotation: 0.0,
            font_size: 24.0,
            stroke_color: ShapeColor::default(),
            fill_color: None,
            stroke_width: 2.0,
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
    /// A [`Line`] segment.
    Line(Line),
    /// An [`Arrow`] with a V-shaped head.
    Arrow(Arrow),
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
            Element::Arrow(e) => e.data.id,
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
            Element::Arrow(e) => &e.data,
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
            Element::Arrow(e) => &mut e.data,
            Element::Text(e) => &mut e.data,
            Element::Freehand(e) => &mut e.data,
        }
    }
}

// ── Into<Element> conversions ─────────────────────────────────────────

impl From<Rectangle> for Element {
    fn from(e: Rectangle) -> Self { Element::Rectangle(e) }
}
impl From<Ellipse> for Element {
    fn from(e: Ellipse) -> Self { Element::Ellipse(e) }
}
impl From<Line> for Element {
    fn from(e: Line) -> Self { Element::Line(e) }
}
impl From<Arrow> for Element {
    fn from(e: Arrow) -> Self { Element::Arrow(e) }
}
impl From<Text> for Element {
    fn from(e: Text) -> Self { Element::Text(e) }
}
impl From<Freehand> for Element {
    fn from(e: Freehand) -> Self { Element::Freehand(e) }
}

// ── Trait definitions ──────────────────────────────────────────────────

/// SVG rendering for an element type.
pub trait Render {
    /// Produce an SVG view for this element, scaled by `zoom`.
    fn render(&self, zoom: f64) -> leptos::View;
}

/// Hit-testing: check if a world-space point intersects the element within a margin.
pub trait HitTest {
    /// Returns `true` if `point` lies on/near this element within `margin` world-space units.
    fn hit_test(&self, point: (f64, f64), margin: f64) -> bool;
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
/// point-based types — do not "fix" that by also writing into
/// `data.rotation` for point-based types, which would double-apply the
/// rotation.
pub trait Rotate {
    /// Rotate by `delta` radians around the point (`cx`, `cy`).
    fn rotate_around(&mut self, cx: f64, cy: f64, delta: f64);
}

/// Parameters for resizing an element via drag handles.
#[derive(Clone, Copy)]
pub struct ResizeContext<'a> {
    /// Original (pre-drag) element data — used to prevent scale compounding.
    pub orig: &'a Element,
    /// Bounding box at drag start.
    pub bx: f64,
    pub by: f64,
    pub bw: f64,
    pub bh: f64,
    /// Total mouse delta from drag start.
    pub dx: f64,
    pub dy: f64,
    /// Per-frame mouse delta (for elements like lines that move individual endpoints).
    pub fdx: f64,
    pub fdy: f64,
    /// Handle index (0–7 for corners/edges).
    pub handle: usize,
    /// Whether Shift is held (preserve aspect ratio).
    pub shift: bool,
    /// Whether multiple elements are being resized together.
    pub multi: bool,
}

/// Resize the element using the given context.
pub trait Resize {
    /// Mutate this element's geometry based on `ctx` (drag deltas, handle index, etc.).
    fn resize(&mut self, ctx: &ResizeContext);
}

/// Construct an element from a mouse-drag operation (anchor → current position).
pub trait FromDrag: Sized {
    /// Create a new element of this type given the drag start and current world-space points.
    fn from_drag(
        anchor: (f64, f64),
        current: (f64, f64),
        color: ShapeColor,
        shift: bool,
    ) -> Self;
}

/// Update an element while it is being drawn (e.g. freehand adding points).
pub trait UpdateDrag {
    /// Adjust the element in response to a mouse move during the drag that created it.
    fn update_drag(&mut self, current: (f64, f64), anchor: (f64, f64), shift: bool);
}

// ── Blanket trait implementations on Element ───────────────────────────

impl Render for Element {
    fn render(&self, zoom: f64) -> leptos::View {
        match self {
            Element::Rectangle(e) => e.render(zoom),
            Element::Ellipse(e) => e.render(zoom),
            Element::Line(e) => e.render(zoom),
            Element::Arrow(e) => e.render(zoom),
            Element::Text(e) => e.render(zoom),
            Element::Freehand(e) => e.render(zoom),
        }
    }
}

impl HitTest for Element {
    fn hit_test(&self, point: (f64, f64), margin: f64) -> bool {
        match self {
            Element::Rectangle(e) => e.hit_test(point, margin),
            Element::Ellipse(e) => e.hit_test(point, margin),
            Element::Line(e) => e.hit_test(point, margin),
            Element::Arrow(e) => e.hit_test(point, margin),
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
            Element::Arrow(e) => e.bounds(),
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
            Element::Arrow(e) => e.offset(dx, dy),
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
            Element::Arrow(e) => e.snap_to_grid(grid),
            Element::Text(e) => e.snap_to_grid(grid),
            Element::Freehand(e) => e.snap_to_grid(grid),
        }
    }
}

impl Rotate for Element {
    fn rotate_around(&mut self, cx: f64, cy: f64, delta: f64) {
        match self {
            Element::Rectangle(e) => e.rotate_around(cx, cy, delta),
            Element::Ellipse(e) => e.rotate_around(cx, cy, delta),
            Element::Line(e) => e.rotate_around(cx, cy, delta),
            Element::Arrow(e) => e.rotate_around(cx, cy, delta),
            Element::Text(e) => e.rotate_around(cx, cy, delta),
            Element::Freehand(e) => e.rotate_around(cx, cy, delta),
        }
    }
}

impl Resize for Element {
    fn resize(&mut self, ctx: &ResizeContext) {
        match self {
            Element::Rectangle(e) => e.resize(ctx),
            Element::Ellipse(e) => e.resize(ctx),
            Element::Line(e) => e.resize(ctx),
            Element::Arrow(e) => e.resize(ctx),
            Element::Text(e) => e.resize(ctx),
            Element::Freehand(e) => e.resize(ctx),
        }
    }
}

impl UpdateDrag for Element {
    fn update_drag(&mut self, current: (f64, f64), anchor: (f64, f64), shift: bool) {
        match self {
            Element::Rectangle(e) => e.update_drag(current, anchor, shift),
            Element::Ellipse(e) => e.update_drag(current, anchor, shift),
            Element::Line(e) => e.update_drag(current, anchor, shift),
            Element::Arrow(e) => e.update_drag(current, anchor, shift),
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
