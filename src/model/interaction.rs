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

/// The box smart guides line up: the visible content box, turned into world
/// space for elements that rotate in place. Rotating keeps the box centre, so
/// the corners of the rotated box give the rectangle the guides should match
/// against.
pub fn guide_bounds(element: &Element) -> Rect {
    let bounds = layout_bounds(element);
    let rotation = element.data().rotation;
    if rotation == 0.0 {
        return bounds;
    }
    rotated_aabb(bounds, bounds.center(), rotation)
}

/// Union of the boxes smart guides line up for a selection.
pub fn guide_union_bounds(ids: &[ElementId], elements: &[Element]) -> Option<Rect> {
    elements
        .iter()
        .filter(|el| ids.contains(&el.id()))
        .map(guide_bounds)
        .reduce(|left, right| left.union(&right))
}

/// Axis-aligned box around `bounds` after rotating it around `center`.
fn rotated_aabb(bounds: Rect, center: Point, rotation: f64) -> Rect {
    let (sin, cos) = rotation.sin_cos();
    let mut min = Point::new(f64::INFINITY, f64::INFINITY);
    let mut max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for corner in bounds.corners() {
        let dx = corner.x - center.x;
        let dy = corner.y - center.y;
        let x = center.x + dx * cos - dy * sin;
        let y = center.y + dx * sin + dy * cos;
        min.x = min.x.min(x);
        min.y = min.y.min(y);
        max.x = max.x.max(x);
        max.y = max.y.max(y);
    }
    Rect::new(min.x, min.y, max.x - min.x, max.y - min.y)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::elements::rect::Rectangle;
    use crate::model::elements::ElementData;
    use std::f64::consts::FRAC_PI_4;

    fn rect_element(id: ElementId, x: f64, y: f64, w: f64, h: f64, rotation: f64) -> Element {
        let mut data = ElementData::new(id);
        data.world_point = Point::new(x, y);
        data.width = w;
        data.height = h;
        data.rotation = rotation;
        Element::Rectangle(Rectangle { data })
    }

    #[test]
    fn guide_bounds_are_the_rotated_box() {
        let square = rect_element(1, 0.0, 0.0, 100.0, 100.0, FRAC_PI_4);
        // A square turned 45 degrees grows by sqrt(2) around its centre.
        let bounds = guide_bounds(&square);
        let reach = 50.0 * 2.0f64.sqrt();
        assert!((bounds.x - (50.0 - reach)).abs() < 1e-9);
        assert!((bounds.y - (50.0 - reach)).abs() < 1e-9);
        assert!((bounds.w - reach * 2.0).abs() < 1e-9);
        assert!((bounds.h - reach * 2.0).abs() < 1e-9);

        // A half turn leaves the box where it was.
        let flipped = rect_element(2, 10.0, 20.0, 60.0, 40.0, std::f64::consts::PI);
        let bounds = guide_bounds(&flipped);
        assert!((bounds.x - 10.0).abs() < 1e-9);
        assert!((bounds.y - 20.0).abs() < 1e-9);
        assert!((bounds.w - 60.0).abs() < 1e-9);
        assert!((bounds.h - 40.0).abs() < 1e-9);
    }

    #[test]
    fn guide_union_bounds_cover_the_selection() {
        let elements = [
            rect_element(1, 0.0, 0.0, 20.0, 20.0, 0.0),
            rect_element(2, 100.0, 50.0, 20.0, 20.0, 0.0),
        ];
        assert_eq!(
            guide_union_bounds(&[1, 2], &elements),
            Some(Rect::new(0.0, 0.0, 120.0, 70.0))
        );
        assert_eq!(guide_union_bounds(&[3], &elements), None);
    }
}
