pub mod alignment;
mod color;
pub mod elements;
pub mod interaction;
pub mod point;
pub mod rect;
pub mod resize;
pub mod selection;

pub use alignment::{
    align_selection, distribute_selection, selection_item_count, Alignment, Distribution,
};
pub use color::Color;
pub use elements::*;
pub use point::Point;
pub use rect::Rect;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

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
#[derive(Clone, Debug)]
pub struct Scene {
    chunks: HashMap<(i64, i64), SceneChunk>,
    locations: HashMap<ElementId, (i64, i64)>,
    oversized: Vec<ElementId>,
    next_order: u64,
    pub(crate) next_id: u64,
    pub(crate) groups: Vec<Group>,
    pub(crate) next_group_id: GroupId,
}

#[derive(Clone, Debug)]
struct SceneChunk {
    /// Absolute world-space origin; element geometry is stored relative to it.
    origin: Point,
    elements: Vec<StoredElement>,
}

const CHUNK_SIZE: f64 = 1024.0;

fn chunk_coord(value: f64) -> i64 {
    if value.is_nan() {
        0
    } else if value == f64::INFINITY {
        i64::MAX
    } else if value == f64::NEG_INFINITY {
        i64::MIN
    } else {
        (value / CHUNK_SIZE)
            .floor()
            .clamp(i64::MIN as f64, i64::MAX as f64) as i64
    }
}

fn chunk_of(point: Point) -> (i64, i64) {
    (chunk_coord(point.x), chunk_coord(point.y))
}

fn chunk_origin((x, y): (i64, i64)) -> Point {
    Point::new(x as f64 * CHUNK_SIZE, y as f64 * CHUNK_SIZE)
}

fn chunk_range(rect: Rect) -> (i64, i64, i64, i64) {
    (
        chunk_coord(rect.x.min(rect.right())),
        chunk_coord(rect.y.min(rect.bottom())),
        chunk_coord(rect.x.max(rect.right())),
        chunk_coord(rect.y.max(rect.bottom())),
    )
}

fn is_oversized(bounds: Rect) -> bool {
    bounds.w.abs() > CHUNK_SIZE || bounds.h.abs() > CHUNK_SIZE
}

fn chunk_count(x0: i64, y0: i64, x1: i64, y1: i64) -> i64 {
    x1.saturating_sub(x0)
        .saturating_add(1)
        .saturating_mul(y1.saturating_sub(y0).saturating_add(1))
}

fn element_bounds(element: &Element) -> Rect {
    crate::model::interaction::visual_envelope(element)
        .padded(element.data().style.stroke_width.max(0.0))
}

#[derive(Clone, Debug)]
struct StoredElement {
    order: u64,
    element: Element,
}

#[derive(Serialize, Deserialize)]
struct SceneFile {
    elements: Vec<Element>,
    next_id: u64,
    #[serde(default)]
    groups: Vec<Group>,
    #[serde(default = "default_next_group_id")]
    next_group_id: GroupId,
}

impl Serialize for Scene {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SceneFile {
            elements: self.elements(),
            next_id: self.next_id,
            groups: self.groups.clone(),
            next_group_id: self.next_group_id,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Scene {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let file = SceneFile::deserialize(deserializer)?;
        Ok(Self::from_file(file))
    }
}

impl PartialEq for Scene {
    fn eq(&self, other: &Self) -> bool {
        self.elements() == other.elements()
            && self.next_id == other.next_id
            && self.groups == other.groups
            && self.next_group_id == other.next_group_id
    }
}

/// Mutable absolute-coordinate view of one chunk-stored element.
pub struct ElementMut<'a> {
    scene: &'a mut Scene,
    id: ElementId,
    order: u64,
    element: Option<Element>,
}

impl Deref for ElementMut<'_> {
    type Target = Element;
    fn deref(&self) -> &Element {
        self.element.as_ref().expect("element guard is live")
    }
}

impl DerefMut for ElementMut<'_> {
    fn deref_mut(&mut self) -> &mut Element {
        self.element.as_mut().expect("element guard is live")
    }
}

impl Drop for ElementMut<'_> {
    fn drop(&mut self) {
        if let Some(mut element) = self.element.take() {
            element.data_mut().id = self.id;
            self.scene.put_element(element, self.order);
        }
    }
}

