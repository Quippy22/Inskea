use crate::model::elements::{Bounds, Offset};
use crate::model::{
    Alignment, ConnectionEnd, Distribution, Element, ElementId, Group, LayerOrder, Point, Rect,
    Scene,
};
use leptos::{RwSignal, SignalGet, SignalSet, SignalUpdate, SignalWith, SignalWithUntracked};
use std::rc::Rc;

/// Copied elements plus the group structure they carried.
#[derive(Clone, Debug, Default)]
pub struct Clipboard {
    pub elements: Vec<Element>,
    /// Groups referencing the original element ids of `elements`.
    pub groups: Vec<Group>,
}

/// Apply one scene mutation at the standard undo boundary.
pub fn mutate_scene(
    scene: RwSignal<Scene>,
    push_snapshot: &Rc<dyn Fn()>,
    mutation: impl FnOnce(&mut Scene),
) {
    (push_snapshot)();
    scene.update(mutation);
}

/// Store copies of the currently selected elements in the clipboard,
/// keeping the group structure so paste can regroup the copies.
pub fn copy_selection(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    clipboard: RwSignal<Clipboard>,
) {
    let ids = selected_ids.get();
    let (els, groups) = scene.with(|s| {
        let els = s.elements_for_ids(&ids);
        let groups = s
            .groups
            .iter()
            .filter_map(|g| {
                let members = g
                    .members
                    .iter()
                    .filter(|m| ids.contains(m))
                    .copied()
                    .collect::<Vec<_>>();
                (members.len() >= 2).then_some(Group { id: g.id, members })
            })
            .collect();
        (els, groups)
    });
    clipboard.set(Clipboard {
        elements: els,
        groups,
    });
}

/// Remap a copied/seeded group's original member ids to their new clones.
fn regroup_clones(scene: &mut Scene, groups: &[Group], id_map: &[(ElementId, ElementId)]) {
    for g in groups {
        let members = g
            .members
            .iter()
            .filter_map(|m| id_map.iter().find(|(old, _)| old == m).map(|&(_, new)| new))
            .collect::<Vec<_>>();
        if members.len() >= 2 {
            scene.create_group(&members);
        }
    }
}

type ConnectorBindings = (Option<ConnectionEnd>, Option<ConnectionEnd>);

/// Take a copied connector's bindings before it is offset. Bound endpoints do
/// not move with `offset`, so detaching first lets the whole copy shift and
/// keeps the original targets available for remapping.
fn take_connector_bindings(element: &mut Element) -> ConnectorBindings {
    let Element::Connector(connector) = element else {
        return (None, None);
    };
    let bindings = (connector.start_binding, connector.end_binding);
    connector.start_binding = None;
    connector.end_binding = None;
    bindings
}

/// Point copied connector bindings at the cloned shapes. A binding whose
/// shape was not copied stays detached, so the clone moves on its own.
fn restore_connector_bindings(
    element: &mut Element,
    bindings: ConnectorBindings,
    id_map: &[(ElementId, ElementId)],
) {
    let Element::Connector(connector) = element else {
        return;
    };
    let remap = |binding: ConnectionEnd| {
        id_map
            .iter()
            .find(|(old, _)| *old == binding.element)
            .map(|&(_, new)| ConnectionEnd {
                element: new,
                point: binding.point,
            })
    };
    connector.start_binding = bindings.0.and_then(remap);
    connector.end_binding = bindings.1.and_then(remap);
}

