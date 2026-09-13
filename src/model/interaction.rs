use super::{Bounds, Element, ElementId, HitTest, Point, Rect};

pub const HIT_MARGIN: f64 = 6.0;
pub const CLICK_MARGIN: f64 = 12.0;

/// The element's persisted layout box, used for scene geometry and export.
pub fn layout_bounds(element: &Element) -> Rect {
    element.bounds()
}

/// The visible content box, used for selection visuals and tight overlays.
pub fn visual_bounds(element: &Element) -> Rect {
    match element {
        Element::Text(text) => text.visual_bounds(),
        _ => layout_bounds(element),
    }
}

/// Bounds used when distributing visible content. Text has a larger layout
/// column than its rendered glyphs, so using its layout box makes mixed text
/// and shape distributions look uneven to the user.
pub fn distribution_bounds(element: &Element) -> Rect {
    visual_bounds(element)
}

/// Return the axis-aligned bounds of the selected elements.
pub fn combined_bounds(ids: &[ElementId], elements: &[Element]) -> Option<Rect> {
    elements
        .iter()
        .filter(|el| ids.contains(&el.id()))
        .map(layout_bounds)
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
    rect.contains_rect(&layout_bounds(element))
}
