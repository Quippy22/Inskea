use super::{ElementId, Scene};

/// Keep only existing IDs, preserving the caller's order and removing duplicates.
pub fn normalize_ids(scene: &Scene, ids: &[ElementId]) -> Vec<ElementId> {
    ids.iter()
        .copied()
        .filter(|id| scene.element_by_id(*id).is_some())
        .fold(Vec::new(), |mut out, id| {
            if !out.contains(&id) {
                out.push(id);
            }
            out
        })
}

/// Toggle a set of IDs in the current selection without introducing duplicates.
pub fn toggle_ids(current: &[ElementId], ids: &[ElementId]) -> Vec<ElementId> {
    let mut out = current.to_vec();
    for id in ids {
        if let Some(index) = out.iter().position(|selected| selected == id) {
            out.remove(index);
        } else {
            out.push(*id);
        }
    }
    out
}

/// Expand group members and normalize the result in one operation.
pub fn expand_ids(scene: &Scene, ids: &[ElementId]) -> Vec<ElementId> {
    normalize_ids(scene, &scene.expand_to_groups(ids))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::elements::rect::Rectangle;
    use crate::model::ElementData;

    fn scene_with_three() -> Scene {
        let mut scene = Scene::new();
        for _ in 0..3 {
            scene.add_element(crate::model::Element::Rectangle(Rectangle {
                data: ElementData::new(0),
            }));
        }
        scene
    }

    #[test]
    fn normalize_ids_removes_missing_and_duplicate_ids() {
        let scene = scene_with_three();
        assert_eq!(normalize_ids(&scene, &[3, 99, 1, 3, 2]), vec![3, 1, 2]);
    }

    #[test]
    fn toggle_ids_preserves_order_and_toggles_each_id() {
        assert_eq!(toggle_ids(&[1, 3], &[3, 2]), vec![1, 2]);
    }

    #[test]
    fn expand_ids_returns_a_normalized_group_selection() {
        let mut scene = scene_with_three();
        scene.create_group(&[1, 2]);
        assert_eq!(expand_ids(&scene, &[2, 99, 2]), vec![1, 2]);
    }
}
