use crate::canvas::{CanvasMode, Viewport};
use crate::model::Scene;
use crate::skea;
use crate::tauri_bridge;
use crate::ui::classes;
use crate::ui::icon;
use leptos::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

fn is_tauri() -> bool {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return false,
    };
    js_sys::Reflect::get(&window, &JsValue::from_str("__TAURI__"))
        .map(|v| !v.is_undefined() && !v.is_null())
        .unwrap_or(false)
}

fn browser_export(scene: Scene) {
    let content = skea::save_to_string(&scene);
    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(&content));
    let blob = web_sys::Blob::new_with_str_sequence(&parts).expect("failed to create Blob");
    let url = web_sys::Url::create_object_url_with_blob(&blob).expect("failed to create URL");
    let document = web_sys::window().unwrap().document().unwrap();
    let anchor = document.create_element("a").unwrap();
    anchor.set_attribute("href", &url).ok();
    anchor.set_attribute("download", "untitled.skea").ok();
    anchor.set_attribute("style", "display:none").ok();
    document.body().unwrap().append_child(&anchor).ok();
    let _ = anchor
        .dyn_ref::<web_sys::HtmlElement>()
        .map(|el| el.click());
    document.body().unwrap().remove_child(&anchor).ok();
    web_sys::Url::revoke_object_url(&url).ok();
}

fn browser_import(scene: RwSignal<Scene>) {
    let document = web_sys::window().unwrap().document().unwrap();
    let input = document
        .create_element("input")
        .expect("failed to create input");
    input.set_attribute("type", "file").ok();
    input.set_attribute("accept", ".skea").ok();
    input.set_attribute("style", "display:none").ok();
    document.body().unwrap().append_child(&input).ok();

    let input_el = input
        .dyn_ref::<web_sys::HtmlInputElement>()
        .expect("input is not HtmlInputElement")
        .clone();
    let input_el2 = input_el.clone();
    let doc = document.clone();

    let on_change = Closure::wrap(Box::new(move || {
        let input_el = input_el.clone();
        let doc = doc.clone();
        if let Some(file) = input_el.files().and_then(|f| f.item(0)) {
            let reader = web_sys::FileReader::new().expect("failed to create FileReader");
            let reader_c = reader.clone();
            let scene_c = scene;
            let on_load = Closure::wrap(Box::new(move || {
                if let Ok(val) = reader_c.result() {
                    if let Some(text) = val.as_string() {
                        match skea::load_from_str(&text) {
                            Ok(loaded) => scene_c.set(loaded),
                            Err(e) => {
                                web_sys::console::error_1(&format!("parse error: {e}").into());
                            }
                        }
                    }
                }
            }) as Box<dyn FnMut()>);
            reader.set_onload(Some(on_load.as_ref().unchecked_ref()));
            on_load.forget();
            let _ = reader.read_as_text(&file);
        }
        doc.body().unwrap().remove_child(&input_el).ok();
    }) as Box<dyn FnMut()>);

    input_el2.set_onchange(Some(on_change.as_ref().unchecked_ref()));
    on_change.forget();
    input_el2.click();
}

