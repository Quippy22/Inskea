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
#[derive(Debug)]
pub struct Scene {
    chunks: HashMap<(i32, i32), SceneChunk>,
    locations: HashMap<ElementId, (i32, i32)>,
    oversized: Vec<ElementId>,
    next_order: u64,
    pub(crate) next_id: u64,
    pub(crate) groups: Vec<Group>,
    pub(crate) next_group_id: GroupId,
    pending: Vec<PendingChange>,
    pending_index: HashMap<ElementId, usize>,
    groups_before: Option<GroupSnapshot>,
    next_id_before: Option<u64>,
}

impl Clone for Scene {
    fn clone(&self) -> Self {
        Self {
            chunks: self.chunks.clone(),
            locations: self.locations.clone(),
            oversized: self.oversized.clone(),
            next_order: self.next_order,
            next_id: self.next_id,
            groups: self.groups.clone(),
            next_group_id: self.next_group_id,
            // A clone is a baseline/snapshot of committed state; unsealed
            // journal entries belong only to the live scene.
            pending: Vec::new(),
            pending_index: HashMap::new(),
            groups_before: None,
            next_id_before: None,
        }
    }
}

#[derive(Clone, Debug)]
struct SceneChunk {
    elements: Vec<StoredElement>,
}

const CHUNK_SIZE: f64 = 1024.0;

/// Chunk indices are 32-bit: at 1024 world units per chunk this covers
/// ±2.2e12 units, far beyond any practical canvas.
fn chunk_coord(value: f64) -> i32 {
    if value.is_nan() {
        0
    } else if value == f64::INFINITY {
        i32::MAX
    } else if value == f64::NEG_INFINITY {
        i32::MIN
    } else {
        (value / CHUNK_SIZE)
            .floor()
            .clamp(i32::MIN as f64, i32::MAX as f64) as i32
    }
}

fn chunk_of(point: Point) -> (i32, i32) {
    (chunk_coord(point.x), chunk_coord(point.y))
}

fn chunk_origin((x, y): (i32, i32)) -> Point {
    Point::new(x as f64 * CHUNK_SIZE, y as f64 * CHUNK_SIZE)
}

fn chunk_range(rect: Rect) -> (i32, i32, i32, i32) {
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

fn chunk_count(x0: i32, y0: i32, x1: i32, y1: i32) -> i64 {
    let width = i64::from(x1) - i64::from(x0) + 1;
    let height = i64::from(y1) - i64::from(y0) + 1;
    width.saturating_mul(height)
}

fn bounds_intersect(a: Rect, b: Rect) -> bool {
    a.x <= b.right() && a.right() >= b.x && a.y <= b.bottom() && a.bottom() >= b.y
}

fn chunk_has_visible(chunk: &SceneChunk, origin: Point, view: Rect) -> bool {
    chunk.elements.iter().any(|stored| {
        let mut bounds = element_bounds(&stored.element);
        bounds.x += origin.x;
        bounds.y += origin.y;
        bounds_intersect(bounds, view)
    })
}

fn collect_visible_chunk<'a>(
    key: (i32, i32),
    chunk: &'a SceneChunk,
    seen: &mut std::collections::HashSet<(i32, i32)>,
    visible: &mut Vec<VisibleElement<'a>>,
    view: Rect,
) {
    if !seen.insert(key) {
        return;
    }
    let origin = chunk_origin(key);
    for stored in &chunk.elements {
        let mut bounds = element_bounds(&stored.element);
        bounds.x += origin.x;
        bounds.y += origin.y;
        if bounds_intersect(bounds, view) {
            visible.push(VisibleElement {
                element: &stored.element,
                origin,
                order: stored.order,
            });
        }
    }
}

/// Drop element payloads when only the z-order changed, so reorders keep a
/// compact undo entry.
fn normalize_snapshots(
    before: ElementSnapshot,
    after: ElementSnapshot,
) -> (ElementSnapshot, ElementSnapshot) {
    match (&before, &after) {
        (
            ElementSnapshot::Present {
                element: Some(a),
                order: before_order,
            },
            ElementSnapshot::Present {
                element: Some(b),
                order: after_order,
            },
        ) if a == b => (
            ElementSnapshot::Present {
                element: None,
                order: *before_order,
            },
            ElementSnapshot::Present {
                element: None,
                order: *after_order,
            },
        ),
        (
            ElementSnapshot::Present {
                element: None,
                order: before_order,
            },
            ElementSnapshot::Present {
                element: Some(_),
                order: after_order,
            },
        ) => (
            ElementSnapshot::Present {
                element: None,
                order: *before_order,
            },
            ElementSnapshot::Present {
                element: None,
                order: *after_order,
            },
        ),
        _ => (before, after),
    }
}

