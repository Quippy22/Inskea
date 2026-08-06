use crate::model::elements::{Bounds, Offset};
use crate::model::{Element, ElementId, Point, Rect, Scene};
use leptos::{RwSignal, SignalGet, SignalSet, SignalUpdate, SignalWith};
use std::rc::Rc;

/// Store copies of the currently selected elements in the clipboard.
pub fn copy_selection(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    clipboard: RwSignal<Vec<Element>>,
) {
    let ids = selected_ids.get();
    let els = scene.with(|s| {
        s.elements()
            .iter()
            .filter(|e| ids.contains(&e.id()))
            .cloned()
            .collect::<Vec<_>>()
    });
    clipboard.set(els);
}

/// Clone every selected element, offset by 20px, and select the clones.
pub fn duplicate_selection(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    push_snapshot: &Rc<dyn Fn()>,
) {
    let ids = selected_ids.get();
    if ids.is_empty() {
        return;
    }
    (push_snapshot)();
    let mut new_ids = Vec::new();
    scene.update(|s| {
        let elements = s.elements().to_vec();
        let was_group = s.selection_is_group(&ids);
        for id in &ids {
            if let Some(el) = elements.iter().find(|e| e.id() == *id) {
                let mut clone = el.clone();
                clone.offset(20.0, 20.0);
                clone.data_mut().id = 0;
                s.add_element(clone);
                new_ids.push(s.next_id - 1);
            }
        }
        if was_group {
            s.create_group(&new_ids);
        }
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
    if ids.len() < 2 {
        return;
    }
    (push_snapshot)();
    scene.update(|s| s.create_group(&ids));
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
    (push_snapshot)();
    scene.update(|s| {
        s.remove_groups_containing(&ids);
    });
}

/// Insert all clipboard elements centred on the cursor and select the new elements.
pub fn paste_clipboard(
    scene: RwSignal<Scene>,
    clipboard: RwSignal<Vec<Element>>,
    selected_ids: RwSignal<Vec<ElementId>>,
    push_snapshot: &Rc<dyn Fn()>,
    cursor_world: RwSignal<Point>,
) {
    let els = clipboard.get();
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
    (push_snapshot)();
    let mut new_ids = Vec::new();
    scene.update(|s| {
        for mut el in els {
            el.offset(dx, dy);
            el.data_mut().id = 0;
            s.add_element(el);
            new_ids.push(s.next_id - 1);
        }
    });
    selected_ids.set(new_ids);
}
