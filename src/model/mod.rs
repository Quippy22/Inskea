mod color;
pub mod elements;
pub mod point;
pub mod resize;

pub use color::ShapeColor;
pub use elements::{
    Arrow, Element, ElementData, ElementId, Ellipse, Freehand, Line, Rectangle, Text,
};
pub use elements::{
    Bounds, FromDrag, HitTest, Offset, PathPoints, Render, Resize, Rotate, SnapToGrid, UpdateDrag,
};
pub use point::Point;


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

    pub fn elements(&self) -> &[Element] {
        &self.elements
    }

    pub fn elements_mut(&mut self) -> &mut Vec<Element> {
        &mut self.elements
    }

    pub fn element_by_id(&self, id: ElementId) -> Option<&Element> {
        self.elements.iter().find(|e| e.id() == id)
    }

    pub fn element_by_id_mut(&mut self, id: ElementId) -> Option<&mut Element> {
        self.elements.iter_mut().find(|e| e.id() == id)
    }

    pub fn remove_by_id(&mut self, id: ElementId) {
        self.elements.retain(|e| e.id() != id);
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


