use crate::document::DocContext;
use crate::model::Scene;
use crate::skea;
use crate::tauri_bridge;
use leptos::*;
use wasm_bindgen_futures::spawn_local;

pub fn file_new(doc: DocContext) {
    doc.adopt(Scene::new(), None);
}

pub fn file_save(doc: DocContext) {
    match doc.saved_path.get_untracked() {
        Some(path) => save_to_path(doc, path),
        None => file_save_as(doc),
    }
}

pub fn file_save_as(doc: DocContext) {
    spawn_local(async move {
        let dir = tauri_bridge::get_documents_dir().await.ok();
        let path = tauri_bridge::pick_save_path("untitled.skea", dir.as_deref()).await;
        if let Some(path) = path {
            save_to_path(doc, path);
        }
    });
}

fn save_to_path(doc: DocContext, path: String) {
    spawn_local(async move {
        let saved = doc.scene.get_untracked();
        let content = skea::save_to_string(&saved);
        match tauri_bridge::save_skea(&path, &content).await {
            Ok(()) => doc.mark_saved(Some(path), saved),
            Err(e) => web_sys::console::error_1(&format!("save: {e}").into()),
        }
    });
}

pub fn file_open(doc: DocContext) {
    spawn_local(async move {
        let dir = tauri_bridge::get_documents_dir().await.ok();
        let path = tauri_bridge::pick_open_path(dir.as_deref()).await;
        let Some(path) = path else {
            return;
        };
        match tauri_bridge::load_skea(&path).await {
            Ok(content) => match skea::load_from_str(&content) {
                Ok(loaded) => doc.adopt(loaded, Some(path)),
                Err(e) => web_sys::console::error_1(&format!("parse: {e}").into()),
            },
            Err(e) => web_sys::console::error_1(&format!("load: {e}").into()),
        }
    });
}
