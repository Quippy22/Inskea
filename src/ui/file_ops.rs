use crate::document::DocContext;
use crate::model::Scene;
use crate::skea;
use crate::tauri_bridge;
use crate::ui::export;
use crate::ui::notify;
use leptos::*;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

pub type SaveCallback = Rc<dyn Fn(bool)>;

const BROWSER_DEFAULT_NAME: &str = "untitled.skea";

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
    if !tauri_bridge::is_tauri() {
        browser_save(doc, on_done);
        return;
    }
    match doc.saved_path.get_untracked() {
        Some(path) => save_to_path(doc, path, on_done),
        None => save_as(doc, on_done),
    }
}

pub fn file_save_as(doc: DocContext) {
    if tauri_bridge::is_tauri() {
        save_as(doc, None);
    } else {
        browser_save(doc, None);
    }
}

/// Browsers cannot write back to a chosen path, so saving downloads a copy.
/// The download name is remembered so repeated saves keep the same filename.
fn browser_save(doc: DocContext, on_done: Option<SaveCallback>) {
    let saved = doc.scene.get_untracked();
    let name = doc
        .saved_path
        .get_untracked()
        .unwrap_or_else(|| BROWSER_DEFAULT_NAME.to_string());
    let content = skea::save_to_string(&saved);
    export::download_string(&content, &name);
    doc.mark_saved(Some(name.clone()), saved);
    notify::info(doc.notifications, format!("Downloaded {name}"));
    report_saved(&on_done, true);
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
    if !tauri_bridge::is_tauri() {
        browser_open(doc);
        return;
    }
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

/// Browser fallback for opening a drawing: a hidden file input plus FileReader.
pub fn browser_open(doc: DocContext) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(input) = document.create_element("input") else {
        return;
    };
    input.set_attribute("type", "file").ok();
    input.set_attribute("accept", ".skea").ok();
    input.set_attribute("style", "display:none").ok();
    let Some(body) = document.body() else {
        return;
    };
    body.append_child(&input).ok();

    let Ok(input_el) = input.clone().dyn_into::<web_sys::HtmlInputElement>() else {
        input.remove();
        return;
    };
    let input_for_change = input_el.clone();
    let document_for_cleanup = document.clone();
    let doc_for_load = doc.clone();

    let on_change = Closure::wrap(Box::new(move || {
        let input_el = input_for_change.clone();
        if let Some(file) = input_el.files().and_then(|files| files.item(0)) {
            let Ok(reader) = web_sys::FileReader::new() else {
                return;
            };
            let reader_c = reader.clone();
            let doc = doc_for_load.clone();
            let on_load = Closure::wrap(Box::new(move || {
                if let Ok(value) = reader_c.result() {
                    if let Some(text) = value.as_string() {
                        match skea::load_from_str(&text) {
                            Ok(loaded) => doc.adopt(loaded, None),
                            Err(e) => notify::error(
                                doc.notifications,
                                format!("That file is not a valid Inskea drawing: {e}"),
                            ),
                        }
                    }
                }
            }) as Box<dyn FnMut()>);
            reader.set_onload(Some(on_load.as_ref().unchecked_ref()));
            on_load.forget();
            let _ = reader.read_as_text(&file);
        }
        if let Some(body) = document_for_cleanup.body() {
            body.remove_child(&input_el).ok();
        }
    }) as Box<dyn FnMut()>);

    input_el.set_onchange(Some(on_change.as_ref().unchecked_ref()));
    on_change.forget();
    input_el.click();
}
