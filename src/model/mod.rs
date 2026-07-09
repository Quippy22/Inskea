mod color;
pub mod elements;

pub use color::ShapeColor;
pub use elements::{
    Arrow, Element, ElementData, ElementId, Ellipse, Freehand, Line, Point, Rectangle, Text,
};
pub use elements::{Bounds, FromDrag, HitTest, Offset, PathPoints, Render, Resize, Rotate, SnapToGrid, UpdateDrag};

/// The single source of truth for everything on the canvas.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Scene {
    pub(crate) elements: Vec<Element>,
    pub(crate) next_id: u64,
}

impl Scene {
    /// Creates an empty scene with no elements (IDs start at 1).
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            next_id: 1,
        }
    }

    /// Adds an element to the scene, assigning it a unique ID.
    ///
    /// The element's previous ID is overwritten.
    pub fn add_element(&mut self, mut element: Element) {
        element.data_mut().id = self.next_id;
        self.next_id += 1;
        self.elements.push(element);
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}
