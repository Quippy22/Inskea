use super::interaction::layout_bounds;
use super::{ElementId, Offset, Rect, Scene};

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
                bounds: layout_bounds(element),
            });
            continue;
        };

        if !group.members.iter().all(|member| ids.contains(member)) {
            items.push(SelectionItem {
                ids: vec![id],
                bounds: layout_bounds(element),
            });
            continue;
        }

        let mut bounds: Option<Rect> = None;
        for member in &group.members {
            if let Some(element) = scene.element_by_id(*member) {
                bounds = Some(match bounds {
                    Some(current) => current.union(&layout_bounds(element)),
                    None => layout_bounds(element),
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
    for element in scene.elements_mut() {
        if item.ids.contains(&element.id()) {
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

pub fn align_selection(scene: &mut Scene, ids: &[ElementId], alignment: Alignment) {
    let items = selection_items(scene, ids);
    let Some(all_bounds) = union_bounds(&items) else {
        return;
    };

    for item in &items {
        let target = match alignment {
            Alignment::Left => all_bounds.x,
            Alignment::HorizontalCenter => all_bounds.center().x - item.bounds.w / 2.0,
            Alignment::Right => all_bounds.right() - item.bounds.w,
            Alignment::Top => all_bounds.y,
            Alignment::VerticalCenter => all_bounds.center().y - item.bounds.h / 2.0,
            Alignment::Bottom => all_bounds.bottom() - item.bounds.h,
        };
        let (dx, dy) = match alignment {
            Alignment::Left | Alignment::HorizontalCenter | Alignment::Right => {
                (target - item.bounds.x, 0.0)
            }
            Alignment::Top | Alignment::VerticalCenter | Alignment::Bottom => {
                (0.0, target - item.bounds.y)
            }
        };
        offset_item(scene, item, dx, dy);
    }
}

pub fn distribute_selection(scene: &mut Scene, ids: &[ElementId], distribution: Distribution) {
    let mut items = selection_items(scene, ids);
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
    use crate::model::elements::Bounds;
    use crate::model::{Element, ElementData, Point};

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
    fn alignment_treats_a_complete_group_as_one_item() {
        let mut scene = scene();
        scene.create_group(&[1, 2]);
        align_selection(&mut scene, &[1, 2, 3], Alignment::Top);

        assert_eq!(scene.element_by_id(1).unwrap().bounds().y, 10.0);
        assert_eq!(scene.element_by_id(2).unwrap().bounds().y, 30.0);
        assert_eq!(scene.element_by_id(3).unwrap().bounds().y, 10.0);
    }
}
