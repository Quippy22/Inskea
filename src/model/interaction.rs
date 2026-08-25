use super::{Bounds, Element, ElementId, HitTest, Point, Rect};

pub const HIT_MARGIN: f64 = 6.0;
pub const CLICK_MARGIN: f64 = 12.0;

/// Return the axis-aligned bounds of the selected elements.
pub fn combined_bounds(ids: &[ElementId], elements: &[Element]) -> Option<Rect> {
    elements
        .iter()
        .filter(|el| ids.contains(&el.id()))
        .map(|el| el.bounds())
        .reduce(|left, right| left.union(&right))
}

/// Hit-test from front to back and return the topmost matching element.
pub fn hit_test_topmost(point: Point, elements: &[Element], margin: f64) -> Option<ElementId> {
    elements
        .iter()
        .rev()
        .find(|el| el.hit_test(point, margin))
        .map(|el| el.id())
}

/// Return whether a point is inside any element's padded interaction bounds.
pub fn point_inside_any_element(point: Point, elements: &[Element], margin: f64) -> bool {
    elements.iter().any(|el| {
        let b = el.bounds();
        point.x >= b.x - margin
            && point.x <= b.x + b.w + margin
            && point.y >= b.y - margin
            && point.y <= b.y + b.h + margin
    })
}

/// Return whether the rectangle fully contains an element's bounds.
pub fn rect_fully_contains_element(rect: Rect, element: &Element) -> bool {
    rect.contains_rect(&element.bounds())
}
