use crate::canvas::{CanvasMode, Viewport};
use crate::document::DocContext;
use crate::edit_ops;
use crate::model::elements::Bounds;
use crate::model::{Point, Scene};
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
        None => save_as(doc, None, on_done),
    }
}

pub fn file_save_as(doc: DocContext) {
    if tauri_bridge::is_tauri() {
        let current = doc.saved_path.get_untracked();
        save_as(doc, current, None);
    } else {
        browser_save(doc, None);
    }
}

/// Split a path into its parent directory and file name.
fn split_path(path: &str) -> (Option<String>, String) {
    match path.rfind(['/', '\\']) {
        Some(i) => (Some(path[..i].to_string()), path[i + 1..].to_string()),
        None => (None, path.to_string()),
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
    let content = match skea::save_to_string(&saved) {
        Ok(content) => content,
        Err(e) => {
            notify::error(
                doc.notifications,
                format!("Could not save the drawing: {e}"),
            );
            report_saved(&on_done, false);
            return;
        }
    };
    export::download_string(&content, &name);
    doc.mark_saved(Some(name.clone()), saved);
    notify::info(doc.notifications, format!("Downloaded {name}"));
    report_saved(&on_done, true);
}

fn save_as(doc: DocContext, current: Option<String>, on_done: Option<SaveCallback>) {
    spawn_local(async move {
        // Save As on an existing drawing starts in its folder with its name.
        let (dir, name) = match current.as_deref() {
            Some(path) => {
                let (dir, name) = split_path(path);
                (dir, name)
            }
            None => (None, "untitled.skea".to_string()),
        };
        let dir = match dir {
            Some(dir) => Some(dir),
            None => tauri_bridge::get_documents_dir().await.ok(),
        };
        let path = tauri_bridge::pick_save_path(&name, dir.as_deref()).await;
        match path {
            Some(path) => save_to_path(doc, path, on_done),
            None => report_saved(&on_done, false),
        }
    });
}

fn save_to_path(doc: DocContext, path: String, on_done: Option<SaveCallback>) {
    let notifications = doc.notifications;
    let generation = doc.generation_now();
    let path_before = doc.path_now();
    spawn_local(async move {
        let path = tauri_bridge::with_skea_extension(&path);
        let saved = doc.scene.get_untracked();
        let content = match skea::save_to_string(&saved) {
            Ok(content) => content,
            Err(e) => {
                notify::error(notifications, format!("Could not save the drawing: {e}"));
                report_saved(&on_done, false);
                return;
            }
        };
        match tauri_bridge::save_skea(&path, &content).await {
            Ok(()) => {
                doc.mark_saved_for(generation, path_before, Some(path), saved);
                report_saved(&on_done, true);
            }
            Err(e) => {
                if doc.generation_now() == generation && doc.path_now() == path_before {
                    notify::error(notifications, format!("Could not save the file: {e}"));
                }
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
    spawn_local(async move {
        let dir = tauri_bridge::get_documents_dir().await.ok();
        let path = tauri_bridge::pick_open_path(dir.as_deref()).await;
        if let Some(path) = path {
            open_path(doc, path);
        }
    });
}

/// Load a specific drawing, replacing the current document.
pub fn open_path(doc: DocContext, path: String) {
    let notifications = doc.notifications;
    spawn_local(async move {
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
            let name = file.name();
            let on_load = Closure::wrap(Box::new(move || {
                if let Ok(value) = reader_c.result() {
                    if let Some(text) = value.as_string() {
                        match skea::load_from_str(&text) {
                            Ok(loaded) => {
                                let name = (!name.is_empty()).then(|| name.clone());
                                doc.adopt(loaded, name)
                            }
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

/// Import elements from a `.skea` document into the current drawing, placing
/// them centered at `at` (or the visible center if `at` is `None`).
pub fn import_drawing_from_str(
    doc: &DocContext,
    viewport: RwSignal<Viewport>,
    canvas_mode: RwSignal<CanvasMode>,
    push_snapshot: &Rc<dyn Fn()>,
    content: &str,
    at: Option<Point>,
) {
    let notifications = doc.notifications;
    let loaded = match skea::load_from_str(content) {
        Ok(scene) => scene,
        Err(e) => {
            notify::error(notifications, format!("Could not import drawing: {e}"));
            return;
        }
    };
    let elements = loaded.elements();
    if elements.is_empty() {
        notify::info(notifications, "The imported drawing has no elements.");
        return;
    }
    let count = elements.len();
    let bounds = elements.iter().fold(None::<crate::model::Rect>, |acc, el| match acc {
        Some(r) => Some(r.union(&el.bounds())),
        None => Some(el.bounds()),
    });
    let Some(bounds) = bounds else {
        return;
    };
    let target = at.unwrap_or_else(|| {
        let visible = viewport.get_untracked().visible_rect(crate::util::window_size());
        Point::new(visible.x + visible.w / 2.0, visible.y + visible.h / 2.0)
    });
    let center = bounds.center();
    let dx = target.x - center.x;
    let dy = target.y - center.y;

    let mut new_ids = Vec::new();
    let groups = loaded.groups.clone();
    edit_ops::mutate_scene(doc.scene, push_snapshot, |s| {
        new_ids = edit_ops::insert_elements(s, elements, &groups, dx, dy);
    });
    doc.selected_ids.set(new_ids);
    canvas_mode.set(CanvasMode::Select);
    doc.show_welcome.set(false);
    notify::info(
        notifications,
        if count == 1 {
            "Imported 1 element from drawing.".to_string()
        } else {
            format!("Imported {count} elements from drawing.")
        },
    );
}

/// Browser fallback for importing a drawing: hidden file input -> import_drawing_from_str.
pub fn browser_import_drawing(
    doc: DocContext,
    viewport: RwSignal<Viewport>,
    canvas_mode: RwSignal<CanvasMode>,
    push_snapshot: Rc<dyn Fn()>,
    at: Option<Point>,
) {
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
    let vp = viewport;
    let cm = canvas_mode;
    let ps = push_snapshot;

    let on_change = Closure::wrap(Box::new(move || {
        let input_el = input_for_change.clone();
        if let Some(file) = input_el.files().and_then(|files| files.item(0)) {
            let Ok(reader) = web_sys::FileReader::new() else {
                return;
            };
            let reader_c = reader.clone();
            let doc = doc_for_load.clone();
            let vp = vp;
            let cm = cm;
            let ps = ps.clone();
            let at = at;
            let on_load = Closure::wrap(Box::new(move || {
                if let Ok(value) = reader_c.result() {
                    if let Some(text) = value.as_string() {
                        import_drawing_from_str(&doc, vp, cm, &ps, &text, at);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Point;
    use std::rc::Rc;

    #[test]
    fn split_path_keeps_the_file_name_and_parent() {
        assert_eq!(
            split_path("/home/user/Drawings/plan.skea"),
            (
                Some("/home/user/Drawings".to_string()),
                "plan.skea".to_string()
            )
        );
        assert_eq!(
            split_path("C:\\Drawings\\plan.skea"),
            (Some("C:\\Drawings".to_string()), "plan.skea".to_string())
        );
        assert_eq!(split_path("plan.skea"), (None, "plan.skea".to_string()));
    }

    #[test]
    fn import_drawing_inserts_elements_at_target() {
        let runtime = create_runtime();
        use crate::model::elements::rect::Rectangle;
        use crate::model::Element;
        use crate::model::ElementData;

        let mut source_scene = Scene::new();
        let mut rect = Rectangle {
            data: ElementData::new(0),
        };
        rect.data.world_point = Point::new(10.0, 10.0);
        rect.data.width = 100.0;
        rect.data.height = 50.0;
        source_scene.add_element(Element::Rectangle(rect));
        let serialized = skea::save_to_string(&source_scene).unwrap();

        let doc = DocContext {
            scene: create_rw_signal(Scene::new()),
            saved_path: create_rw_signal(None),
            selected_ids: create_rw_signal(Vec::new()),
            show_welcome: create_rw_signal(true),
            baseline: create_rw_signal(Scene::new()),
            dirty: create_rw_signal(false),
            notifications: create_rw_signal(Vec::new()),
            generation: create_rw_signal(0),
            reset_history: Rc::new(|| {}),
            clear_autosave: Rc::new(|| {}),
        };
        let viewport = create_rw_signal(Viewport::default());
        let canvas_mode = create_rw_signal(CanvasMode::Pan);
        let push_snapshot: Rc<dyn Fn()> = Rc::new(|| {});

        import_drawing_from_str(
            &doc,
            viewport,
            canvas_mode,
            &push_snapshot,
            &serialized,
            Some(Point::new(200.0, 200.0)),
        );

        assert_eq!(doc.scene.get().element_count(), 1);
        assert_eq!(doc.selected_ids.get().len(), 1);
        assert!(matches!(canvas_mode.get(), CanvasMode::Select));
        assert!(!doc.show_welcome.get());
        runtime.dispose();
    }
}
