use super::elements::Bounds;
use super::interaction::{distribution_bounds, layout_bounds};
use super::{Element, ElementId, Frame, Offset, Rect, Scene};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alignment {
    Left,
    HorizontalCenter,
    Right,
    Top,
    VerticalCenter,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Distribution {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug)]
struct SelectionItem {
    ids: Vec<ElementId>,
    bounds: Rect,
}

fn selection_items(scene: &Scene, ids: &[ElementId]) -> Vec<SelectionItem> {
    selection_items_with_bounds(scene, ids, layout_bounds)
}

fn distribution_items(scene: &Scene, ids: &[ElementId]) -> Vec<SelectionItem> {
    selection_items_with_bounds(scene, ids, distribution_bounds)
}

fn selection_items_with_bounds(
    scene: &Scene,
    ids: &[ElementId],
    bounds_for: fn(&super::Element) -> Rect,
) -> Vec<SelectionItem> {
    let mut items = Vec::new();
    let mut grouped_ids = Vec::new();

    for element in scene.elements() {
        let id = element.id();
        if !ids.contains(&id) || grouped_ids.contains(&id) {
            continue;
        }

        let Some(group) = scene.group_containing(id) else {
            items.push(SelectionItem {
                ids: vec![id],
                bounds: bounds_for(&element),
            });
            continue;
        };

        if !group.members.iter().all(|member| ids.contains(member)) {
            items.push(SelectionItem {
                ids: vec![id],
                bounds: bounds_for(&element),
            });
            continue;
        }

        let mut bounds: Option<Rect> = None;
        for member in &group.members {
            if let Some(element) = scene.element_by_id(*member) {
                bounds = Some(match bounds {
                    Some(current) => current.union(&bounds_for(&element)),
                    None => bounds_for(&element),
                });
            }
        }
        if let Some(bounds) = bounds {
            grouped_ids.extend(group.members.iter().copied());
            items.push(SelectionItem {
                ids: group.members.clone(),
                bounds,
            });
        }
    }

    items
}

fn offset_item(scene: &mut Scene, item: &SelectionItem, dx: f64, dy: f64) {
    for id in &item.ids {
        if let Some(mut element) = scene.element_by_id_mut(*id) {
            element.offset(dx, dy);
        }
    }
}

fn union_bounds(items: &[SelectionItem]) -> Option<Rect> {
    items
        .iter()
        .map(|item| item.bounds)
        .reduce(|left, right| left.union(&right))
}

pub fn selection_item_count(scene: &Scene, ids: &[ElementId]) -> usize {
    selection_items(scene, ids).len()
}

