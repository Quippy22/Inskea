mod color;
mod element;

pub use color::ShapeColor;
pub use element::{Element, ElementData, ElementId, Point};

/// The single source of truth for everything on the canvas.
#[derive(Clone, Debug)]
pub struct Scene {
    pub elements: Vec<Element>,
}
