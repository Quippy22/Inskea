use crate::canvas::{CanvasMode, Viewport};
use crate::model::Scene;
use crate::skea;
use crate::tauri_bridge;
use crate::ui::classes;
use crate::ui::icon;
use leptos::*;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn ToolBar(
    scene: RwSignal<Scene>,
    viewport: RwSignal<Viewport>,
    canvas_mode: RwSignal<CanvasMode>,
) -> impl IntoView {
    let menu_open = create_rw_signal(false);
    let saved_path = create_rw_signal::<Option<String>>(None);

    let on_home = move |_| viewport.set(Viewport::default());

    let close_menu = move || menu_open.set(false);

    let on_new = move |_| {
        close_menu();
        saved_path.set(None);
        scene.set(Scene::new());
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
    };

    let on_import = move |_| {
        close_menu();
        spawn_local(async move {
            let path = tauri_bridge::pick_open_path(None).await;
            if let Some(path) = path {
                saved_path.set(Some(path.clone()));
                match tauri_bridge::load_skea(&path).await {
                    Ok(content) => match skea::load_from_str(&content) {
                        Ok(loaded) => scene.set(loaded),
                        Err(e) => {
                            web_sys::console::error_1(
                                &format!("parse error: {e}").into(),
                            );
                        }
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("load failed: {e}").into());
                    }
                }
            }
        });
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
                            web_sys::console::error_1(
                                &format!("parse error: {e}").into(),
                            );
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
                <button class=classes::BTN_GHOST title="Undo">
                    {icon::undo()}
                </button>
                <button class=classes::BTN_GHOST title="Redo">
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
                                        <button class=classes::MENU_ITEM on:click=on_new>
                                            "New"
                                        </button>
                                        <button class=classes::MENU_ITEM on:click=on_save>
                                            "Save"
                                        </button>
                                        <button class=classes::MENU_ITEM on:click=on_open>
                                            "Open"
                                        </button>
                                        <button class=classes::MENU_ITEM on:click=on_export>
                                            "Export"
                                        </button>
                                        <button class=classes::MENU_ITEM on:click=on_import>
                                            "Import"
                                        </button>
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