/// Find an enclosing Frame for the given selection, if any.
///
/// 1. If any selected element is a Frame and there are other elements in the selection,
///    that Frame is used as the target frame.
/// 2. If no Frame is explicitly selected, searches the scene's frames to find one that
///    contains the selection's bounding box center (or bounds).
pub fn find_enclosing_frame(scene: &Scene, ids: &[ElementId]) -> Option<Frame> {
    if ids.is_empty() {
        return None;
    }

    let selected_frames: Vec<Frame> = ids
        .iter()
        .filter_map(|id| match scene.element_by_id(*id) {
            Some(Element::Frame(f)) => Some(f.clone()),
            _ => None,
        })
        .collect();

    let non_frame_count = ids.len() - selected_frames.len();
    if !selected_frames.is_empty() && non_frame_count > 0 {
        return selected_frames.into_iter().next();
    }

    let items = selection_items(scene, ids);
    let all_bounds = union_bounds(&items)?;

    let center = all_bounds.center();
    let frames = scene.frames_ordered();

    frames
        .into_iter()
        .filter(|f| f.bounds().contains_point(center) || f.bounds().contains_rect(&all_bounds))
        .min_by(|a, b| {
            let area_a = a.bounds().w * a.bounds().h;
            let area_b = b.bounds().w * b.bounds().h;
            area_a
                .partial_cmp(&area_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// Find all element IDs positioned inside the given frame (excluding frames themselves).
pub fn elements_inside_frame(scene: &Scene, frame: &Frame) -> Vec<ElementId> {
    let frame_bounds = frame.bounds();
    let frame_id = frame.data.id;
    scene
        .elements()
        .iter()
        .filter(|el| {
            if el.id() == frame_id || matches!(el, Element::Frame(_)) {
                return false;
            }
            let b = layout_bounds(el);
            frame_bounds.contains_point(b.center()) || frame_bounds.intersects(&b)
        })
        .map(|el| el.id())
        .collect()
}

/// Returns whether the current selection can be aligned.
///
/// Returns true if:
/// - 2 or more items are selected, OR
/// - Exactly 1 item is selected, and it is a Frame that contains elements inside it.
pub fn can_align_selection(scene: &Scene, ids: &[ElementId]) -> bool {
    let items = selection_items(scene, ids);
    if items.is_empty() {
        return false;
    }
    if ids.len() == 1 {
        if let Some(Element::Frame(frame)) = scene.element_by_id(ids[0]) {
            return !elements_inside_frame(scene, &frame).is_empty();
        }
        return false;
    }
    items.len() >= 2
}

pub fn align_selection(scene: &mut Scene, ids: &[ElementId], alignment: Alignment) {
    let items = selection_items(scene, ids);
    if items.is_empty() {
        return;
    }

    // When a single frame is selected, align all elements inside the frame as a block/group!
    if ids.len() == 1 {
        if let Some(Element::Frame(frame)) = scene.element_by_id(ids[0]) {
            let inside_ids = elements_inside_frame(scene, &frame);
            if !inside_ids.is_empty() {
                let inner_items = selection_items(scene, &inside_ids);
                align_items_to_frame(scene, &inner_items, &frame, alignment);
            }
            return;
        }
    }

    if items.len() < 2 {
        return;
    }

    let has_frame_in_selection = ids
        .iter()
        .any(|id| matches!(scene.element_by_id(*id), Some(Element::Frame(_))));

    if has_frame_in_selection {
        if let Some(frame) = find_enclosing_frame(scene, ids) {
            align_items_to_frame(scene, &items, &frame, alignment);
            return;
        }
    }

    let Some(all_bounds) = union_bounds(&items) else {
        return;
    };

    for item in &items {
        let target_pos = match alignment {
            Alignment::Left => all_bounds.x,
            Alignment::HorizontalCenter => all_bounds.center().x - item.bounds.w / 2.0,
            Alignment::Right => all_bounds.right() - item.bounds.w,
            Alignment::Top => all_bounds.y,
            Alignment::VerticalCenter => all_bounds.center().y - item.bounds.h / 2.0,
            Alignment::Bottom => all_bounds.bottom() - item.bounds.h,
        };
        let (dx, dy) = match alignment {
            Alignment::Left | Alignment::HorizontalCenter | Alignment::Right => {
                (target_pos - item.bounds.x, 0.0)
            }
            Alignment::Top | Alignment::VerticalCenter | Alignment::Bottom => {
                (0.0, target_pos - item.bounds.y)
            }
        };
        offset_item(scene, item, dx, dy);
    }
}

fn align_items_to_frame(
    scene: &mut Scene,
    items: &[SelectionItem],
    frame: &Frame,
    alignment: Alignment,
) {
    let frame_bounds = frame.bounds();
    let frame_id = frame.data.id;

    // Filter out the frame itself so it NEVER moves
    let non_frame_items: Vec<&SelectionItem> = items
        .iter()
        .filter(|item| !item.ids.contains(&frame_id))
        .collect();

    let Some(all_bounds) = non_frame_items
        .iter()
        .map(|item| item.bounds)
        .reduce(|left, right| left.union(&right))
    else {
        return;
    };

    let (dx, dy) = match alignment {
        Alignment::Left => (frame_bounds.x - all_bounds.x, 0.0),
        Alignment::HorizontalCenter => (frame_bounds.center().x - all_bounds.center().x, 0.0),
        Alignment::Right => (frame_bounds.right() - all_bounds.right(), 0.0),
        Alignment::Top => (0.0, frame_bounds.y - all_bounds.y),
        Alignment::VerticalCenter => (0.0, frame_bounds.center().y - all_bounds.center().y),
        Alignment::Bottom => (0.0, frame_bounds.bottom() - all_bounds.bottom()),
    };

    for item in non_frame_items {
        offset_item(scene, item, dx, dy);
    }
}

pub fn distribute_selection(scene: &mut Scene, ids: &[ElementId], distribution: Distribution) {
    let mut items = distribution_items(scene, ids);
    if items.len() < 3 {
        return;
    }

    match distribution {
        Distribution::Horizontal => items.sort_by(|left, right| {
            left.bounds
                .x
                .partial_cmp(&right.bounds.x)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        Distribution::Vertical => items.sort_by(|left, right| {
            left.bounds
                .y
                .partial_cmp(&right.bounds.y)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }

    let first = items.first().unwrap().bounds;
    let last = items.last().unwrap().bounds;
    let total_size: f64 = items
        .iter()
        .map(|item| match distribution {
            Distribution::Horizontal => item.bounds.w,
            Distribution::Vertical => item.bounds.h,
        })
        .sum();
    let span = match distribution {
        Distribution::Horizontal => last.right() - first.x,
        Distribution::Vertical => last.bottom() - first.y,
    };
    let gap = (span - total_size) / (items.len() - 1) as f64;
    let mut cursor = match distribution {
        Distribution::Horizontal => first.right() + gap,
        Distribution::Vertical => first.bottom() + gap,
    };

    for item in items.iter().skip(1).take(items.len() - 2) {
        let (position, size) = match distribution {
            Distribution::Horizontal => (item.bounds.x, item.bounds.w),
            Distribution::Vertical => (item.bounds.y, item.bounds.h),
        };
        let delta = cursor - position;
        match distribution {
            Distribution::Horizontal => offset_item(scene, item, delta, 0.0),
            Distribution::Vertical => offset_item(scene, item, 0.0, delta),
        }
        cursor += size + gap;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::elements::rect::Rectangle;
    use crate::model::elements::text::{Text, WrappedText};
    use crate::model::elements::Bounds;
    use crate::model::{Element, ElementData, Point, TextAlign};

    fn rectangle(id: ElementId, x: f64, y: f64, width: f64, height: f64) -> Element {
        Element::Rectangle(Rectangle {
            data: ElementData {
                id,
                world_point: Point::new(x, y),
                width,
                height,
                ..ElementData::new(id)
            },
        })
    }

    fn text(id: ElementId, x: f64, y: f64, raw: &str, width: f64, height: f64) -> Element {
        let mut data = ElementData::new(id);
        data.world_point = Point::new(x, y);
        data.width = width;
        data.height = height;
        Element::Text(Text {
            data,
            wrapped: WrappedText::new(raw),
            align: TextAlign::Left,
        })
    }

    fn scene() -> Scene {
        let mut scene = Scene::new();
        scene.add_element(rectangle(0, 10.0, 10.0, 20.0, 10.0));
        scene.add_element(rectangle(0, 60.0, 30.0, 20.0, 10.0));
        scene.add_element(rectangle(0, 110.0, 50.0, 20.0, 10.0));
        scene
    }

    #[test]
    fn align_left_uses_the_outer_selection_edge() {
        let mut scene = scene();
        align_selection(&mut scene, &[1, 2, 3], Alignment::Left);

        assert_eq!(scene.element_by_id(1).unwrap().bounds().x, 10.0);
        assert_eq!(scene.element_by_id(2).unwrap().bounds().x, 10.0);
        assert_eq!(scene.element_by_id(3).unwrap().bounds().x, 10.0);
    }

    #[test]
    fn distribute_horizontal_preserves_outer_edges() {
        let mut scene = scene();
        distribute_selection(&mut scene, &[1, 2, 3], Distribution::Horizontal);

        assert_eq!(scene.element_by_id(1).unwrap().bounds().x, 10.0);
        assert_eq!(scene.element_by_id(2).unwrap().bounds().x, 60.0);
        assert_eq!(scene.element_by_id(3).unwrap().bounds().x, 110.0);
    }

    #[test]
    fn distribute_horizontal_moves_middle_item_to_equalize_gaps() {
        let mut scene = Scene::new();
        scene.add_element(rectangle(0, 10.0, 0.0, 20.0, 10.0));
        scene.add_element(rectangle(0, 50.0, 0.0, 10.0, 10.0));
        scene.add_element(rectangle(0, 180.0, 0.0, 30.0, 10.0));

        distribute_selection(&mut scene, &[1, 2, 3], Distribution::Horizontal);

        assert_eq!(scene.element_by_id(1).unwrap().bounds().x, 10.0);
        assert_eq!(scene.element_by_id(2).unwrap().bounds().x, 100.0);
        assert_eq!(scene.element_by_id(3).unwrap().bounds().x, 180.0);
    }

    #[test]
    fn distribute_horizontal_uses_text_content_bounds() {
        let mut scene = Scene::new();
        scene.add_element(rectangle(0, 0.0, 0.0, 20.0, 20.0));
        scene.add_element(text(0, 40.0, 0.0, "text", 100.0, 22.0));
        scene.add_element(rectangle(0, 140.0, 0.0, 20.0, 20.0));

        distribute_selection(&mut scene, &[1, 2, 3], Distribution::Horizontal);

        let first = distribution_bounds(&scene.element_by_id(1).unwrap());
        let middle = distribution_bounds(&scene.element_by_id(2).unwrap());
        let last = distribution_bounds(&scene.element_by_id(3).unwrap());
        let first_gap = middle.x - first.right();
        let last_gap = last.x - middle.right();
        assert!((first_gap - last_gap).abs() < 1e-9);
        assert!(middle.x > 40.0);
    }

    #[test]
    fn distribute_vertical_uses_text_content_bounds() {
        let mut scene = Scene::new();
        scene.add_element(rectangle(0, 0.0, 0.0, 20.0, 20.0));
        scene.add_element(text(0, 0.0, 30.0, "text", 100.0, 80.0));
        scene.add_element(rectangle(0, 0.0, 140.0, 20.0, 20.0));

        distribute_selection(&mut scene, &[1, 2, 3], Distribution::Vertical);

        let first = distribution_bounds(&scene.element_by_id(1).unwrap());
        let middle = distribution_bounds(&scene.element_by_id(2).unwrap());
        let last = distribution_bounds(&scene.element_by_id(3).unwrap());
        let first_gap = middle.y - first.bottom();
        let last_gap = last.y - middle.bottom();
        assert!((first_gap - last_gap).abs() < 1e-9);
        assert!(middle.y > 30.0);
    }

    #[test]
    fn alignment_treats_a_complete_group_as_one_item() {
        let mut scene = scene();
        scene.create_group(&[1, 2]);
        align_selection(&mut scene, &[1, 2, 3], Alignment::Top);

        assert_eq!(scene.element_by_id(1).unwrap().bounds().y, 10.0);
        assert_eq!(scene.element_by_id(2).unwrap().bounds().y, 30.0);
        assert_eq!(scene.element_by_id(3).unwrap().bounds().y, 10.0);
    }

    use crate::model::elements::frame::{Frame, FrameOrientation, FramePreset};

    fn test_frame(id: ElementId, x: f64, y: f64, width: f64, height: f64, name: &str) -> Element {
        Element::Frame(Frame {
            data: ElementData {
                id,
                world_point: Point::new(x, y),
                width,
                height,
                ..ElementData::new(id)
            },
            name: name.to_string(),
            preset: FramePreset::Freehand,
            orientation: FrameOrientation::Portrait,
            frame_order: 1,
        })
    }

    #[test]
    fn single_element_in_frame_can_align_and_aligns_correctly() {
        let mut scene = Scene::new();
        scene.add_element(test_frame(0, 100.0, 100.0, 400.0, 300.0, "Frame 1"));
        scene.add_element(rectangle(0, 150.0, 150.0, 100.0, 50.0));

        let frame_id = 1;
        let rect_id = 2;

        // A single shape does not align to the frame on its own
        assert!(!can_align_selection(&scene, &[rect_id]));
        // A selected frame containing elements CAN align its contents
        assert!(can_align_selection(&scene, &[frame_id]));

        align_selection(&mut scene, &[frame_id], Alignment::HorizontalCenter);
        assert_eq!(scene.element_by_id(rect_id).unwrap().bounds().x, 250.0);

        align_selection(&mut scene, &[frame_id], Alignment::VerticalCenter);
        assert_eq!(scene.element_by_id(rect_id).unwrap().bounds().y, 225.0);

        align_selection(&mut scene, &[frame_id], Alignment::Left);
        assert_eq!(scene.element_by_id(rect_id).unwrap().bounds().x, 100.0);

        align_selection(&mut scene, &[frame_id], Alignment::Right);
        assert_eq!(scene.element_by_id(rect_id).unwrap().bounds().x, 400.0);

        align_selection(&mut scene, &[frame_id], Alignment::Top);
        assert_eq!(scene.element_by_id(rect_id).unwrap().bounds().y, 100.0);

        align_selection(&mut scene, &[frame_id], Alignment::Bottom);
        assert_eq!(scene.element_by_id(rect_id).unwrap().bounds().y, 350.0);
    }

    #[test]
    fn empty_frame_cannot_align() {
        let mut scene = Scene::new();
        scene.add_element(test_frame(0, 100.0, 100.0, 400.0, 300.0, "Frame 1"));
        let frame_id = 1;
        assert!(!can_align_selection(&scene, &[frame_id]));
    }

    #[test]
    fn single_element_outside_frame_cannot_align() {
        let mut scene = Scene::new();
        scene.add_element(test_frame(0, 100.0, 100.0, 400.0, 300.0, "Frame 1"));
        scene.add_element(rectangle(0, 700.0, 700.0, 100.0, 50.0));

        let rect_id = 2;
        assert!(!can_align_selection(&scene, &[rect_id]));

        let orig_x = scene.element_by_id(rect_id).unwrap().bounds().x;
        align_selection(&mut scene, &[rect_id], Alignment::HorizontalCenter);
        assert_eq!(scene.element_by_id(rect_id).unwrap().bounds().x, orig_x);
    }

    #[test]
    fn multiple_elements_in_frame_align_as_block_without_grouping() {
        let mut scene = Scene::new();
        scene.add_element(test_frame(0, 0.0, 0.0, 600.0, 400.0, "Frame 1"));
        scene.add_element(rectangle(0, 50.0, 100.0, 100.0, 50.0));
        scene.add_element(rectangle(0, 200.0, 100.0, 100.0, 50.0));

        let frame_id = 1;
        let rect_a = 2;
        let rect_b = 3;

        align_selection(&mut scene, &[frame_id], Alignment::HorizontalCenter);

        let bounds_a = scene.element_by_id(rect_a).unwrap().bounds();
        let bounds_b = scene.element_by_id(rect_b).unwrap().bounds();

        assert_eq!(bounds_a.x, 175.0);
        assert_eq!(bounds_b.x, 325.0);
        assert_eq!(bounds_b.x - bounds_a.right(), 50.0);
        let union_center = (bounds_a.x + bounds_b.right()) / 2.0;
        assert_eq!(union_center, 300.0);
    }

    #[test]
    fn frame_selected_with_elements_keeps_frame_pinned() {
        let mut scene = Scene::new();
        scene.add_element(test_frame(0, 100.0, 100.0, 400.0, 300.0, "Frame 1"));
        scene.add_element(rectangle(0, 150.0, 150.0, 100.0, 50.0));

        let frame_id = 1;
        let rect_id = 2;

        align_selection(&mut scene, &[frame_id, rect_id], Alignment::Left);

        assert_eq!(scene.element_by_id(frame_id).unwrap().bounds().x, 100.0);
        assert_eq!(scene.element_by_id(rect_id).unwrap().bounds().x, 100.0);
    }
}
