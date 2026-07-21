use crate::model::{ElementId, Scene};
use crate::skea;
use crate::tauri_bridge;
use leptos::*;
use wasm_bindgen_futures::spawn_local;

pub fn file_new(
    scene: RwSignal<Scene>,
    saved_path: RwSignal<Option<String>>,
    selected_ids: RwSignal<Vec<ElementId>>,
) {
    saved_path.set(None);
    selected_ids.set(Vec::new());
    scene.set(Scene::new());
}

pub fn file_save(scene: RwSignal<Scene>, saved_path: RwSignal<Option<String>>) {
    let path = saved_path.get();
    if let Some(path) = path {
        let s = scene.get();
        spawn_local(async move {
            let c = skea::save_to_string(&s);
            let _ = tauri_bridge::save_skea(&path, &c).await;
        });
    } else {
        file_save_as(scene, saved_path);
    }
}

pub fn file_save_as(scene: RwSignal<Scene>, saved_path: RwSignal<Option<String>>) {
    let s = scene.get();
    spawn_local(async move {
        let dir = tauri_bridge::get_app_data_dir().await.ok();
        let path = tauri_bridge::pick_save_path("untitled.skea", dir.as_deref()).await;
        if let Some(path) = path {
            saved_path.set(Some(path.clone()));
            let c = skea::save_to_string(&s);
            let _ = tauri_bridge::save_skea(&path, &c).await;
        }
    });
}

pub fn file_open(
    scene: RwSignal<Scene>,
    saved_path: RwSignal<Option<String>>,
    selected_ids: RwSignal<Vec<ElementId>>,
) {
    spawn_local(async move {
        let dir = tauri_bridge::get_app_data_dir().await.ok();
        let path = tauri_bridge::pick_open_path(dir.as_deref()).await;
        if let Some(path) = path {
            saved_path.set(Some(path.clone()));
            match tauri_bridge::load_skea(&path).await {
                Ok(c) => match skea::load_from_str(&c) {
                    Ok(loaded) => {
                        selected_ids.set(Vec::new());
                        scene.set(loaded);
                    }
                    Err(e) => web_sys::console::error_1(&format!("parse: {e}").into()),
                },
                Err(e) => web_sys::console::error_1(&format!("load: {e}").into()),
            }
        }
    });
}
