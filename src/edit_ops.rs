use crate::model::elements::{Bounds, Offset};
use crate::model::{
    Alignment, ConnectionEnd, Distribution, Element, ElementId, Group, LayerOrder, Point, Rect,
    Scene,
};
use leptos::{RwSignal, SignalGet, SignalSet, SignalUpdate, SignalWith, SignalWithUntracked};
use std::rc::Rc;

/// Copied elements plus the group structure they carried.
/// Copied elements plus the group structure they carried.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Clipboard {
    pub elements: Vec<Element>,
    /// Groups referencing the original element ids of `elements`.
    pub groups: Vec<Group>,
}

/// Serialized payload for copying/pasting Inskea elements across windows/sessions.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ClipboardPayload {
    pub inskea_clipboard: bool,
    pub elements: Vec<Element>,
    pub groups: Vec<Group>,
}

/// Apply one scene mutation at the standard undo boundary.
pub fn mutate_scene<R>(
    scene: RwSignal<Scene>,
    push_snapshot: &Rc<dyn Fn()>,
    mutation: impl FnOnce(&mut Scene) -> R,
) -> R {
    (push_snapshot)();
    let mut result = None;
    scene.update(|s| {
        result = Some(mutation(s));
    });
    result.expect("mutation ran")
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
        let groups: Vec<Group> = s
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
        elements: els.clone(),
        groups: groups.clone(),
    });

    #[cfg(target_arch = "wasm32")]
    {
        if !els.is_empty() {
            let text_to_write = if els.len() == 1 {
                if let Some(Element::Text(t)) = els.first() {
                    Some(t.wrapped.raw.to_string())
                } else {
                    None
                }
            } else {
                None
            };

            let payload_str = serde_json::to_string(&ClipboardPayload {
                inskea_clipboard: true,
                elements: els,
                groups,
            });

            if let Some(w) = web_sys::window() {
                if let Some(text) = text_to_write {
                    let _ = w.navigator().clipboard().write_text(&text);
                } else if let Ok(json) = payload_str {
                    let _ = w.navigator().clipboard().write_text(&json);
                }
            }
        }
    }
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

/// Insert copies of `elements`, offset by `(dx, dy)`, giving each a fresh id.
/// Groups among the copies are rebuilt and connector bindings are remapped, so
/// a pasted or duplicated shape stays attached to the copies of its targets.
/// Returns the new element ids in insertion order.
pub(crate) fn insert_elements(
    scene: &mut Scene,
    elements: Vec<Element>,
    groups: &[Group],
    dx: f64,
    dy: f64,
) -> Vec<ElementId> {
    let mut new_ids = Vec::new();
    let mut id_map = Vec::new();
    let mut pending_bindings = Vec::new();
    for mut element in elements {
        let old_id = element.id();
        let bindings = take_connector_bindings(&mut element);
        element.offset(dx, dy);
        element.data_mut().id = 0;
        if let Element::Frame(frame) = &mut element {
            frame.frame_order = 0;
            if frame
                .name
                .strip_prefix("Frame ")
                .is_some_and(|rest| rest.trim().parse::<u32>().is_ok())
            {
                frame.name.clear();
            }
        }
        scene.add_element(element);
        let new_id = scene.next_id - 1;
        new_ids.push(new_id);
        id_map.push((old_id, new_id));
        pending_bindings.push((new_id, bindings));
    }
    for (new_id, bindings) in pending_bindings {
        if let Some(mut element) = scene.element_by_id_mut(new_id) {
            restore_connector_bindings(&mut element, bindings, &id_map);
        }
    }
    regroup_clones(scene, groups, &id_map);
    new_ids
}

/// Clone every selected element, offset by 20px, and select the clones.
/// Duplicates keep the group structure of the original selection.
pub fn duplicate_selection(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    push_snapshot: &Rc<dyn Fn()>,
) -> bool {
    let ids = selected_ids.get();
    if ids.is_empty() {
        return false;
    }
    let mut new_ids = Vec::new();
    mutate_scene(scene, push_snapshot, |s| {
        let elements = s.elements_for_ids(&ids);
        let groups = s.groups.clone();
        new_ids = insert_elements(s, elements, &groups, 20.0, 20.0);
    });
    selected_ids.set(new_ids);
    true
}

/// Group the selected elements into one persistent group. Returns true if a group was created.
pub fn group_selection(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    push_snapshot: &Rc<dyn Fn()>,
) -> bool {
    let ids = selected_ids.get();
    if scene.with_untracked(|s| crate::model::selection_item_count(s, &ids)) < 2 {
        return false;
    }
    mutate_scene(scene, push_snapshot, |s| s.create_group(&ids));
    true
}

/// Dissolve every group touching the selection; the members stay in the scene.
/// Returns true if any groups were removed.
pub fn ungroup_selection(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    push_snapshot: &Rc<dyn Fn()>,
) -> bool {
    let ids = selected_ids.get();
    if ids.is_empty() {
        return false;
    }
    let mut removed = false;
    mutate_scene(scene, push_snapshot, |s| {
        removed = !s.remove_groups_containing(&ids).is_empty();
    });
    removed
}

