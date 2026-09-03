mod color;
pub mod elements;
pub mod interaction;
pub mod point;
pub mod rect;
pub mod resize;
pub mod selection;

pub use color::Color;
pub use elements::*;
pub use point::Point;
pub use rect::Rect;

/// A persistent group: a set of element ids that stay selected, move, and act together.
pub type GroupId = u64;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Group {
    /// Stable identity for this group, independent of its member elements.
    #[serde(default)]
    pub id: GroupId,
    /// Member element ids, in the order they were grouped.
    pub members: Vec<ElementId>,
}

fn default_next_group_id() -> GroupId {
    1
}

/// The single source of truth for everything on the canvas.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Scene {
    pub(crate) elements: Vec<Element>,
    pub(crate) next_id: u64,
    #[serde(default)]
    pub(crate) groups: Vec<Group>,
    #[serde(default = "default_next_group_id")]
    pub(crate) next_group_id: GroupId,
}

impl Scene {
    /// Creates an empty scene with no elements (IDs start at 1).
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            next_id: 1,
            groups: Vec::new(),
            next_group_id: 1,
        }
    }

    pub fn elements(&self) -> &[Element] {
        &self.elements
    }

    pub fn elements_mut(&mut self) -> &mut Vec<Element> {
        &mut self.elements
    }

    pub fn element_by_id(&self, id: ElementId) -> Option<&Element> {
        self.elements.iter().find(|e| e.id() == id)
    }

    pub fn element_by_id_mut(&mut self, id: ElementId) -> Option<&mut Element> {
        self.elements.iter_mut().find(|e| e.id() == id)
    }

    pub fn remove_by_id(&mut self, id: ElementId) {
        self.elements.retain(|e| e.id() != id);
        for group in &mut self.groups {
            group.members.retain(|m| *m != id);
        }
        self.normalize_groups();
    }

    /// Assign IDs to legacy or duplicated groups loaded from older files.
    pub fn ensure_group_ids(&mut self) {
        let mut next = self.next_group_id.max(1);
        let mut used = Vec::new();
        for group in &mut self.groups {
            if group.id == 0 || used.contains(&group.id) {
                group.id = next;
                next += 1;
            }
            used.push(group.id);
            next = next.max(group.id.saturating_add(1));
        }
        self.next_group_id = next;
    }

    /// Repair group membership after loading or a scene mutation.
    ///
    /// Groups are flat, so the first group in scene order owns a member when
    /// malformed data contains overlaps. Member order follows canvas order
    /// after normalization, making serialized scenes deterministic.
    pub fn normalize_groups(&mut self) {
        self.ensure_group_ids();
        let element_ids: Vec<_> = self.elements.iter().map(Element::id).collect();
        let mut claimed = Vec::new();
        let mut groups = Vec::with_capacity(self.groups.len());

        for mut group in std::mem::take(&mut self.groups) {
            group
                .members
                .retain(|id| element_ids.contains(id) && !claimed.contains(id));
            group.members.sort_by_key(|id| {
                element_ids
                    .iter()
                    .position(|element_id| element_id == id)
                    .unwrap_or(usize::MAX)
            });
            group.members.dedup();
            if group.members.len() >= 2 {
                claimed.extend(group.members.iter().copied());
                groups.push(group);
            }
        }

        self.groups = groups;
    }

    /// The group that contains `id`, if any.
    pub fn group_containing(&self, id: ElementId) -> Option<&Group> {
        self.groups.iter().find(|g| g.members.contains(&id))
    }

    /// Expand `ids` to full groups: every id belonging to a group brings all its members.
    pub fn expand_to_groups(&self, ids: &[ElementId]) -> Vec<ElementId> {
        let mut out: Vec<ElementId> = Vec::new();
        for id in ids {
            if let Some(g) = self.group_containing(*id) {
                for m in &g.members {
                    if !out.contains(m) {
                        out.push(*m);
                    }
                }
            } else if !out.contains(id) {
                out.push(*id);
            }
        }
        out
    }

    /// True if the selection is exactly the members of one group.
    pub fn selection_is_group(&self, ids: &[ElementId]) -> bool {
        if ids.is_empty() {
            return false;
        }
        match self.group_containing(ids[0]) {
            Some(g) => ids.len() == g.members.len() && ids.iter().all(|id| g.members.contains(id)),
            None => false,
        }
    }

    /// Group `ids` into one group, dissolving any existing groups that overlap the members.
    pub fn create_group(&mut self, ids: &[ElementId]) {
        self.normalize_groups();
        let members = self.expand_to_groups(ids);
        if members.len() < 2 {
            return;
        }
        self.groups
            .retain(|g| !g.members.iter().any(|m| members.contains(m)));
        let id = self.next_group_id;
        self.next_group_id += 1;
        self.groups.push(Group { id, members });
    }

    /// Remove every group containing any of `ids`, returning the affected member ids.
    pub fn remove_groups_containing(&mut self, ids: &[ElementId]) -> Vec<ElementId> {
        let mut removed = Vec::new();
        self.groups.retain(|g| {
            let hit = ids.iter().any(|id| g.members.contains(id));
            if hit {
                removed.extend(g.members.iter().copied());
            }
            !hit
        });
        self.normalize_groups();
        removed
    }

    /// Adds an element to the scene, assigning it a unique ID.
    ///
    /// The element's previous ID is overwritten.
    pub fn add_element(&mut self, mut element: Element) {
        element.data_mut().id = self.next_id;
        self.next_id += 1;
        self.elements.push(element);
    }

    /// Move element to the front of the z-order (top of canvas).
    pub fn bring_to_front(&mut self, id: ElementId) {
        if let Some(idx) = self.elements.iter().position(|e| e.id() == id) {
            let el = self.elements.remove(idx);
            self.elements.push(el);
        }
    }

    /// Move element to the back of the z-order (bottom of canvas).
    pub fn send_to_back(&mut self, id: ElementId) {
        if let Some(idx) = self.elements.iter().position(|e| e.id() == id) {
            let el = self.elements.remove(idx);
            self.elements.insert(0, el);
        }
    }

    /// Move element one step toward the front.
    pub fn bring_forward(&mut self, id: ElementId) {
        let len = self.elements.len();
        if len < 2 {
            return;
        }
        if let Some(idx) = self.elements.iter().position(|e| e.id() == id) {
            if idx < len - 1 {
                self.elements.swap(idx, idx + 1);
            }
        }
    }

    /// Move element one step toward the back.
    pub fn send_backward(&mut self, id: ElementId) {
        if let Some(idx) = self.elements.iter().position(|e| e.id() == id) {
            if idx > 0 {
                self.elements.swap(idx, idx - 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::elements::rect::Rectangle;

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
    fn expand_to_groups_expands_members() {
        let mut s = scene_with_three();
        s.create_group(&[1, 2]);
        assert_eq!(s.expand_to_groups(&[2]), vec![1, 2]);
        assert_eq!(s.expand_to_groups(&[1, 3]), vec![1, 2, 3]);
    }

    #[test]
    fn create_group_needs_two_members() {
        let mut s = scene_with_three();
        s.create_group(&[1]);
        assert!(s.groups.is_empty());
        assert!(!s.selection_is_group(&[1]));
    }

    #[test]
    fn create_group_dissolves_overlapping_groups() {
        let mut s = scene_with_three();
        s.create_group(&[1, 2]);
        s.create_group(&[2, 3]);
        assert_eq!(s.groups.len(), 1);
        assert_eq!(s.groups[0].members, vec![1, 2, 3]);
        assert_eq!(s.groups[0].id, 2);
        assert!(s.selection_is_group(&[1, 2, 3]));
    }

    #[test]
    fn selection_is_group_requires_the_complete_group() {
        let mut s = scene_with_three();
        s.create_group(&[1, 2]);

        assert!(!s.selection_is_group(&[1]));
        assert!(s.selection_is_group(&[1, 2]));
        assert!(s.selection_is_group(&[2, 1]));
    }

    #[test]
    fn group_ids_are_allocated_monotonically() {
        let mut s = scene_with_three();
        s.create_group(&[1, 2]);
        s.remove_groups_containing(&[1]);
        s.create_group(&[2, 3]);

        assert_eq!(s.groups.iter().map(|g| g.id).collect::<Vec<_>>(), vec![2]);
    }

    #[test]
    fn ensure_group_ids_repairs_legacy_and_duplicate_ids() {
        let mut s = scene_with_three();
        s.groups = vec![
            Group {
                id: 0,
                members: vec![1, 2],
            },
            Group {
                id: 1,
                members: vec![2, 3],
            },
        ];

        s.ensure_group_ids();

        assert_eq!(s.groups[0].id, 1);
        assert_eq!(s.groups[1].id, 2);
        assert_eq!(s.next_group_id, 3);
    }

    #[test]
    fn normalize_groups_removes_invalid_members_and_overlaps() {
        let mut s = scene_with_three();
        s.groups = vec![
            Group {
                id: 1,
                members: vec![3, 1, 1, 99],
            },
            Group {
                id: 2,
                members: vec![2, 3],
            },
        ];

        s.normalize_groups();

        assert_eq!(s.groups.len(), 1);
        assert_eq!(s.groups[0].members, vec![1, 3]);
    }

    #[test]
    fn ungroup_removes_membership_only() {
        let mut s = scene_with_three();
        s.create_group(&[1, 2]);
        let removed = s.remove_groups_containing(&[2]);
        assert_eq!(removed, vec![1, 2]);
        assert!(s.groups.is_empty());
        assert_eq!(s.elements().len(), 3);
    }

    #[test]
    fn removing_member_drops_undersized_group() {
        let mut s = scene_with_three();
        s.create_group(&[1, 2]);
        s.remove_by_id(1);
        assert!(s.groups.is_empty());
        s.remove_by_id(2);
        assert_eq!(s.elements().len(), 1);
    }

    #[test]
    fn old_scenes_default_to_no_groups() {
        let legacy = r#"{"elements":[],"next_id":1}"#;
        let s: Scene = serde_json::from_str(legacy).unwrap();
        assert!(s.groups.is_empty());
    }
}
