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

/// The frame smart guides attach to: the corners of the visible content box in
/// world space. Elements that rotate in place keep their layout box centre, so
/// the frame is the content box swung around it. Text uses its glyph box, the
/// one its selection outline hugs, instead of the taller and wider layout
/// column.
pub fn guide_corners(element: &Element) -> [Point; 4] {
    let bounds = visual_bounds(element);
    let rotation = element.data().rotation;
    if rotation == 0.0 {
        return bounds.corners();
    }
    rotated_corners(bounds, layout_bounds(element).center(), rotation)
}

/// Axis-aligned envelope of the visible content after rotation. Multi-element
/// selection frames use this so rotated elements stay inside the union box.
pub fn visual_envelope(element: &Element) -> Rect {
    envelope(guide_corners(element))
}

/// The shape inscribed in a frame's box: the box itself, or the ellipse.
/// Guides end on it, so a circle's line stops on its outline rather than at
/// the corners of the box drawn around it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameShape {
    Rect,
    Ellipse,
}

/// The box smart guides measure against, with the shape inside it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuideFrame {
    pub corners: [Point; 4],
    pub shape: FrameShape,
}

impl GuideFrame {
    /// The frame of a plain box: for selections with no single element behind
    /// them, and for elements whose shape is the box.
    pub fn rect(bounds: Rect) -> Self {
        Self {
            corners: bounds.corners(),
            shape: FrameShape::Rect,
        }
    }

    /// The same frame, moved by the delta.
    pub fn translated(self, dx: f64, dy: f64) -> Self {
        Self {
            corners: self
                .corners
                .map(|corner| Point::new(corner.x + dx, corner.y + dy)),
            shape: self.shape,
        }
    }
}

/// The shape inscribed in [`guide_corners`].
fn guide_shape(element: &Element) -> FrameShape {
    match element {
        Element::Ellipse(_) => FrameShape::Ellipse,
        _ => FrameShape::Rect,
    }
}

/// The frame smart guides attach to for one element.
pub fn guide_frame(element: &Element) -> GuideFrame {
    GuideFrame {
        corners: guide_corners(element),
        shape: guide_shape(element),
    }
}

/// The box smart guides line up: the axis-aligned envelope of the shape. For
/// a plain box that is the envelope of its frame; an ellipse stops short of
/// the frame's corners, so the extent of the outline itself is measured.
pub fn guide_bounds(element: &Element) -> Rect {
    match guide_shape(element) {
        FrameShape::Rect => envelope(guide_corners(element)),
        FrameShape::Ellipse => ellipse_bounds(element),
    }
}

/// Union of the boxes smart guides line up for a selection.
pub fn guide_union_bounds(ids: &[ElementId], elements: &[Element]) -> Option<Rect> {
    elements
        .iter()
        .filter(|el| ids.contains(&el.id()))
        .map(guide_bounds)
        .reduce(|left, right| left.union(&right))
}

/// The frame smart guides attach to for a selection. A single element keeps
/// its own frame, so guides meet the shape the selection outline hugs. A
/// multi-selection has no single frame and falls back to the corners of the
/// union box.
pub fn guide_union_frame(ids: &[ElementId], elements: &[Element]) -> Option<GuideFrame> {
    if ids.len() == 1 {
        return elements
            .iter()
            .find(|el| el.id() == ids[0])
            .map(guide_frame);
    }
    guide_union_bounds(ids, elements).map(GuideFrame::rect)
}

/// Axis-aligned extent of the turned ellipse inscribed in the frame's box:
/// the half-axes swing with the rotation, so the box's own envelope would
/// overstate them.
fn ellipse_bounds(element: &Element) -> Rect {
    let bounds = visual_bounds(element);
    let (half_w, half_h) = (bounds.w / 2.0, bounds.h / 2.0);
    let (sin, cos) = element.data().rotation.sin_cos();
    let half_x = ((half_w * cos).powi(2) + (half_h * sin).powi(2)).sqrt();
    let half_y = ((half_w * sin).powi(2) + (half_h * cos).powi(2)).sqrt();
    let center = layout_bounds(element).center();
    Rect::new(
        center.x - half_x,
        center.y - half_y,
        half_x * 2.0,
        half_y * 2.0,
    )
}

/// Corners of `bounds` swung around `center` by `rotation` radians.
fn rotated_corners(bounds: Rect, center: Point, rotation: f64) -> [Point; 4] {
    bounds.corners().map(|mut corner| {
        corner.rotate_around(center, rotation);
        corner
    })
}

