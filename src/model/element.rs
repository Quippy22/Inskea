use super::ShapeColor;

/// Unique identifier for an element in the scene.
pub type ElementId = u64;

/// A 2‑D point in world space.
#[derive(Clone, Debug)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// Fields common to every element variant.
#[derive(Clone, Debug)]
pub struct ElementData {
    pub id: ElementId,
    /// World‑space X of the element's anchor point (top‑left for rects/ellipses).
    pub x: f64,
    /// World‑space Y of the element's anchor point.
    pub y: f64,
    /// Width of the element's bounding box (ignored by line‑like shapes).
    pub width: f64,
    /// Height of the element's bounding box (ignored by line‑like shapes).
    pub height: f64,
    /// Clockwise rotation in radians around the element's center.
    pub rotation: f64,
    /// Stroke (outline) colour.
    pub stroke_color: ShapeColor,
    /// Optional fill colour — `None` means transparent.
    pub fill_color: Option<ShapeColor>,
    /// Stroke width in world‑space units.
    pub stroke_width: f64,
}

impl Element {
    pub fn id(&self) -> ElementId {
        match self {
            Element::Rectangle(d)
            | Element::Ellipse(d)
            | Element::Line(d, ..)
            | Element::Arrow(d, ..)
            | Element::Text(d, ..)
            | Element::Freehand(d, ..) => d.id,
        }
    }
}

impl ElementData {
    /// Create an `ElementData` with sensible defaults.
    ///
    /// Defaults: position (0,0), size 100×100, no rotation, white stroke,
    /// no fill, stroke width 2.
    pub fn new(id: ElementId) -> Self {
        Self {
            id,
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
            rotation: 0.0,
            stroke_color: ShapeColor::default(),
            fill_color: None,
            stroke_width: 2.0,
        }
    }
}

/// Every drawable shape on the canvas.
#[derive(Clone, Debug)]
pub enum Element {
    Rectangle(ElementData),
    Ellipse(ElementData),
    /// Line from point A to point B.
    Line(ElementData, Point, Point),
    /// Arrow from point A to point B.
    Arrow(ElementData, Point, Point),
    /// A piece of text with a string content.
    Text(ElementData, String),
    /// A free‑hand stroke made up of a list of sampled points.
    Freehand(ElementData, Vec<Point>),
}