/// Clone every selected element, offset by 20px, and select the clones.
/// Duplicates keep the group structure of the original selection.
pub fn duplicate_selection(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    push_snapshot: &Rc<dyn Fn()>,
) {
    let ids = selected_ids.get();
    if ids.is_empty() {
        return;
    }
    let mut new_ids = Vec::new();
    let mut id_map = Vec::new();
    let mut pending_bindings = Vec::new();
    mutate_scene(scene, push_snapshot, |s| {
        let elements = s.elements_for_ids(&ids);
        let groups = s.groups.clone();
        for id in &ids {
            if let Some(el) = elements.iter().find(|e| e.id() == *id) {
                let mut clone = el.clone();
                let bindings = take_connector_bindings(&mut clone);
                clone.offset(20.0, 20.0);
                clone.data_mut().id = 0;
                s.add_element(clone);
                let new_id = s.next_id - 1;
                new_ids.push(new_id);
                id_map.push((*id, new_id));
                pending_bindings.push((new_id, bindings));
            }
        }
        for (new_id, bindings) in pending_bindings {
            if let Some(mut element) = s.element_by_id_mut(new_id) {
                restore_connector_bindings(&mut element, bindings, &id_map);
            }
        }
        regroup_clones(s, &groups, &id_map);
    });
    selected_ids.set(new_ids);
}

/// Group the selected elements into one persistent group.
pub fn group_selection(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    push_snapshot: &Rc<dyn Fn()>,
) {
    let ids = selected_ids.get();
    if scene.with_untracked(|s| crate::model::selection_item_count(s, &ids)) < 2 {
        return;
    }
    mutate_scene(scene, push_snapshot, |s| s.create_group(&ids));
}

/// Dissolve every group touching the selection; the members stay in the scene.
pub fn ungroup_selection(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    push_snapshot: &Rc<dyn Fn()>,
) {
    let ids = selected_ids.get();
    if ids.is_empty() {
        return;
    }
    mutate_scene(scene, push_snapshot, |s| {
        s.remove_groups_containing(&ids);
    });
}

/// Insert all clipboard elements centred on the cursor and select the new
/// elements. Pasting a copied group restores its grouping.
pub fn paste_clipboard(
    scene: RwSignal<Scene>,
    clipboard: RwSignal<Clipboard>,
    selected_ids: RwSignal<Vec<ElementId>>,
    push_snapshot: &Rc<dyn Fn()>,
    cursor_world: RwSignal<Point>,
) {
    let Clipboard {
        elements: els,
        groups,
    } = clipboard.get();
    if els.is_empty() {
        return;
    }
    let target = cursor_world.get();
    let bounds = els
        .iter()
        .fold(None::<Rect>, |acc, el| match acc {
            Some(rect) => Some(rect.union(&el.bounds())),
            None => Some(el.bounds()),
        })
        .expect("clipboard is non-empty");
    let center = bounds.center();
    let dx = target.x - center.x;
    let dy = target.y - center.y;
    let mut new_ids = Vec::new();
    let mut id_map = Vec::new();
    let mut pending_bindings = Vec::new();
    mutate_scene(scene, push_snapshot, |s| {
        for mut el in els {
            let old_id = el.id();
            let bindings = take_connector_bindings(&mut el);
            el.offset(dx, dy);
            el.data_mut().id = 0;
            s.add_element(el);
            let new_id = s.next_id - 1;
            new_ids.push(new_id);
            id_map.push((old_id, new_id));
            pending_bindings.push((new_id, bindings));
        }
        for (new_id, bindings) in pending_bindings {
            if let Some(mut element) = s.element_by_id_mut(new_id) {
                restore_connector_bindings(&mut element, bindings, &id_map);
            }
        }
        regroup_clones(s, &groups, &id_map);
    });
    selected_ids.set(new_ids);
}

/// Reorder the selection as one z-order block, including complete groups.
pub fn reorder_selection(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    direction: LayerOrder,
    push_snapshot: &Rc<dyn Fn()>,
) {
    let ids = selected_ids.get();
    if ids.is_empty() {
        return;
    }
    mutate_scene(scene, push_snapshot, |s| {
        s.reorder_selection(&ids, direction);
    });
}