fn element_bounds(element: &Element) -> Rect {
    crate::model::interaction::visual_envelope(element)
        .padded(element.data().style.stroke_width.max(0.0))
}

/// Squared distance from `point` to the nearest point of `bounds`.
fn distance_to_bounds(bounds: Rect, point: Point) -> f64 {
    let dx = (bounds.x - point.x).max(0.0).max(point.x - bounds.right());
    let dy = (bounds.y - point.y).max(0.0).max(point.y - bounds.bottom());
    dx * dx + dy * dy
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

/// One element borrowed from its chunk, together with the chunk origin needed
/// to place it in world space.
pub struct VisibleElement<'a> {
    pub element: &'a Element,
    pub origin: Point,
    pub order: u64,
}

/// Snapshot of one element's contribution to a scene change. `element: None`
/// means the element's geometry did not change and only its z-order matters.
#[derive(Clone, Debug, PartialEq)]
pub enum ElementSnapshot {
    Absent,
    Present {
        element: Option<Box<Element>>,
        order: u64,
    },
}

/// A reversible change to one element.
#[derive(Clone, Debug)]
pub struct ElementChange {
    pub id: ElementId,
    pub before: ElementSnapshot,
    pub after: ElementSnapshot,
}

/// A reversible change to the group table.
#[derive(Clone, Debug)]
pub struct GroupChange {
    pub before: Vec<Group>,
    pub before_next_id: GroupId,
    pub after: Vec<Group>,
    pub after_next_id: GroupId,
}

/// A reversible change to the id allocator.
#[derive(Clone, Debug)]
pub struct NextIdChange {
    pub before: u64,
    pub after: u64,
}

/// One undo/redo entry: only the elements (and group/id state) that actually
/// changed, not a copy of the whole document.
#[derive(Clone, Debug, Default)]
pub struct ChangeSet {
    pub elements: Vec<ElementChange>,
    pub groups: Option<GroupChange>,
    pub next_id: Option<NextIdChange>,
}

impl ChangeSet {
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty() && self.groups.is_none() && self.next_id.is_none()
    }
}

#[derive(Clone, Debug)]
struct PendingChange {
    id: ElementId,
    before: ElementSnapshot,
}

