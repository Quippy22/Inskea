use crate::model::{ElementId, Scene};
use crate::ui::notify::Notifications;
use leptos::*;
use std::rc::Rc;

/// Shared state for the open document: its scene, file path, selection, the
/// last saved contents, and the undo-history reset hook.
#[derive(Clone)]
pub struct DocContext {
    pub scene: RwSignal<Scene>,
    pub saved_path: RwSignal<Option<String>>,
    pub selected_ids: RwSignal<Vec<ElementId>>,
    /// Scene contents last written to disk (or the empty scene before the
    /// first save). Compared against `scene` to derive the dirty flag.
    pub baseline: RwSignal<Scene>,
    pub dirty: RwSignal<bool>,
    pub notifications: Notifications,
    pub reset_history: Rc<dyn Fn()>,
}

impl DocContext {
    /// True when the in-memory scene differs from the last saved contents.
    /// The comparison clones both scenes, so call this at user-paced moments
    /// (action guards, close requests), not on every pointer move.
    pub fn is_dirty_now(&self) -> bool {
        self.scene.get_untracked() != self.baseline.get_untracked()
    }

    /// Replace the document with a freshly created or loaded scene.
    pub fn adopt(&self, scene: Scene, path: Option<String>) {
        self.selected_ids.set(Vec::new());
        self.baseline.set(scene.clone());
        self.scene.set(scene);
        self.saved_path.set(path);
        self.dirty.set(false);
        (self.reset_history)();
    }

    /// Record a successful write of `saved` to `path`.
    pub fn mark_saved(&self, path: Option<String>, saved: Scene) {
        if let Some(path) = path {
            self.saved_path.set(Some(path));
        }
        let unchanged = self.scene.get_untracked() == saved;
        self.baseline.set(saved);
        self.dirty.set(!unchanged);
    }
}
