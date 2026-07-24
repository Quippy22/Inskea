mod color;
pub mod elements;
pub mod point;
pub mod resize;

pub use color::Color;
pub use elements::*;
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

    /// Move element to the front of the z-order (top of canvas).
    pub fn bring_to_front(&mut self, id: ElementId) {
        if let Some(idx) = self.elements.iter().position(|e| e.id() == id) {
            let el = self.elements.remove(idx);
            self.elements.push(el);
        }
    }

    /// Move element to the back of the z-order (bottom of canvas).
    pub fn send_to_back(&mut self, id: ElementId) {
        if let Some(idx) = self.elements.iter().position(|e| e.id() == id) {
            let el = self.elements.remove(idx);
            self.elements.insert(0, el);
        }
    }

    /// Move element one step toward the front.
    pub fn bring_forward(&mut self, id: ElementId) {
        let len = self.elements.len();
        if len < 2 {
            return;
        }
        if let Some(idx) = self.elements.iter().position(|e| e.id() == id) {
            if idx < len - 1 {
                self.elements.swap(idx, idx + 1);
            }
        }
    }

    /// Move element one step toward the back.
    pub fn send_backward(&mut self, id: ElementId) {
        if let Some(idx) = self.elements.iter().position(|e| e.id() == id) {
            if idx > 0 {
                self.elements.swap(idx, idx - 1);
            }
        }
    }
}