#[derive(Clone, Debug)]
struct GroupSnapshot {
    groups: Vec<Group>,
    next_group_id: GroupId,
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
            pending: Vec::new(),
            pending_index: HashMap::new(),
            groups_before: None,
            next_id_before: None,
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
            .iter()
            .flat_map(|(key, chunk)| {
                let origin = chunk_origin(*key);
                chunk.elements.iter().map(move |stored| {
                    let mut element = stored.element.clone();
                    element.offset(origin.x, origin.y);
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

    /// Borrow just the elements intersecting the view from their spatial
    /// chunks; element geometry stays relative to each chunk origin, so the
    /// canvas can render it under a per-chunk translation without cloning.
    pub fn visible_elements(&self, view: Rect) -> Vec<VisibleElement<'_>> {
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

        let mut seen = std::collections::HashSet::new();
        let mut visible = Vec::new();

        if chunk_count(x0, y0, x1, y1) > 4096 {
            // Zoomed far out: walk the chunks that exist instead of every
            // coordinate in the visible range.
            for (key, chunk) in &self.chunks {
                collect_visible_chunk(*key, chunk, &mut seen, &mut visible, view);
            }
        } else {
            for x in x0..=x1 {
                for y in y0..=y1 {
                    if let Some(chunk) = self.chunks.get(&(x, y)) {
                        collect_visible_chunk((x, y), chunk, &mut seen, &mut visible, view);
                    }
                }
            }
            // A large shape can live in a chunk outside the search range.
            for id in &self.oversized {
                if let Some(key) = self.locations.get(id) {
                    if let Some(chunk) = self.chunks.get(key) {
                        collect_visible_chunk(*key, chunk, &mut seen, &mut visible, view);
                    }
                }
            }
        }

        visible.sort_by_key(|element| element.order);
        visible
    }

    /// Materialize just the elements intersecting the view.
    pub fn elements_in_view(&self, view: Rect) -> Vec<Element> {
        self.visible_elements(view)
            .into_iter()
            .map(|visible| {
                let mut element = visible.element.clone();
                element.offset(visible.origin.x, visible.origin.y);
                element
            })
            .collect()
    }

    /// Number of elements in the scene.
    pub fn element_count(&self) -> usize {
        self.locations.len()
    }

    /// Number of persistent groups in the scene.
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    /// True when at least one element intersects the view. Stops at the first
    /// hit instead of collecting the visible set.
    pub fn has_elements_in_view(&self, view: Rect) -> bool {
        let (x0, y0, x1, y1) = chunk_range(view);
        let (x0, y0, x1, y1) = (
            x0.saturating_sub(1),
            y0.saturating_sub(1),
            x1.saturating_add(1),
            y1.saturating_add(1),
        );

        if chunk_count(x0, y0, x1, y1) > 4096 {
            return self
                .chunks
                .iter()
                .any(|(key, chunk)| chunk_has_visible(chunk, chunk_origin(*key), view));
        }

        for x in x0..=x1 {
            for y in y0..=y1 {
                if let Some(chunk) = self.chunks.get(&(x, y)) {
                    if chunk_has_visible(chunk, chunk_origin((x, y)), view) {
                        return true;
                    }
                }
            }
        }

        self.oversized.iter().any(|id| {
            let Some(key) = self.locations.get(id) else {
                return false;
            };
            self.chunks
                .get(key)
                .is_some_and(|chunk| chunk_has_visible(chunk, chunk_origin(*key), view))
        })
    }

    /// Center of the element whose visual bounds are nearest to `point`.
    pub fn nearest_element_center(&self, point: Point) -> Option<Point> {
        let mut nearest: Option<(f64, u64, Point)> = None;
        for (key, chunk) in &self.chunks {
            let origin = chunk_origin(*key);
            for stored in &chunk.elements {
                let mut bounds = element_bounds(&stored.element);
                bounds.x += origin.x;
                bounds.y += origin.y;
                let distance = distance_to_bounds(bounds, point);
                let closer = nearest.is_none_or(|(best, best_order, _)| {
                    distance
                        .total_cmp(&best)
                        .then(stored.order.cmp(&best_order))
                        .is_lt()
                });
                if closer {
                    nearest = Some((distance, stored.order, bounds.center()));
                }
            }
        }
        nearest.map(|(_, _, center)| center)
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
        let origin = chunk_origin(key);
        element.offset(origin.x, origin.y);
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

    /// Element ids in canvas z-order, without cloning element geometry.
    fn ordered(&self) -> Vec<(u64, ElementId)> {
        let mut ordered: Vec<_> = self
            .chunks
            .values()
            .flat_map(|chunk| {
                chunk
                    .elements
                    .iter()
                    .map(|stored| (stored.order, stored.element.id()))
            })
            .collect();
        ordered.sort_by_key(|(order, _)| *order);
        ordered
    }

    fn ordered_ids(&self) -> Vec<ElementId> {
        self.ordered().into_iter().map(|(_, id)| id).collect()
    }

    /// Capture the state of `id` before its first mutation in this
    /// transaction. Repeated mutations keep the original before-state.
    fn record_before(&mut self, id: ElementId) {
        if let Some(&index) = self.pending_index.get(&id) {
            // An order-only record may be followed by a content change later
            // in the same transaction; upgrade it to a full before snapshot.
            let order = match &self.pending[index].before {
                ElementSnapshot::Present {
                    element: None,
                    order,
                } => Some(*order),
                _ => None,
            };
            if let Some(order) = order {
                if let Some(element) = self.element_by_id(id) {
                    self.pending[index].before = ElementSnapshot::Present {
                        element: Some(Box::new(element)),
                        order,
                    };
                }
            }
            return;
        }
        let before = match self.element_by_id(id) {
            Some(element) => ElementSnapshot::Present {
                element: Some(Box::new(element)),
                order: self.element_order(id).unwrap_or(0),
            },
            None => ElementSnapshot::Absent,
        };
        self.pending_index.insert(id, self.pending.len());
        self.pending.push(PendingChange { id, before });
    }

    /// Record an order-only change without cloning the element.
    fn record_order_before(&mut self, id: ElementId) {
        if self.pending_index.contains_key(&id) {
            return;
        }
        let Some(order) = self.element_order(id) else {
            return;
        };
        self.pending_index.insert(id, self.pending.len());
        self.pending.push(PendingChange {
            id,
            before: ElementSnapshot::Present {
                element: None,
                order,
            },
        });
    }

    fn record_groups_before(&mut self) {
        if self.groups_before.is_none() {
            self.groups_before = Some(GroupSnapshot {
                groups: self.groups.clone(),
                next_group_id: self.next_group_id,
            });
        }
    }

    fn record_next_id_before(&mut self) {
        if self.next_id_before.is_none() {
            self.next_id_before = Some(self.next_id);
        }
    }

    fn element_state(&self, id: ElementId) -> ElementSnapshot {
        match self.element_by_id(id) {
            Some(element) => ElementSnapshot::Present {
                element: Some(Box::new(element)),
                order: self.element_order(id).unwrap_or(0),
            },
            None => ElementSnapshot::Absent,
        }
    }

    /// Seal the accumulated journal into one undo entry. Returns `None` when
    /// the transaction had no net effect.
    pub fn seal_pending(&mut self) -> Option<ChangeSet> {
        let pending = std::mem::take(&mut self.pending);
        self.pending_index = HashMap::new();
        let groups_before = self.groups_before.take();
        let next_id_before = self.next_id_before.take();

        let mut elements = Vec::new();
        for change in pending {
            let after = self.element_state(change.id);
            if change.before == after {
                continue;
            }
            let (before, after) = normalize_snapshots(change.before, after);
            if before == after {
                continue;
            }
            elements.push(ElementChange {
                id: change.id,
                before,
                after,
            });
        }

        let groups = groups_before.and_then(|before| {
            if before.groups == self.groups {
                None
            } else {
                Some(GroupChange {
                    before: before.groups,
                    before_next_id: before.next_group_id,
                    after: self.groups.clone(),
                    after_next_id: self.next_group_id,
                })
            }
        });

        // Adding and then removing an element leaves the scene unchanged; do
        // not create an undo entry just to rewind the id allocator.
        let next_id = if elements.is_empty() {
            None
        } else {
            next_id_before.and_then(|before| {
                (before != self.next_id).then_some(NextIdChange {
                    before,
                    after: self.next_id,
                })
            })
        };

        let changes = ChangeSet {
            elements,
            groups,
            next_id,
        };
        if changes.is_empty() {
            None
        } else {
            Some(changes)
        }
    }

    /// Apply a sealed entry forwards (redo) or backwards (undo).
    pub fn apply_change_set(&mut self, changes: &ChangeSet, forward: bool) {
        self.clear_pending();
        if forward {
            for change in &changes.elements {
                self.apply_snapshot(change.id, &change.after);
            }
        } else {
            for change in changes.elements.iter().rev() {
                self.apply_snapshot(change.id, &change.before);
            }
        }
        if let Some(groups) = &changes.groups {
            if forward {
                self.groups = groups.after.clone();
                self.next_group_id = groups.after_next_id;
            } else {
                self.groups = groups.before.clone();
                self.next_group_id = groups.before_next_id;
            }
        }
        if let Some(next_id) = &changes.next_id {
            self.next_id = if forward {
                next_id.after
            } else {
                next_id.before
            };
        }
    }

    fn apply_snapshot(&mut self, id: ElementId, snapshot: &ElementSnapshot) {
        match snapshot {
            ElementSnapshot::Absent => {
                self.take_element_raw(id);
            }
            ElementSnapshot::Present {
                element: Some(element),
                order,
            } => {
                self.put_element_raw((**element).clone(), *order);
            }
            ElementSnapshot::Present {
                element: None,
                order,
            } => {
                self.set_element_order_raw(id, *order);
            }
        }
    }

    /// True while the live scene has mutations that have not been sealed.
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty() || self.groups_before.is_some() || self.next_id_before.is_some()
    }

    pub fn clear_pending(&mut self) {
        self.pending = Vec::new();
        self.pending_index = HashMap::new();
        self.groups_before = None;
        self.next_id_before = None;
    }

    fn take_element_raw(&mut self, id: ElementId) -> Option<Element> {
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

    fn take_element(&mut self, id: ElementId) -> Option<Element> {
        self.record_before(id);
        self.take_element_raw(id)
    }

    fn put_element_raw(&mut self, mut element: Element, order: u64) {
        let id = element.id();
        if self.locations.contains_key(&id) {
            let _ = self.take_element_raw(id);
        }
        let bounds = element_bounds(&element);
        let home = chunk_of(bounds.center());
        let origin = chunk_origin(home);
        element.offset(-origin.x, -origin.y);
        self.chunks
            .entry(home)
            .or_insert_with(|| SceneChunk {
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

    fn put_element(&mut self, element: Element, order: u64) {
        self.record_before(element.id());
        self.put_element_raw(element, order);
    }

    fn set_element_order_raw(&mut self, id: ElementId, order: u64) {
        let Some(key) = self.locations.get(&id).copied() else {
            return;
        };
        let mut found = false;
        if let Some(chunk) = self.chunks.get_mut(&key) {
            if let Some(stored) = chunk
                .elements
                .iter_mut()
                .find(|stored| stored.element.id() == id)
            {
                stored.order = order;
                found = true;
            }
        }
        if found {
            self.next_order = self.next_order.max(order.saturating_add(1));
        }
    }

    fn set_element_order(&mut self, id: ElementId, order: u64) {
        self.record_order_before(id);
        self.set_element_order_raw(id, order);
    }

    fn replace_all(&mut self, elements: Vec<Element>) {
        self.clear_pending();
        self.chunks.clear();
        self.locations.clear();
        self.oversized.clear();
        self.next_order = 0;
        for element in elements {
            let order = self.next_order;
            self.put_element_raw(element, order);
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
        self.record_groups_before();
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
        self.record_groups_before();
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
        let element_ids = self.ordered_ids();
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
        self.record_groups_before();
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
        self.record_groups_before();
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
        self.record_next_id_before();
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

        let elements = self.ordered();
        let selected: Vec<_> = elements
            .iter()
            .filter(|(_, id)| selected_ids.contains(id))
            .map(|(_, id)| *id)
            .collect();
        if selected.is_empty() {
            return;
        }

        let first_selected_index = elements
            .iter()
            .position(|(_, id)| selected_ids.contains(id))
            .unwrap_or(0);
        let elements_before = elements[..first_selected_index]
            .iter()
            .filter(|(_, id)| !selected_ids.contains(id))
            .count();
        let mut remaining: Vec<_> = elements
            .iter()
            .filter(|(_, id)| !selected_ids.contains(id))
            .map(|(_, id)| *id)
            .collect();

        let insert_at = match direction {
            LayerOrder::Back => 0,
            LayerOrder::Backward => elements_before.saturating_sub(1),
            LayerOrder::Forward => (elements_before + 1).min(remaining.len()),
            LayerOrder::Front => remaining.len(),
        };
        remaining.splice(insert_at..insert_at, selected);

        // Reuse the existing order values. Only elements whose position in the
        // stack changed get a journal entry, and those entries are order-only.
        let order_values: Vec<_> = elements.iter().map(|(order, _)| *order).collect();
        for (id, order) in remaining.into_iter().zip(order_values) {
            if self.element_order(id) != Some(order) {
                self.set_element_order(id, order);
            }
        }
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
        // Setup is not a user action.
        s.clear_pending();
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

    #[test]
    fn visible_elements_culls_distant_chunks_and_keeps_origins() {
        let mut s = Scene::new();
        let mut near = Element::Rectangle(Rectangle {
            data: ElementData::new(0),
        });
        near.data_mut().world_point = Point::new(10.0, 20.0);
        s.add_element(near);

        let mut far = Element::Rectangle(Rectangle {
            data: ElementData::new(0),
        });
        far.data_mut().world_point = Point::new(10_000.0, 10_000.0);
        s.add_element(far);

        let visible = s.visible_elements(Rect::new(0.0, 0.0, 100.0, 100.0));
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].element.id(), 1);
        assert_eq!(
            visible[0].element.data().world_point.x + visible[0].origin.x,
            10.0
        );
        assert_eq!(
            visible[0].element.data().world_point.y + visible[0].origin.y,
            20.0
        );
    }

    #[test]
    fn visible_elements_includes_oversized_shapes_outside_search_chunks() {
        let mut s = Scene::new();
        let mut wide = Element::Rectangle(Rectangle {
            data: ElementData::new(0),
        });
        wide.data_mut().world_point = Point::new(-5000.0, 0.0);
        wide.data_mut().width = 5000.0;
        wide.data_mut().height = 100.0;
        s.add_element(wide);

        let visible = s.visible_elements(Rect::new(0.0, 0.0, 10.0, 10.0));
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].element.id(), 1);
    }

    #[test]
    fn counts_elements_and_groups() {
        let s = scene_with_three();
        assert_eq!((s.element_count(), s.group_count()), (3, 0));

        let mut s = s;
        s.create_group(&[1, 2]);
        assert_eq!((s.element_count(), s.group_count()), (3, 1));
    }

    #[test]
    fn has_elements_in_view_stops_at_the_view_edge() {
        let mut s = scene_with_three();
        assert!(!s.has_elements_in_view(Rect::new(1000.0, 1000.0, 10.0, 10.0)));

        {
            let mut element = s.element_by_id_mut(2).unwrap();
            element.data_mut().world_point = Point::new(1005.0, 1005.0);
        }

        assert!(s.has_elements_in_view(Rect::new(1000.0, 1000.0, 10.0, 10.0)));
    }

    #[test]
    fn nearest_element_center_measures_distance_to_bounds() {
        let mut s = Scene::new();
        let mut wide = Element::Rectangle(Rectangle {
            data: ElementData::new(0),
        });
        wide.data_mut().world_point = Point::new(0.0, 60.0);
        wide.data_mut().width = 1000.0;
        wide.data_mut().height = 20.0;
        s.add_element(wide);

        let mut distant = Element::Rectangle(Rectangle {
            data: ElementData::new(0),
        });
        distant.data_mut().world_point = Point::new(0.0, 200.0);
        distant.data_mut().width = 10.0;
        distant.data_mut().height = 10.0;
        s.add_element(distant);

        // The wide shape's centre is far away, but its top edge is closest to
        // the point, so it wins over the shape whose centre is nearer.
        assert_eq!(
            s.nearest_element_center(Point::new(0.0, 0.0)),
            Some(Point::new(500.0, 70.0))
        );
    }

    #[test]
    fn change_set_round_trips_a_moved_element() {
        let mut s = scene_with_three();
        let original = s.element_by_id(1).unwrap();
        {
            let mut element = s.element_by_id_mut(1).unwrap();
            element.data_mut().world_point.x += 9000.0;
        }
        let changes = s.seal_pending().expect("move should be recorded");
        assert_eq!(changes.elements.len(), 1);
        assert_eq!(changes.elements[0].id, 1);

        s.apply_change_set(&changes, false);
        assert_eq!(s.element_by_id(1), Some(original.clone()));

        s.apply_change_set(&changes, true);
        let moved = s.element_by_id(1).unwrap();
        assert_eq!(
            moved.data().world_point.x,
            original.data().world_point.x + 9000.0
        );
    }

    #[test]
    fn change_set_restores_a_removed_element() {
        let mut s = scene_with_three();
        let original = s.element_by_id(2).unwrap();
        s.remove_by_id(2);

        let changes = s.seal_pending().expect("removal should be recorded");
        assert!(matches!(changes.elements[0].after, ElementSnapshot::Absent));

        s.apply_change_set(&changes, false);
        assert_eq!(s.element_by_id(2), Some(original));

        s.apply_change_set(&changes, true);
        assert!(s.element_by_id(2).is_none());
    }

    #[test]
    fn change_set_restores_the_group_table() {
        let mut s = scene_with_three();
        s.create_group(&[1, 2]);

        let changes = s.seal_pending().expect("group should be recorded");
        assert!(changes.elements.is_empty());
        let groups = changes.groups.as_ref().expect("group change");
        assert_eq!(groups.after.len(), 1);

        s.apply_change_set(&changes, false);
        assert!(s.groups.is_empty());
        assert_eq!(s.next_group_id, groups.before_next_id);

        s.apply_change_set(&changes, true);
        assert_eq!(s.groups, groups.after);
        assert_eq!(s.next_group_id, groups.after_next_id);
    }

    #[test]
    fn change_set_keeps_reorders_order_only() {
        let mut s = scene_with_three();
        s.reorder_selection(&[1], LayerOrder::Front);

        let changes = s.seal_pending().expect("reorder should be recorded");
        assert!(!changes.elements.is_empty());
        assert!(changes.elements.iter().all(|change| matches!(
            (&change.before, &change.after),
            (
                ElementSnapshot::Present { element: None, .. },
                ElementSnapshot::Present { element: None, .. }
            )
        )));

        s.apply_change_set(&changes, false);
        assert_eq!(
            s.elements().iter().map(Element::id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );

        s.apply_change_set(&changes, true);
        assert_eq!(
            s.elements().iter().map(Element::id).collect::<Vec<_>>(),
            vec![2, 3, 1]
        );
    }

    #[test]
    fn added_then_removed_element_leaves_no_history() {
        let mut s = Scene::new();
        s.add_element(Element::Rectangle(Rectangle {
            data: ElementData::new(0),
        }));
        s.remove_by_id(1);

        assert!(s.seal_pending().is_none());
        assert!(s.elements().is_empty());
    }

    #[test]
    fn change_set_rewinds_the_id_allocator_for_adds() {
        let mut s = Scene::new();
        s.add_element(Element::Rectangle(Rectangle {
            data: ElementData::new(0),
        }));
        assert_eq!(s.next_id, 2);

        let changes = s.seal_pending().expect("add should be recorded");
        assert_eq!(
            changes
                .next_id
                .as_ref()
                .map(|change| (change.before, change.after)),
            Some((1, 2))
        );

        s.apply_change_set(&changes, false);
        assert_eq!(s.next_id, 1);
        assert!(s.elements().is_empty());

        s.apply_change_set(&changes, true);
        assert_eq!(s.next_id, 2);
        assert_eq!(s.elements().len(), 1);
    }

    #[test]
    fn change_set_coalesces_repeated_moves_of_one_element() {
        let mut s = scene_with_three();
        let original = s.element_by_id(1).unwrap();
        for dx in [1.0, 2.0, 3.0] {
            let mut element = s.element_by_id_mut(1).unwrap();
            element.data_mut().world_point.x += dx;
        }

        let changes = s.seal_pending().expect("moves should be recorded");
        assert_eq!(changes.elements.len(), 1);

        s.apply_change_set(&changes, false);
        assert_eq!(s.element_by_id(1), Some(original));
    }

    #[test]
    fn sealed_change_sets_compose_for_undo_and_redo() {
        let mut s = scene_with_three();
        let original = s.element_by_id(1).unwrap();
        {
            let mut element = s.element_by_id_mut(1).unwrap();
            element.data_mut().world_point.x += 5000.0;
        }
        let move_changes = s.seal_pending().expect("move should be recorded");
        let moved = s.element_by_id(1).unwrap();

        s.add_element(Element::Rectangle(Rectangle {
            data: ElementData::new(0),
        }));
        let add_changes = s.seal_pending().expect("add should be recorded");
        let added_id = 4;

        s.apply_change_set(&add_changes, false);
        s.apply_change_set(&move_changes, false);
        assert_eq!(s.element_by_id(1), Some(original));
        assert!(s.element_by_id(added_id).is_none());
        assert_eq!(s.next_id, 4);

        s.apply_change_set(&move_changes, true);
        s.apply_change_set(&add_changes, true);
        assert_eq!(s.element_by_id(1), Some(moved));
        assert!(s.element_by_id(added_id).is_some());
        assert_eq!(s.next_id, 5);
    }

    #[test]
    fn change_set_undo_restores_grouped_element_and_membership() {
        let mut s = scene_with_three();
        s.create_group(&[1, 2]);
        s.seal_pending().expect("group should be recorded");
        let groups = s.groups.clone();
        let original = s.element_by_id(1).unwrap();

        s.remove_by_id(1);
        let changes = s.seal_pending().expect("removal should be recorded");
        assert!(changes.groups.is_some());

        s.apply_change_set(&changes, false);
        assert_eq!(s.element_by_id(1), Some(original));
        assert_eq!(s.groups, groups);

        s.apply_change_set(&changes, true);
        assert!(s.element_by_id(1).is_none());
        assert!(s.groups.is_empty());
    }

    #[test]
    fn unchanged_element_guard_leaves_no_history() {
        let mut s = scene_with_three();
        let _ = s.element_by_id_mut(1).unwrap();
        assert!(s.seal_pending().is_none());
    }
}