/// Insert elements centred on `target`, restoring groups among them.
pub fn paste_elements(
    scene: RwSignal<Scene>,
    elements: Vec<Element>,
    groups: &[Group],
    selected_ids: RwSignal<Vec<ElementId>>,
    push_snapshot: &Rc<dyn Fn()>,
    target: Point,
) {
    if elements.is_empty() {
        return;
    }
    let bounds = elements
        .iter()
        .fold(None::<Rect>, |acc, el| match acc {
            Some(rect) => Some(rect.union(&el.bounds())),
            None => Some(el.bounds()),
        })
        .expect("elements is non-empty");
    let center = bounds.center();
    let dx = target.x - center.x;
    let dy = target.y - center.y;
    let mut new_ids = Vec::new();
    mutate_scene(scene, push_snapshot, |s| {
        new_ids = insert_elements(s, elements, groups, dx, dy);
    });
    selected_ids.set(new_ids);
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
    paste_elements(scene, els, &groups, selected_ids, push_snapshot, target);
}

/// Create a new text element from raw text, sized and centered at `target`.
pub fn paste_text_to_scene(
    scene: &mut Scene,
    text: &str,
    target: Point,
    default_style: crate::model::ElementStyle,
) -> Option<ElementId> {
    let text = text.replace("\r\n", "\n");
    if text.trim().is_empty() {
        return None;
    }
    let fs = default_style
        .font_size
        .max(crate::model::elements::text::MIN_FONT_SIZE);
    let natural_w = crate::model::elements::text::natural_line_width(
        &text,
        fs,
        crate::model::elements::text::FONT_FAMILY,
    );
    let width = if natural_w > 800.0 { 800.0 } else { natural_w };

    let mut data = crate::model::ElementData::new(0);
    data.world_point = target;
    data.width = width;
    data.style = default_style;

    let mut text_elem = crate::model::Text {
        data,
        wrapped: crate::model::elements::text::WrappedText::new(&text),
        align: crate::model::TextAlign::Left,
    };
    text_elem.recalc();

    // Center around target point
    text_elem.data.world_point = Point::new(
        target.x - text_elem.data.width / 2.0,
        target.y - text_elem.data.height / 2.0,
    );

    scene.add_element(Element::Text(text_elem));
    Some(scene.next_id - 1)
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

/// Align selected elements or complete groups against the outer selection bounds or enclosing frame.
pub fn align_selection(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    alignment: Alignment,
    push_snapshot: &Rc<dyn Fn()>,
) -> bool {
    let ids = selected_ids.get();
    if ids.is_empty() {
        return false;
    }
    let can = scene.with_untracked(|s| crate::model::can_align_selection(s, &ids));
    if !can {
        return false;
    }
    mutate_scene(scene, push_snapshot, |s| {
        crate::model::align_selection(s, &ids, alignment);
    });
    true
}

/// Distribute selected elements or complete groups with equal gaps.
pub fn distribute_selection(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    distribution: Distribution,
    push_snapshot: &Rc<dyn Fn()>,
) -> bool {
    let ids = selected_ids.get();
    if scene.with_untracked(|s| crate::model::selection_item_count(s, &ids)) < 3 {
        return false;
    }
    mutate_scene(scene, push_snapshot, |s| {
        crate::model::distribute_selection(s, &ids, distribution);
    });
    true
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

    #[test]
    fn paste_text_creates_element_centered() {
        let mut scene = Scene::new();
        let target = Point::new(100.0, 200.0);
        let id = paste_text_to_scene(
            &mut scene,
            "Hello World\nLine 2",
            target,
            crate::model::ElementStyle::default(),
        )
        .expect("text should be pasted");

        let el = scene.element_by_id(id).expect("element exists");
        let Element::Text(text) = el else {
            panic!("expected text element");
        };
        assert_eq!(text.wrapped.raw.as_str(), "Hello World\nLine 2");
        let bounds = text.bounds();
        let center = bounds.center();
        assert!((center.x - target.x).abs() < 1.0);
        assert!((center.y - target.y).abs() < 1.0);
    }

    #[test]
    fn paste_empty_or_whitespace_text_returns_none() {
        let mut scene = Scene::new();
        let target = Point::new(0.0, 0.0);
        assert!(paste_text_to_scene(&mut scene, "", target, Default::default()).is_none());
        assert!(paste_text_to_scene(&mut scene, "   \n\t  ", target, Default::default()).is_none());
    }

    #[test]
    fn clipboard_payload_serialization_roundtrip() {
        let payload = ClipboardPayload {
            inskea_clipboard: true,
            elements: vec![],
            groups: vec![],
        };
        let json = serde_json::to_string(&payload).unwrap();
        let decoded: ClipboardPayload = serde_json::from_str(&json).unwrap();
        assert!(decoded.inskea_clipboard);
        assert!(decoded.elements.is_empty());
    }
}
