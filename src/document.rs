use crate::model::{ElementId, Scene};
use crate::ui::notify::Notifications;
use leptos::*;
use std::rc::Rc;

/// A document action that may need the user's confirmation first.
#[derive(Clone)]
pub struct PendingAction {
    /// Completes the sentence "Save your drawing before you ...".
    pub label: &'static str,
    pub run: Rc<dyn Fn()>,
}

/// Ask to run an action, prompting first when the document has unsaved edits.
pub type RequestAction = Rc<dyn Fn(PendingAction)>;

/// Shared state for the open document: its scene, file path, selection,
/// opening hint, the last saved contents, and the undo-history reset hook.
#[derive(Clone)]
pub struct DocContext {
    pub scene: RwSignal<Scene>,
    pub saved_path: RwSignal<Option<String>>,
    pub selected_ids: RwSignal<Vec<ElementId>>,
    /// Whether the one-time opening hint should cover the canvas.
    pub show_welcome: RwSignal<bool>,
    /// Scene contents last written to disk (or the empty scene before the
    /// first save). Compared against `scene` to derive the dirty flag.
    pub baseline: RwSignal<Scene>,
    pub dirty: RwSignal<bool>,
    pub notifications: Notifications,
    /// Bumped whenever the open document is replaced. Async work uses it to
    /// avoid writing its result into a different document.
    pub generation: RwSignal<u32>,
    pub reset_history: Rc<dyn Fn()>,
    /// Drop any browser-side autosaved copy (no-op on desktop).
    pub clear_autosave: Rc<dyn Fn()>,
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
        self.replace(scene, path, true);
    }

    /// Replace the document with a recovered autosave. Keeps the stored copy
    /// so another reload can restore it again before the next edit.
    pub fn restore(&self, scene: Scene) {
        self.replace(scene, None, false);
    }

    fn replace(&self, scene: Scene, path: Option<String>, clear_autosave: bool) {
        self.generation.update(|generation| {
            *generation = generation.wrapping_add(1);
        });
        self.selected_ids.set(Vec::new());
        self.show_welcome.set(path.is_none() && scene.is_empty());
        self.baseline.set(scene.clone());
        self.scene.set(scene);
        self.saved_path.set(path);
        self.dirty.set(false);
        (self.reset_history)();
        if clear_autosave {
            (self.clear_autosave)();
        }
    }

    pub fn generation_now(&self) -> u32 {
        self.generation.get_untracked()
    }

    /// Record a successful write only if the document has not been replaced
    /// and its file path has not changed since the write started.
    pub fn mark_saved_for(
        &self,
        generation: u32,
        path_before: Option<String>,
        path: Option<String>,
        saved: Scene,
    ) {
        if self.generation_now() == generation && self.saved_path.get_untracked() == path_before {
            self.mark_saved(path, saved);
        }
    }

    pub fn path_now(&self) -> Option<String> {
        self.saved_path.get_untracked()
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