#[component]
pub fn ToolBar<F1, F2>(
    scene: RwSignal<Scene>,
    viewport: RwSignal<Viewport>,
    canvas_mode: RwSignal<CanvasMode>,
    on_undo: F1,
    on_redo: F2,
    can_undo: Signal<bool>,
    can_redo: Signal<bool>,
) -> impl IntoView
where
    F1: Fn() + 'static,
    F2: Fn() + 'static,
{
    let menu_open = create_rw_signal(false);
    let submenu_open = create_rw_signal(false);
    let saved_path = create_rw_signal::<Option<String>>(None);
    let tauri = is_tauri();

    let on_home = move |_| viewport.set(Viewport::default());

    let close_menu = move || {
        menu_open.set(false);
        submenu_open.set(false);
    };

    let on_new = move |_| {
        close_menu();
        saved_path.set(None);
        scene.set(Scene::new());
    };

    let on_save_as = move |_| {
        close_menu();
        spawn_local(async move {
            let dir = tauri_bridge::get_app_data_dir().await.ok();
            let path = tauri_bridge::pick_save_path("untitled.skea", dir.as_deref()).await;
            if let Some(path) = path {
                saved_path.set(Some(path.clone()));
                let scene = scene.get();
                let content = skea::save_to_string(&scene);
                if let Err(e) = tauri_bridge::save_skea(&path, &content).await {
                    web_sys::console::error_1(&format!("save failed: {e}").into());
                }
            }
        });
    };

    let on_save = move |_| {
        close_menu();
        let saved = saved_path.get();
        if let Some(path) = saved {
            let scene = scene.get();
            spawn_local(async move {
                let content = skea::save_to_string(&scene);
                if let Err(e) = tauri_bridge::save_skea(&path, &content).await {
                    web_sys::console::error_1(&format!("save failed: {e}").into());
                }
            });
        } else {
            spawn_local(async move {
                let dir = tauri_bridge::get_app_data_dir().await.ok();
                let path = tauri_bridge::pick_save_path("untitled.skea", dir.as_deref()).await;
                if let Some(path) = path {
                    saved_path.set(Some(path.clone()));
                    let scene = scene.get();
                    let content = skea::save_to_string(&scene);
                    if let Err(e) = tauri_bridge::save_skea(&path, &content).await {
                        web_sys::console::error_1(&format!("save failed: {e}").into());
                    }
                }
            });
        }
    };

    let on_export = move |_| {
        close_menu();
        let scene = scene.get();
        if tauri {
            spawn_local(async move {
                let dir = tauri_bridge::get_app_data_dir().await.ok();
                let path = tauri_bridge::pick_save_path("untitled.skea", dir.as_deref()).await;
                if let Some(path) = path {
                    let content = skea::save_to_string(&scene);
                    if let Err(e) = tauri_bridge::save_skea(&path, &content).await {
                        web_sys::console::error_1(&format!("export failed: {e}").into());
                    }
                }
            });
        } else {
            browser_export(scene);
        }
    };

    let on_import = move |_| {
        close_menu();
        if tauri {
            spawn_local(async move {
                let path = tauri_bridge::pick_open_path(None).await;
                if let Some(path) = path {
                    saved_path.set(Some(path.clone()));
                    match tauri_bridge::load_skea(&path).await {
                        Ok(content) => match skea::load_from_str(&content) {
                            Ok(loaded) => scene.set(loaded),
                            Err(e) => {
                                web_sys::console::error_1(&format!("parse error: {e}").into());
                            }
                        },
                        Err(e) => {
                            web_sys::console::error_1(&format!("load failed: {e}").into());
                        }
                    }
                }
            });
        } else {
            browser_import(scene);
        }
    };

    let on_open = move |_| {
        close_menu();
        spawn_local(async move {
            let dir = tauri_bridge::get_app_data_dir().await.ok();
            let path = tauri_bridge::pick_open_path(dir.as_deref()).await;
            if let Some(path) = path {
                saved_path.set(Some(path.clone()));
                match tauri_bridge::load_skea(&path).await {
                    Ok(content) => match skea::load_from_str(&content) {
                        Ok(loaded) => scene.set(loaded),
                        Err(e) => {
                            web_sys::console::error_1(&format!("parse error: {e}").into());
                        }
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("load failed: {e}").into());
                    }
                }
            }
        });
    };

    let btn = move |mode: CanvasMode| -> &'static str {
        if canvas_mode.get() == mode {
            classes::BTN_TBAR_ACTIVE
        } else {
            classes::BTN_TBAR_INACTIVE
        }
    };

    view! {
        <div class=classes::CONTAINER_STATUSBAR>
            <div class=classes::TBAR_INNER>
                <button
                    class=move || btn(CanvasMode::Hand)
                    on:click=move |_| canvas_mode.set(CanvasMode::Hand)
                    title="Hand / Pan"
                >
                    {icon::hand()}
                </button>
                <button
                    class=move || btn(CanvasMode::Select)
                    on:click=move |_| canvas_mode.set(CanvasMode::Select)
                    title="Select"
                >
                    {icon::cursor()}
                </button>
                <button
                    class=move || btn(CanvasMode::Draw)
                    on:click=move |_| canvas_mode.set(CanvasMode::Draw)
                    title="Draw"
                >
                    {icon::pencil()}
                </button>
                <div class=classes::SEP_V />
                <button class=classes::BTN_GHOST on:click=on_home title="Home">
                    {icon::home()}
                </button>
                <button
                    class=classes::BTN_GHOST
                    class:opacity-40=move || !can_undo.get()
                    class:cursor-not-allowed=move || !can_undo.get()
                    on:click=move |_| on_undo()
                    title="Undo"
                >
                    {icon::undo()}
                </button>
                <button
                    class=classes::BTN_GHOST
                    class:opacity-40=move || !can_redo.get()
                    class:cursor-not-allowed=move || !can_redo.get()
                    on:click=move |_| on_redo()
                    title="Redo"
                >
                    {icon::redo()}
                </button>
                <div class=classes::SEP_V />
                <div class="relative">
                    <button
                        class=classes::BTN_GHOST
                        on:click=move |_| menu_open.update(|v| *v = !*v)
                        title="Menu"
                    >
                        {icon::menu()}
                    </button>
                    {move || {
                        if menu_open.get() {
                            view! {
                                <>
                                    <div
                                        class="fixed inset-0 z-40"
                                        on:click=move |_| close_menu()
                                    ></div>
                                    <div class=classes::MENU_DROPDOWN>
                                        <div
                                            class="relative"
                                            on:mouseenter=move |_| submenu_open.set(true)
                                            on:mouseleave=move |_| submenu_open.set(false)
                                        >
                                            <button class=classes::MENU_ITEM>
                                                <span>"File"</span>
                                                {icon::chevron_right()}
                                            </button>
                                            {move || {
                                                if submenu_open.get() {
                                                    view! {
                                                        <div
                                                            class=classes::MENU_DROPDOWN
                                                            style="left: 100%; top: -4px;"
                                                        >
                                                            <button class=classes::MENU_ITEM on:click=on_new>
                                                                "New"
                                                            </button>
                                                            {move || tauri.then(|| view! {
                                                                <button class=classes::MENU_ITEM on:click=on_save>
                                                                    "Save"
                                                                </button>
                                                                <button class=classes::MENU_ITEM on:click=on_save_as>
                                                                    "Save As"
                                                                </button>
                                                                <button class=classes::MENU_ITEM on:click=on_open>
                                                                    "Open"
                                                                </button>
                                                            })}
                                                            <button class=classes::MENU_ITEM on:click=on_export>
                                                                "Export"
                                                            </button>
                                                            <button class=classes::MENU_ITEM on:click=on_import>
                                                                "Import"
                                                            </button>
                                                        </div>
                                                    }
                                                        .into_view()
                                                } else {
                                                    view! {}.into_view()
                                                }
                                            }}
                                        </div>
                                    </div>
                                </>
                            }
                                .into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
