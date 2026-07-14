#![allow(clippy::redundant_locals)]
use crate::canvas::{CanvasMode, CropExportCallback, Viewport};
use crate::model::Scene;
use crate::skea;
use crate::tauri_bridge;
use crate::util::window_size;
use crate::ui::classes;
use crate::ui::components::{Dropdown, DropdownItem, IconButton};
use crate::ui::export;
use crate::ui::icon;
use leptos::*;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

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

// ── ToolBar component ─────────────────────────────────────────────────

#[component]
pub fn ToolBar<F1, F2>(
    scene: RwSignal<Scene>,
    viewport: RwSignal<Viewport>,
    canvas_mode: RwSignal<CanvasMode>,
    on_undo: F1,
    on_redo: F2,
    can_undo: Signal<bool>,
    can_redo: Signal<bool>,
    export_crop_active: RwSignal<bool>,
    on_crop_export: RwSignal<Option<CropExportCallback>>,
    shortcuts_open: RwSignal<bool>,
) -> impl IntoView
where
    F1: Fn() + 'static,
    F2: Fn() + 'static,
{
    let menu_open = create_rw_signal(false);
    let saved_path = create_rw_signal::<Option<String>>(None);
    let tauri = tauri_bridge::is_tauri();

    // Submenu hover tracking — one pair per fly-out panel
    let file_hover = create_rw_signal(false);
    let file_sub_hover = create_rw_signal(false);
    let show_file = Signal::derive(move || file_hover.get() || file_sub_hover.get());

    let export_hover = create_rw_signal(false);
    let export_sub_hover = create_rw_signal(false);
    let show_export = Signal::derive(move || export_hover.get() || export_sub_hover.get());

    let on_home = move || viewport.set(Viewport::default());

    let close_menu = move || {
        menu_open.set(false);
        file_hover.set(false);
        file_sub_hover.set(false);
        export_hover.set(false);
        export_sub_hover.set(false);
    };

    // ── File actions ───────────────────────────────────────────────────

    let on_new = {
        let close_menu = close_menu;
        move || {
            close_menu();
            saved_path.set(None);
            scene.set(Scene::new());
        }
    };
    let on_save_as = {
        let close_menu = close_menu;
        move || {
            close_menu();
            spawn_local(async move {
                let dir = tauri_bridge::get_app_data_dir().await.ok();
                let path = tauri_bridge::pick_save_path("untitled.skea", dir.as_deref()).await;
                if let Some(path) = path {
                    saved_path.set(Some(path.clone()));
                    let s = scene.get();
                    let c = skea::save_to_string(&s);
                    let _ = tauri_bridge::save_skea(&path, &c).await;
                }
            });
        }
    };
    let on_save = {
        let close_menu = close_menu;
        let on_save_as = on_save_as;
        move || {
            close_menu();
            let saved = saved_path.get();
            if let Some(path) = saved {
                let s = scene.get();
                spawn_local(async move {
                    let c = skea::save_to_string(&s);
                    let _ = tauri_bridge::save_skea(&path, &c).await;
                });
            } else {
                on_save_as();
            }
        }
    };
    let on_open = {
        let close_menu = close_menu;
        move || {
            close_menu();
            spawn_local(async move {
                let dir = tauri_bridge::get_app_data_dir().await.ok();
                let path = tauri_bridge::pick_open_path(dir.as_deref()).await;
                if let Some(path) = path {
                    saved_path.set(Some(path.clone()));
                    match tauri_bridge::load_skea(&path).await {
                        Ok(c) => match skea::load_from_str(&c) {
                            Ok(loaded) => scene.set(loaded),
                            Err(e) => web_sys::console::error_1(&format!("parse: {e}").into()),
                        },
                        Err(e) => web_sys::console::error_1(&format!("load: {e}").into()),
                    }
                }
            });
        }
    };
    let on_shortcuts = {
        let close_menu = close_menu;
        move || {
            close_menu();
            shortcuts_open.set(true);
        }
    };

    let on_import = {
        let close_menu = close_menu;
        move || {
            close_menu();
            if tauri {
                spawn_local(async move {
                    let path = tauri_bridge::pick_open_path(None).await;
                    if let Some(path) = path {
                        saved_path.set(Some(path.clone()));
                        match tauri_bridge::load_skea(&path).await {
                            Ok(c) => match skea::load_from_str(&c) {
                                Ok(loaded) => scene.set(loaded),
                                Err(e) => web_sys::console::error_1(&format!("parse: {e}").into()),
                            },
                            Err(e) => web_sys::console::error_1(&format!("load: {e}").into()),
                        }
                    }
                });
            } else {
                browser_import(scene);
            }
        }
    };

    // ── Export actions ─────────────────────────────────────────────────

    let on_export_skea = {
        let close_menu = close_menu;
        move || {
            close_menu();
            let s = scene.get();
            if tauri {
                spawn_local(async move {
                    let dir = tauri_bridge::get_app_data_dir().await.ok();
                    let path = tauri_bridge::pick_save_path("untitled.skea", dir.as_deref()).await;
                    if let Some(path) = path {
                        let c = skea::save_to_string(&s);
                        let _ = tauri_bridge::save_skea(&path, &c).await;
                    }
                });
            } else {
                export::browser_export_skea(s);
            }
        }
    };

    #[derive(Clone, Copy)]
    enum ExportFormat { Svg, Png }

    let on_export = {
        let _close_menu = close_menu;
        let _export_crop_active = export_crop_active;
        let _on_crop_export = on_crop_export;
        let _scene = scene;
        move |fmt: ExportFormat, selection: bool| {
            _close_menu();
            if selection {
                let scene = _scene;
                let on_crop_export = _on_crop_export;
                let export_crop_active = _export_crop_active;
                let close_menu = _close_menu;
                on_crop_export.set(Some(Rc::new(move |rect: (f64, f64, f64, f64)| {
                    close_menu();
                    let s = scene.get();
                    let svg = export::scene_to_svg_crop(&s, rect);
                    match fmt {
                        ExportFormat::Svg => {
                            if tauri { export::tauri_export_svg(svg, true); }
                            else { export::download_string(&svg, "selection.svg"); }
                        }
                        ExportFormat::Png => {
                            if tauri { export::tauri_export_png(svg, true); }
                            else { export::download_png_from_svg(svg, "selection.png".to_string()); }
                        }
                    }
                })));
                export_crop_active.set(true);
                return;
            }
            let s = _scene.get();
            let vp = viewport.get();
            let size = window_size();
            let svg = export::scene_to_svg(&s, &vp, size, None);
            match fmt {
                ExportFormat::Svg => {
                    if tauri { export::tauri_export_svg(svg, false); }
                    else { export::download_string(&svg, "canvas.svg"); }
                }
                ExportFormat::Png => {
                    if tauri { export::tauri_export_png(svg, false); }
                    else { export::download_png_from_svg(svg, "canvas.png".to_string()); }
                }
            }
        }
    };

    // ── Dropdown item lists ────────────────────────────────────────────

    let file_items: Vec<DropdownItem> = vec![
        DropdownItem::Action {
            label: "New",
            on_click: Rc::new(on_new),
        },
        DropdownItem::Action {
            label: "Save",
            on_click: Rc::new(on_save),
        },
        DropdownItem::Action {
            label: "Save As",
            on_click: Rc::new(on_save_as),
        },
        DropdownItem::Action {
            label: "Open",
            on_click: Rc::new(on_open),
        },
        DropdownItem::Separator,
        DropdownItem::Action {
            label: "Import",
            on_click: Rc::new(on_import),
        },
        DropdownItem::Separator,
        DropdownItem::Action {
            label: "Keyboard Shortcuts",
            on_click: Rc::new(on_shortcuts),
        },
    ];

    let export_items: Vec<DropdownItem> = vec![
        DropdownItem::Action {
            label: "Export .skea",
            on_click: Rc::new(on_export_skea),
        },
        DropdownItem::Separator,
        DropdownItem::Header { label: "PNG" },
        DropdownItem::Action {
            label: "Full canvas",
            on_click: Rc::new({
                let f = on_export;
                move || f(ExportFormat::Png, false)
            }),
        },
        DropdownItem::Action {
            label: "Selection",
            on_click: Rc::new({
                let f = on_export;
                move || f(ExportFormat::Png, true)
            }),
        },
        DropdownItem::Separator,
        DropdownItem::Header { label: "SVG" },
        DropdownItem::Action {
            label: "Full canvas",
            on_click: Rc::new({
                let f = on_export;
                move || f(ExportFormat::Svg, false)
            }),
        },
        DropdownItem::Action {
            label: "Selection",
            on_click: Rc::new({
                let f = on_export;
                move || f(ExportFormat::Svg, true)
            }),
        },
    ];

    // ── Mode button helper ─────────────────────────────────────────────

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
                    class=move || btn(CanvasMode::Pan)
                    on:click=move |_| canvas_mode.set(CanvasMode::Pan)
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
                <IconButton on_click=on_home title="Home" class=classes::BTN_GHOST>
                    {icon::home()}
                </IconButton>
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
                    <IconButton
                        on_click=move || menu_open.update(|v| *v = !*v)
                        title="Menu"
                        class=classes::BTN_GHOST
                    >
                        {icon::menu()}
                    </IconButton>
                    {move || {
                        if menu_open.get() {
                            view! {
                                <>
                                    <div
                                        class="fixed inset-0 z-40"
                                        on:click=move |_| close_menu()
                                    ></div>
                                    <div class=classes::MENU_DROPDOWN>
                                        // ── File ─────────────────────────────
                                        <div
                                            class="relative"
                                            on:mouseenter=move |_| file_hover.set(true)
                                            on:mouseleave=move |_| file_hover.set(false)
                                        >
                                            <button class=classes::MENU_ITEM>
                                                <span>"File"</span>
                                                {icon::chevron_right()}
                                            </button>
                                            <Dropdown
                                                show=show_file
                                                on_mouseenter=Rc::new({ let s = file_sub_hover; move || s.set(true) })
                                                on_mouseleave=Rc::new({ let s = file_sub_hover; move || s.set(false) })
                                                items=file_items.clone()
                                            />
                                        </div>

                                        // ── Export ───────────────────────────
                                        <div
                                            class="relative"
                                            on:mouseenter=move |_| export_hover.set(true)
                                            on:mouseleave=move |_| export_hover.set(false)
                                        >
                                            <button class=classes::MENU_ITEM>
                                                <span>"Export"</span>
                                                {icon::chevron_right()}
                                            </button>
                                            <Dropdown
                                                show=show_export
                                                on_mouseenter=Rc::new({ let s = export_sub_hover; move || s.set(true) })
                                                on_mouseleave=Rc::new({ let s = export_sub_hover; move || s.set(false) })
                                                items=export_items.clone()
                                            />
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
                <IconButton
                    on_click=move || shortcuts_open.update(|v| *v = !*v)
                    title="Keyboard shortcuts"
                    class=classes::BTN_GHOST
                >
                    {icon::help()}
                </IconButton>
            </div>
        </div>
    }
}