/// Align selected elements or complete groups against the outer selection bounds.
pub fn align_selection(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    alignment: Alignment,
    push_snapshot: &Rc<dyn Fn()>,
) {
    let ids = selected_ids.get();
    if ids.len() < 2 {
        return;
    }
    mutate_scene(scene, push_snapshot, |s| {
        crate::model::align_selection(s, &ids, alignment);
    });
}

/// Distribute selected elements or complete groups with equal gaps.
pub fn distribute_selection(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    distribution: Distribution,
    push_snapshot: &Rc<dyn Fn()>,
) {
    let ids = selected_ids.get();
    if scene.with_untracked(|s| crate::model::selection_item_count(s, &ids)) < 3 {
        return;
    }
    mutate_scene(scene, push_snapshot, |s| {
        crate::model::distribute_selection(s, &ids, distribution);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::elements::rect::Rectangle;
    use crate::model::{Connector, ConnectorVariant, ElementData, LineStyle};

    fn scene_with_three() -> Scene {
        let mut s = Scene::new();
        for _ in 0..3 {
            s.add_element(Element::Rectangle(Rectangle {
                data: ElementData::new(0),
            }));
        }
        s
    }

    #[test]
    fn regroup_remaps_full_group() {
        let mut s = scene_with_three();
        let groups = vec![Group {
            id: 1,
            members: vec![1, 2],
        }];
        let id_map = vec![(1, 4), (2, 5)];
        regroup_clones(&mut s, &groups, &id_map);
        assert_eq!(s.groups.len(), 1);
        assert_eq!(s.groups[0].members, vec![4, 5]);
    }

    #[test]
    fn regroup_skips_unmapped_members() {
        let mut s = scene_with_three();
        let groups = vec![Group {
            id: 1,
            members: vec![2, 3],
        }];
        let id_map = vec![(2, 4)];
        regroup_clones(&mut s, &groups, &id_map);
        assert!(s.groups.is_empty());
    }

    #[test]
    fn copied_connector_bindings_follow_cloned_shapes() {
        let mut element = Element::Connector(Connector {
            data: ElementData::new(1),
            points: vec![Point::new(0.0, 0.0), Point::new(50.0, 0.0)].into_boxed_slice(),
            line_style: LineStyle::default(),
            variant: ConnectorVariant::Direct,
            start_binding: Some(ConnectionEnd::new(1, 2)),
            end_binding: Some(ConnectionEnd::new(2, 6)),
            curve_bend: None,
            elbow_split: None,
            elbow_bends: Vec::new(),
        });

        let bindings = take_connector_bindings(&mut element);
        if let Element::Connector(connector) = &element {
            assert_eq!(connector.start_binding, None);
            assert_eq!(connector.end_binding, None);
        }
        // With the bindings detached, the copy offsets as one piece.
        element.offset(20.0, 20.0);
        let Element::Connector(connector) = &element else {
            panic!("expected connector");
        };
        assert_eq!(connector.points[0], Point::new(20.0, 20.0));
        assert_eq!(connector.points[1], Point::new(70.0, 20.0));

        restore_connector_bindings(&mut element, bindings, &[(1, 10)]);
        let Element::Connector(connector) = &element else {
            panic!("expected connector");
        };
        assert_eq!(
            connector.start_binding,
            Some(ConnectionEnd {
                element: 10,
                point: 2
            })
        );
        // Shape 2 was not copied, so that end stays free.
        assert_eq!(connector.end_binding, None);
    }

    #[test]
    fn regroup_partial_group_keeps_paste_pair() {
        let mut s = scene_with_three();
        s.create_group(&[1, 2, 3]);
        let groups = s.groups.clone();
        let id_map = vec![(1, 4), (2, 5)];
        regroup_clones(&mut s, &groups, &id_map);
        assert_eq!(s.groups.len(), 2);
        let new_group = s.groups.iter().find(|g| !g.members.contains(&1)).unwrap();
        assert_eq!(new_group.members, vec![4, 5]);
    }
}
