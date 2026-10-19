use crate::document::DocContext;
use crate::model::Scene;
use crate::skea;
use crate::tauri_bridge;
use crate::ui::notify;
use leptos::*;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;

pub type SaveCallback = Rc<dyn Fn(bool)>;

pub fn file_new(doc: DocContext) {
    doc.adopt(Scene::new(), None);
}

pub fn file_save(doc: DocContext) {
    save(doc, None);
}

/// Save and report whether the file was actually written. A cancelled Save As
/// dialog counts as `false`, so callers can keep their pending action alive.
pub fn file_save_with(doc: DocContext, on_done: SaveCallback) {
    save(doc, Some(on_done));
}

fn save(doc: DocContext, on_done: Option<SaveCallback>) {
    match doc.saved_path.get_untracked() {
        Some(path) => save_to_path(doc, path, on_done),
        None => save_as(doc, on_done),
    }
}

pub fn file_save_as(doc: DocContext) {
    save_as(doc, None);
}

fn save_as(doc: DocContext, on_done: Option<SaveCallback>) {
    spawn_local(async move {
        let dir = tauri_bridge::get_documents_dir().await.ok();
        let path = tauri_bridge::pick_save_path("untitled.skea", dir.as_deref()).await;
        match path {
            Some(path) => save_to_path(doc, path, on_done),
            None => report_saved(&on_done, false),
        }
    });
}

fn save_to_path(doc: DocContext, path: String, on_done: Option<SaveCallback>) {
    let notifications = doc.notifications;
    spawn_local(async move {
        let path = tauri_bridge::with_skea_extension(&path);
        let saved = doc.scene.get_untracked();
        let content = skea::save_to_string(&saved);
        match tauri_bridge::save_skea(&path, &content).await {
            Ok(()) => {
                doc.mark_saved(Some(path), saved);
                report_saved(&on_done, true);
            }
            Err(e) => {
                notify::error(notifications, format!("Could not save the file: {e}"));
                report_saved(&on_done, false);
            }
        }
    });
}

fn report_saved(on_done: &Option<SaveCallback>, saved: bool) {
    if let Some(on_done) = on_done {
        on_done(saved);
    }
}

pub fn file_open(doc: DocContext) {
    let notifications = doc.notifications;
    spawn_local(async move {
        let dir = tauri_bridge::get_documents_dir().await.ok();
        let path = tauri_bridge::pick_open_path(dir.as_deref()).await;
        let Some(path) = path else {
            return;
        };
        match tauri_bridge::load_skea(&path).await {
            Ok(content) => match skea::load_from_str(&content) {
                Ok(loaded) => doc.adopt(loaded, Some(path)),
                Err(e) => notify::error(
                    notifications,
                    format!("That file is not a valid Inskea drawing: {e}"),
                ),
            },
            Err(e) => notify::error(notifications, format!("Could not open the file: {e}")),
        }
    });
}