impl Scene {
    /// Creates an empty scene with no elements (IDs start at 1).
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            locations: HashMap::new(),
            oversized: Vec::new(),
            next_order: 0,
            next_id: 1,
            groups: Vec::new(),
            next_group_id: 1,
        }
    }

    fn from_file(file: SceneFile) -> Self {
        let mut scene = Self::new();
        scene.next_id = file.next_id;
        scene.groups = file.groups;
        scene.next_group_id = file.next_group_id;
        scene.replace_all(file.elements);
        scene
    }

    /// Return elements in canvas z-order with absolute world-space geometry.
    pub fn elements(&self) -> Vec<Element> {
        let mut ordered: Vec<_> = self
            .chunks
            .values()
            .flat_map(|chunk| {
                chunk.elements.iter().map(move |stored| {
                    let mut element = stored.element.clone();
                    element.offset(chunk.origin.x, chunk.origin.y);
                    (stored.order, element)
                })
            })
            .collect();
        ordered.sort_by_key(|(order, _)| *order);
        ordered.into_iter().map(|(_, element)| element).collect()
    }

    /// Return a z-ordered subset without materializing unrelated chunks.
    pub fn elements_for_ids(&self, ids: &[ElementId]) -> Vec<Element> {
        let mut selected: Vec<_> = ids
            .iter()
            .filter_map(|id| {
                let order = self.element_order(*id)?;
                Some((order, self.element_by_id(*id)?))
            })
            .collect();
        selected.sort_by_key(|(order, _)| *order);
        selected.into_iter().map(|(_, element)| element).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.locations.is_empty()
    }

    /// Materialize just the elements intersecting the view from their spatial
    /// chunks; element geometry is stored relative to each chunk origin.
    pub fn elements_in_view(&self, view: Rect) -> Vec<Element> {
        // Elements no larger than one chunk are stored by their center. Search
        // one neighboring chunk in each direction to catch shapes crossing a
        // chunk edge; larger paths are kept in the small `oversized` list.
        let (x0, y0, x1, y1) = chunk_range(view);
        let (x0, y0, x1, y1) = (
            x0.saturating_sub(1),
            y0.saturating_sub(1),
            x1.saturating_add(1),
            y1.saturating_add(1),
        );
        if chunk_count(x0, y0, x1, y1) > 4096 {
            return self.elements();
        }
        let mut ids = self.oversized.clone();
        for x in x0..=x1 {
            for y in y0..=y1 {
                if let Some(chunk) = self.chunks.get(&(x, y)) {
                    ids.extend(chunk.elements.iter().map(|stored| stored.element.id()));
                }
            }
        }
        ids.sort_unstable();
        ids.dedup();

        let mut visible = Vec::new();
        for id in ids {
            if let Some(element) = self.element_by_id(id) {
                let bounds = element_bounds(&element);
                if bounds.x <= view.right()
                    && bounds.right() >= view.x
                    && bounds.y <= view.bottom()
                    && bounds.bottom() >= view.y
                {
                    let order = self.element_order(id).unwrap_or(u64::MAX);
                    visible.push((order, element));
                }
            }
        }
        visible.sort_by_key(|(order, _)| *order);
        visible.into_iter().map(|(_, element)| element).collect()
    }

    pub fn element_by_id(&self, id: ElementId) -> Option<Element> {
        let key = *self.locations.get(&id)?;
        let chunk = self.chunks.get(&key)?;
        let mut element = chunk
            .elements
            .iter()
            .find(|stored| stored.element.id() == id)?
            .element
            .clone();
        element.offset(chunk.origin.x, chunk.origin.y);
        Some(element)
    }

    pub fn element_by_id_mut(&mut self, id: ElementId) -> Option<ElementMut<'_>> {
        let order = self.element_order(id)?;
        let element = self.take_element(id)?;
        Some(ElementMut {
            scene: self,
            id,
            order,
            element: Some(element),
        })
    }

    fn element_order(&self, id: ElementId) -> Option<u64> {
        let key = *self.locations.get(&id)?;
        self.chunks
            .get(&key)?
            .elements
            .iter()
            .find(|stored| stored.element.id() == id)
            .map(|stored| stored.order)
    }

    fn take_element(&mut self, id: ElementId) -> Option<Element> {
        let key = self.locations.remove(&id)?;
        let (stored, empty) = {
            let chunk = self.chunks.get_mut(&key)?;
            let index = chunk
                .elements
                .iter()
                .position(|stored| stored.element.id() == id)?;
            let stored = chunk.elements.remove(index);
            (stored, chunk.elements.is_empty())
        };
        if empty {
            self.chunks.remove(&key);
        }
        let origin = chunk_origin(key);
        let mut element = stored.element;
        element.offset(origin.x, origin.y);
        if is_oversized(element_bounds(&element)) {
            self.oversized.retain(|candidate| *candidate != id);
        }
        Some(element)
    }

    fn put_element(&mut self, mut element: Element, order: u64) {
        let id = element.id();
        if self.locations.contains_key(&id) {
            let _ = self.take_element(id);
        }
        let bounds = element_bounds(&element);
        let home = chunk_of(bounds.center());
        let origin = chunk_origin(home);
        element.offset(-origin.x, -origin.y);
        self.chunks
            .entry(home)
            .or_insert_with(|| SceneChunk {
                origin,
                elements: Vec::new(),
            })
            .elements
            .push(StoredElement { order, element });
        self.locations.insert(id, home);
        self.next_order = self.next_order.max(order.saturating_add(1));

        if is_oversized(bounds) {
            self.oversized.push(id);
        }
    }

    fn replace_all(&mut self, elements: Vec<Element>) {
        self.chunks.clear();
        self.locations.clear();
        self.oversized.clear();
        self.next_order = 0;
        for element in elements {
            let order = self.next_order;
            self.put_element(element, order);
        }
    }

    pub fn remove_by_id(&mut self, id: ElementId) {
        self.remove_elements(&[id]);
    }

    /// Remove several elements and normalize groups once after the batch.
    pub fn remove_elements(&mut self, ids: &[ElementId]) {
        if ids.is_empty() {
            return;
        }
        for id in ids {
            let _ = self.take_element(*id);
        }
        for group in &mut self.groups {
            group.members.retain(|member| !ids.contains(member));
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
        let element_ids: Vec<_> = self.elements().iter().map(Element::id).collect();
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
        let order = self.next_order;
        self.put_element(element, order);
    }

    /// Move a selection as one z-order block while preserving internal order.
    pub fn reorder_selection(&mut self, ids: &[ElementId], direction: LayerOrder) {
        let selected_ids = self.expand_to_groups(ids);
        if selected_ids.is_empty() {
            return;
        }

        let elements = self.elements();
        let selected: Vec<_> = elements
            .iter()
            .filter(|element| selected_ids.contains(&element.id()))
            .cloned()
            .collect();
        if selected.is_empty() {
            return;
        }

        let first_selected_index = elements
            .iter()
            .position(|element| selected_ids.contains(&element.id()))
            .unwrap_or(0);
        let elements_before = elements[..first_selected_index]
            .iter()
            .filter(|element| !selected_ids.contains(&element.id()))
            .count();
        let mut remaining: Vec<_> = elements
            .iter()
            .filter(|element| !selected_ids.contains(&element.id()))
            .cloned()
            .collect();

        let insert_at = match direction {
            LayerOrder::Back => 0,
            LayerOrder::Backward => elements_before.saturating_sub(1),
            LayerOrder::Forward => (elements_before + 1).min(remaining.len()),
            LayerOrder::Front => remaining.len(),
        };
        remaining.splice(insert_at..insert_at, selected);
        self.replace_all(remaining);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerOrder {
    Back,
    Backward,
    Forward,
    Front,
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
    fn reorder_selection_moves_groups_as_atomic_blocks() {
        let mut s = scene_with_three();
        s.create_group(&[1, 2]);

        s.reorder_selection(&[1, 2], LayerOrder::Front);
        assert_eq!(
            s.elements().iter().map(Element::id).collect::<Vec<_>>(),
            vec![3, 1, 2]
        );

        s.reorder_selection(&[1], LayerOrder::Back);
        assert_eq!(
            s.elements().iter().map(Element::id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
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

// TEMP MEM PROBE — remove after measurement
#[cfg(test)]
mod mem_probe {
    use super::*;
    use crate::util::ExpandableArray;
    use crate::model::elements::text::WrappedText;
    use std::mem::size_of;

    fn s<T>(name: &str) {
        println!("SIZE\t{}\t{}", name, size_of::<T>());
    }

    #[test]
    fn probe_sizes() {
        s::<Color>("Color");
        s::<Option<Color>>("Option<Color>");
        s::<Point>("Point");
        s::<ElementStyle>("ElementStyle");
        s::<ElementData>("ElementData");
        s::<ExpandableArray<Point>>("ExpandableArray<Point>");
        s::<ExpandableArray<String>>("ExpandableArray<String>");
        s::<ExpandableArray<u8>>("ExpandableArray<u8>");
        s::<crate::model::elements::text::InlineText>("InlineText");
        s::<WrappedText>("WrappedText");
        s::<Rectangle>("Rectangle");
        s::<Ellipse>("Ellipse");
        s::<Line>("Line");
        s::<Freehand>("Freehand");
        s::<Text>("Text");
        s::<Element>("Element");
        s::<Vec<Element>>("Vec<Element>");
        s::<Scene>("Scene");
        s::<SceneChunk>("SceneChunk");
        s::<StoredElement>("StoredElement");

        let mut a: ExpandableArray<u8> =
            ExpandableArray::from_iter_with_capacity(1024, "hi".bytes());
        println!("CAP\tinline_text_bytes\t{}", a.capacity());
        a.push(b'x');
        println!("CAP\tinline_text_after_push\t{}", a.capacity());

        let p: ExpandableArray<Point> =
            ExpandableArray::from_iter_with_capacity(128, [Point::new(0.0, 0.0)]);
        println!("CAP\tfreehand_points\t{}\tbytes\t{}", p.capacity(), p.capacity() * size_of::<Point>());

        let l: ExpandableArray<String> =
            ExpandableArray::from_iter_with_capacity(4, ["a".to_string()]);
        println!("CAP\tlines\t{}\tbytes\t{}", l.capacity(), l.capacity() * size_of::<String>());

        let l5: ExpandableArray<String> =
            ExpandableArray::from_iter_with_capacity(4, (0..5).map(|i| format!("a{i}")));
        println!("CAP\tlines_5\t{}\tbytes\t{}", l5.capacity(), l5.capacity() * size_of::<String>());

        let mut scene = Scene::new();
        for _ in 0..1000 {
            scene.add_element(Element::Rectangle(Rectangle {
                data: ElementData::new(0),
            }));
        }
        let chunk_slots: usize = scene
            .chunks
            .values()
            .map(|c| c.elements.capacity() * size_of::<StoredElement>())
            .sum();
        println!(
            "SCENE\tchunks\t{}\tlocations_cap\t{}\tchunk_alloc_bytes\t{}\toversized\t{}",
            scene.chunks.capacity(),
            scene.locations.capacity(),
            chunk_slots,
            scene.oversized.capacity()
        );
        println!(
            "SCENE\tper_elem_chunk_bytes\t{:.1}\tper_elem_locations_bytes\t{:.1}",
            chunk_slots as f64 / 1000.0,
            (scene.locations.capacity() as f64 * (size_of::<ElementId>() + 16 + 1) as f64) / 1000.0
        );

        let fh = Freehand::from_drag(Point::new(0.0, 0.0), Point::new(1.0, 1.0), Color::new("#ffffff"), false);
        println!("ELEM\tfreehand_1pt\tstruct\t{}\theap_box\t{}\theap_pts\t{}", size_of::<Freehand>(),
            size_of::<ExpandableArray<Point>>(),
            fh.points.capacity() * size_of::<Point>());
        let t = Text { data: ElementData::new(1), wrapped: WrappedText::new("hi"), align: TextAlign::Left };
        println!("ELEM\ttext_2chars\tstruct\t{}\tlines_cap\t{}\tline0_cap\t{}",
            size_of::<Text>(), t.wrapped.lines.capacity(), t.wrapped.lines.first().map(|l| l.capacity()).unwrap_or(0));
    }
}