/// Axis-aligned envelope of four corners.
fn envelope(corners: [Point; 4]) -> Rect {
    let mut min = corners[0];
    let mut max = corners[0];
    for corner in corners {
        min.x = min.x.min(corner.x);
        min.y = min.y.min(corner.y);
        max.x = max.x.max(corner.x);
        max.y = max.y.max(corner.y);
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
    use crate::model::elements::ellipse::Ellipse;
    use crate::model::elements::rect::Rectangle;
    use crate::model::elements::ElementData;
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

    fn placed_data(id: ElementId, x: f64, y: f64, w: f64, h: f64, rotation: f64) -> ElementData {
        let mut data = ElementData::new(id);
        data.world_point = Point::new(x, y);
        data.width = w;
        data.height = h;
        data.rotation = rotation;
        data
    }

    fn rect_element(id: ElementId, x: f64, y: f64, w: f64, h: f64, rotation: f64) -> Element {
        Element::Rectangle(Rectangle {
            data: placed_data(id, x, y, w, h, rotation),
        })
    }

    fn ellipse_element(id: ElementId, x: f64, y: f64, w: f64, h: f64, rotation: f64) -> Element {
        Element::Ellipse(Ellipse {
            data: placed_data(id, x, y, w, h, rotation),
        })
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

    #[test]
    fn guide_corners_follow_the_rotation() {
        // A quarter-turned box keeps its centre and swaps its sides.
        let turned = rect_element(1, 0.0, 0.0, 100.0, 50.0, FRAC_PI_2);
        assert_eq!(
            envelope(guide_corners(&turned)),
            Rect::new(25.0, -25.0, 50.0, 100.0)
        );

        // A square turned 45 degrees keeps its corners on the diamond around
        // its centre, not on the corners of the envelope.
        let square = rect_element(2, 0.0, 0.0, 100.0, 100.0, FRAC_PI_4);
        let reach = 50.0 * 2.0f64.sqrt();
        for corner in guide_corners(&square) {
            let diamond = (corner.x - 50.0).abs() + (corner.y - 50.0).abs();
            assert!((diamond - reach).abs() < 1e-9);
        }

        // Elements that bake rotation into their geometry stay unrotated here.
        let straight = rect_element(3, 0.0, 0.0, 20.0, 10.0, 0.0);
        assert_eq!(
            guide_corners(&straight),
            Rect::new(0.0, 0.0, 20.0, 10.0).corners()
        );
    }

    #[test]
    fn guide_union_frame_keeps_a_single_frame() {
        let square = rect_element(1, 0.0, 0.0, 100.0, 100.0, FRAC_PI_4);
        assert_eq!(
            guide_union_frame(&[1], std::slice::from_ref(&square)),
            Some(guide_frame(&square))
        );

        // A single ellipse keeps its outline too, not the union box.
        let circle = ellipse_element(3, 0.0, 0.0, 100.0, 100.0, FRAC_PI_4);
        let frame = guide_union_frame(&[3], std::slice::from_ref(&circle)).unwrap();
        assert_eq!(frame.shape, FrameShape::Ellipse);

        // A multi-selection has no single frame: the union box's corners.
        let elements = [
            rect_element(1, 0.0, 0.0, 20.0, 20.0, 0.0),
            rect_element(2, 100.0, 50.0, 20.0, 20.0, 0.0),
        ];
        assert_eq!(
            guide_union_frame(&[1, 2], &elements),
            Some(GuideFrame::rect(Rect::new(0.0, 0.0, 120.0, 70.0)))
        );
        assert_eq!(guide_union_frame(&[3], &elements), None);
    }

    #[test]
    fn guide_bounds_of_an_ellipse_follow_its_outline() {
        // A circle turned 45 degrees reaches no further than its own radius:
        // the envelope of the turned box would put it at radius * sqrt(2).
        let circle = ellipse_element(1, 0.0, 0.0, 100.0, 100.0, FRAC_PI_4);
        let turned = guide_bounds(&circle);
        assert!(
            (turned.x).abs() < 1e-9
                && (turned.y).abs() < 1e-9
                && (turned.w - 100.0).abs() < 1e-9
                && (turned.h - 100.0).abs() < 1e-9,
            "{turned:?}"
        );

        // A quarter turn swaps an oval's sides around its centre.
        let oval = ellipse_element(2, 0.0, 0.0, 200.0, 100.0, FRAC_PI_2);
        assert_eq!(guide_bounds(&oval), Rect::new(50.0, -50.0, 100.0, 200.0));

        // At any other angle the half-axes swing with it.
        let slanted = ellipse_element(3, 0.0, 0.0, 200.0, 100.0, FRAC_PI_4);
        let (sin, cos) = FRAC_PI_4.sin_cos();
        let half_x = ((100.0 * cos).powi(2) + (50.0 * sin).powi(2)).sqrt();
        let half_y = ((100.0 * sin).powi(2) + (50.0 * cos).powi(2)).sqrt();
        assert_eq!(
            guide_bounds(&slanted),
            Rect::new(100.0 - half_x, 50.0 - half_y, half_x * 2.0, half_y * 2.0)
        );
    }

    #[test]
    fn text_guides_hug_the_glyphs() {
        use crate::model::elements::text::{Text, TextAlign, WrappedText};

        let mut data = ElementData::new(1);
        data.world_point = Point::new(0.0, 0.0);
        data.width = 300.0;
        data.height = 200.0;
        data.style.font_size = 20.0;
        let element = Element::Text(Text {
            data,
            wrapped: WrappedText::new("Hello"),
            align: TextAlign::Left,
        });

        // The layout column is 300 wide and 200 tall; the guides read the
        // glyphs, which sit at the anchor and stay well inside it.
        let bounds = guide_bounds(&element);
        assert!(bounds.w > 0.0 && bounds.w < 300.0);
        assert!(bounds.h > 0.0 && bounds.h < 200.0);
        assert_eq!(bounds.x, 0.0);

        // Quarter-turned, the box follows the glyphs around the layout centre.
        let mut turned = element.clone();
        if let Element::Text(text) = &mut turned {
            text.data.rotation = std::f64::consts::FRAC_PI_2;
        }
        let turned = guide_bounds(&turned);
        assert!((turned.w - bounds.h).abs() < 1e-9);
        assert!((turned.h - bounds.w).abs() < 1e-9);
    }
}
