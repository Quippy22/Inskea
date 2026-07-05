mod color;
mod element;

pub use color::ShapeColor;
pub use element::{Element, ElementData, ElementId, Point};

/// The single source of truth for everything on the canvas.
#[derive(Clone, Debug)]
pub struct Scene {
    pub elements: Vec<Element>,
    pub next_id: u64,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            next_id: 1,
        }
    }

    pub fn add_element(&mut self, mut element: Element) {
        match &mut element {
            Element::Rectangle(d)
            | Element::Ellipse(d)
            | Element::Line(d, ..)
            | Element::Arrow(d, ..)
            | Element::Text(d, ..)
            | Element::Freehand(d, ..) => {
                d.id = self.next_id;
            }
        }
        self.next_id += 1;
        self.elements.push(element);
    }

    pub fn next_id(&self) -> u64 {
        self.next_id
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}
