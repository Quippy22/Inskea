use std::rc::Rc;

use crate::canvas::{Canvas, CanvasMode, CropExportCallback, Viewport};
use crate::model::{Element, ElementId, Offset, Scene, ShapeColor};
use crate::skea;
use crate::tauri_bridge;
use crate::ui::dock::{Dock, Tool};
use crate::canvas::settings::{CanvasBg, CanvasSettings, CenterStyle, GridSize, GridStyle};
use crate::ui::settings::{from_toml, to_toml};
use crate::ui::{SettingsPanel, ToolBar};
use leptos::ev;
use leptos::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

const MODE_SHORTCUTS: &[(&str, &str)] = &[
    ("s", "Select mode"),
    ("a", "Pan mode"),
    ("d", "Draw mode"),
    ("e", "Erase mode"),
    ("Esc", "Deselect / cancel"),
    ("Delete", "Delete selection"),
];

const TOOL_SHORTCUTS: &[(&str, &str)] = &[
    ("1", "Rectangle"),
    ("2", "Ellipse"),
    ("3", "Line"),
    ("4", "Arrow"),
    ("5", "Text"),
    ("f", "Freehand"),
];

const FILE_SHORTCUTS: &[(&str, &str)] = &[
    ("Ctrl+Z", "Undo"),
    ("Ctrl+Shift+Z", "Redo"),
    ("Ctrl+C", "Copy"),
    ("Ctrl+V", "Paste"),
    ("Ctrl+D", "Duplicate"),
    ("Ctrl+S", "Save"),
    ("Ctrl+Shift+S", "Save As"),
    ("Ctrl+O", "Open"),
    ("Ctrl+N", "New document"),
];

#[component]
pub fn App() -> impl IntoView {
    let cursor_screen = create_rw_signal((0.0_f64, 0.0_f64));
    let cursor_world = create_rw_signal((0.0_f64, 0.0_f64));
    let viewport = create_rw_signal(Viewport::default());

    let selected_tool = create_rw_signal(Tool::Rectangle);
    let selected_color = create_rw_signal(ShapeColor::White);
    let canvas_mode = create_rw_signal(CanvasMode::Select);

    let scene = create_rw_signal(Scene::new());
    let selected_ids = create_rw_signal(Vec::<ElementId>::new());
    let eraser_active = create_rw_signal(false);
    let shortcuts_open = create_rw_signal(false);

    // Crop-export state: when active the canvas lets you drag a rectangle,
    // and on release the region is exported via this callback.
    let export_crop_active = create_rw_signal(false);
    let on_crop_export = create_rw_signal::<Option<CropExportCallback>>(None);

    let settings = create_rw_signal(CanvasSettings {
        center_style: CenterStyle::Crosshair,
        grid_style: GridStyle::Dot,
        grid_size: GridSize::Px30,
        autosave: false,
        canvas_bg: CanvasBg::Dark,
    });

    // ── Clipboard ──────────────────────────────────────────────────────────
    let clipboard = create_rw_signal(Vec::<Element>::new());

    // ── Undo / Redo ────────────────────────────────────────────────────────
    let undo_stack = create_rw_signal(Vec::<Scene>::new());
    let redo_stack = create_rw_signal(Vec::<Scene>::new());

    let push_snapshot = Rc::new(move || {
        undo_stack.update(|s| {
            s.push(scene.get());
            if s.len() > 100 {
                s.remove(0);
            }
        });
        redo_stack.set(Vec::new());
    });

    let do_undo = move || {
        let mut prev = None;
        undo_stack.update(|s| prev = s.pop());
        if let Some(prev) = prev {
            let current = scene.get();
            scene.set(prev);
            redo_stack.update(|s| s.push(current));
        }
    };

    let do_redo = move || {
        let mut next = None;
        redo_stack.update(|s| next = s.pop());
        if let Some(next) = next {
            let current = scene.get();
            scene.set(next);
            undo_stack.update(|s| s.push(current));
        }
    };

    let can_undo = Signal::derive(move || !undo_stack.get().is_empty());
    let can_redo = Signal::derive(move || !redo_stack.get().is_empty());

    // ── Keyboard shortcuts ─────────────────────────────────────────────────
    let is_text_input = |target: &web_sys::EventTarget| -> bool {
        let node = target.clone().dyn_into::<web_sys::Node>();
        if let Ok(node) = node {
            if let Some(el) = node.dyn_ref::<web_sys::HtmlElement>() {
                let tag = el.tag_name().to_lowercase();
                if tag == "input" || tag == "textarea" {
                    return true;
                }
                if el.get_attribute("contenteditable").as_deref() == Some("true") {
                    return true;
                }
            }
        }
        false
    };

    // ── Save / Open state ──────────────────────────────────────────────────
    let saved_path: RwSignal<Option<String>> = create_rw_signal(None);

    let do_save = {
        let saved_path = saved_path;
        move || {
            let path = saved_path.get();
            if let Some(path) = path {
                let s = scene.get();
                spawn_local(async move {
                    let c = skea::save_to_string(&s);
                    let _ = tauri_bridge::save_skea(&path, &c).await;
                });
            } else {
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
        }
    };

    let do_new = move || {
        saved_path.set(None);
        scene.set(Scene::new());
    };

    let do_open = move || {
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
    };

    let do_save_as = move || {
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
    };

    let push_snapshot2 = push_snapshot.clone();
    let handle_shortcut = move |key: &str, ctrl: bool, shift: bool| {
        match (ctrl, key) {
            (true, "z") | (true, "Z") => {
                if shift { do_redo(); } else { do_undo(); }
            }
            (false, "s") => canvas_mode.set(CanvasMode::Select),
            (false, "a") => canvas_mode.set(CanvasMode::Pan),
            (true, "d") => {
                let ids = selected_ids.get();
                if !ids.is_empty() {
                    push_snapshot2();
                    let mut new_ids = Vec::new();
                    scene.update(|s| {
                        let elements = s.elements().to_vec();
                        for id in &ids {
                            if let Some(el) = elements.iter().find(|e| e.id() == *id) {
                                let mut clone = el.clone();
                                clone.offset(20.0, 20.0);
                                clone.data_mut().id = 0;
                                s.add_element(clone);
                                new_ids.push(s.next_id - 1);
                            }
                        }
                    });
                    selected_ids.set(new_ids);
                }
            }
            (true, "n") | (true, "N") => do_new(),
            (true, "o") | (true, "O") => do_open(),
            (true, "s") | (true, "S") => {
                if shift { do_save_as(); } else { do_save(); }
            }
            (false, "?") => shortcuts_open.update(|v| *v = !*v),
            (false, "e") => canvas_mode.set(CanvasMode::Erase),
            (false, "1") => { selected_tool.set(Tool::Rectangle); canvas_mode.set(CanvasMode::Draw); }
            (false, "2") => { selected_tool.set(Tool::Ellipse); canvas_mode.set(CanvasMode::Draw); }
            (false, "3") => { selected_tool.set(Tool::Line); canvas_mode.set(CanvasMode::Draw); }
            (false, "4") => { selected_tool.set(Tool::Arrow); canvas_mode.set(CanvasMode::Draw); }
            (false, "5") => { selected_tool.set(Tool::Text); canvas_mode.set(CanvasMode::Draw); }
            (false, "f") => { selected_tool.set(Tool::Freehand); canvas_mode.set(CanvasMode::Draw); }
            (false, "d") => canvas_mode.set(CanvasMode::Draw),
            (false, "Escape") => {}  // handled inside Canvas component
            (true, "c") => {
                let ids = selected_ids.get();
                let els = scene.with(|s| {
                    s.elements().iter().filter(|e| ids.contains(&e.id())).cloned().collect::<Vec<_>>()
                });
                clipboard.set(els);
                web_sys::console::log_1(&"Copied".into());
            }
            (true, "v") => {
                let els = clipboard.get();
                if !els.is_empty() {
                    push_snapshot2();
                    let mut new_ids = Vec::new();
                    scene.update(|s| {
                        for el in els {
                            let mut el = el;
                            el.data_mut().id = 0; // will be reassigned
                            s.add_element(el);
                            new_ids.push(s.next_id - 1);
                        }
                    });
                    selected_ids.set(new_ids);
                }
            }
            (false, "Delete") | (false, "Backspace") => {
                let ids = selected_ids.get();
                if !ids.is_empty() {
                    push_snapshot2();
                    scene.update(|s| {
                        for id in &ids {
                            s.remove_by_id(*id);
                        }
                    });
                    selected_ids.set(Vec::new());
                }
            }
            _ => {}
        }
    };

    let _ = window_event_listener(ev::keydown, move |ev: ev::KeyboardEvent| {
        if let Some(target) = ev.target() {
            if is_text_input(&target) {
                return;
            }
        }
        let key = ev.key();
        let ctrl = ev.ctrl_key();
        let shift = ev.shift_key();
        handle_shortcut(&key, ctrl, shift);
    });

    // ── Settings persistence ───────────────────────────────────────────────
    let initialized = create_rw_signal(false);
    let is_tauri = tauri_bridge::is_tauri();

    if is_tauri {
        spawn_local(async move {
            if let Ok(content) = tauri_bridge::load_settings().await {
                if let Some((cs, gs, gz, auto, bg)) = from_toml(&content) {
                    settings.set(CanvasSettings {
                        center_style: cs,
                        grid_style: gs,
                        grid_size: gz,
                        autosave: auto,
                        canvas_bg: bg,
                    });
                }
            }
            initialized.set(true);
        });
    } else {
        initialized.set(true);
    }

    create_effect(move |_| {
        let s = settings.get();
        let _ = s;
        if initialized.get() && is_tauri {
            let content = to_toml(s.center_style, s.grid_style, s.grid_size, s.autosave, s.canvas_bg);
            spawn_local(async move {
                let _ = tauri_bridge::save_settings(&content).await;
            });
        }
    });

    view! {
        <div class=move || {
            let bg = if settings.get().canvas_bg == CanvasBg::Dark { "bg-bg" } else { "bg-white" };
            format!("w-screen h-screen {bg} text-fg")
        }>
            <Canvas
                cursor_screen=cursor_screen
                cursor_world=cursor_world
                viewport=viewport
                selected_tool=selected_tool
                selected_color=selected_color
                canvas_mode=canvas_mode
                scene=scene
                eraser_active=eraser_active
                settings=settings
                push_snapshot=push_snapshot
                export_crop_active=export_crop_active
                on_crop_export=on_crop_export
                selected_ids=selected_ids
            />
            <ToolBar
                scene=scene
                viewport=viewport
                canvas_mode=canvas_mode
                on_undo=do_undo
                on_redo=do_redo
                can_undo=can_undo
                can_redo=can_redo
                export_crop_active=export_crop_active
                on_crop_export=on_crop_export
                shortcuts_open=shortcuts_open
            />
            <Dock
                selected_tool=selected_tool
                selected_color=selected_color
                canvas_mode=canvas_mode
                eraser_active=eraser_active
                scene=scene
                selected_ids=selected_ids
            />
            <SettingsPanel
                settings=settings
            />

            {move || {
                if shortcuts_open.get() {
                    view! {
                        <>
                            <div
                                class="fixed inset-0 z-40"
                                on:click=move |_| shortcuts_open.set(false)
                            ></div>
                            <div class="fixed inset-0 z-50 grid place-items-center pointer-events-none">
                                <div class="pointer-events-auto rounded-lg bg-panel/95 backdrop-blur-sm border border-border shadow-xl py-5 px-7 max-w-2xl w-full mx-4 max-h-[85vh] overflow-y-auto">
                                    <button
                                        class="absolute top-3 right-3 text-subtle hover:text-fg transition-colors"
                                        on:click=move |_| shortcuts_open.set(false)
                                        title="Close"
                                    >
                                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="w-5 h-5">
                                            <line x1="18" y1="6" x2="6" y2="18" />
                                            <line x1="6" y1="6" x2="18" y2="18" />
                                        </svg>
                                    </button>
                                    <h2 class="text-lg font-semibold mb-4">"Keyboard Shortcuts"</h2>
                                    <div class="flex gap-10">
                                        <div>
                                            <h3 class="text-xs font-semibold text-subtle uppercase tracking-wider mb-2">"Shapes"</h3>
                                            <table class="text-sm">
                                                <tbody>
                                                    {TOOL_SHORTCUTS.iter().map(|(key, desc)| {
                                                        view! {
                                                            <tr>
                                                                <td class="py-1 pr-5 font-mono text-accent whitespace-nowrap">{*key}</td>
                                                                <td class="py-1 text-fg whitespace-nowrap">{*desc}</td>
                                                            </tr>
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </tbody>
                                            </table>
                                        </div>
                                        <div>
                                            <h3 class="text-xs font-semibold text-subtle uppercase tracking-wider mb-2">"Modes"</h3>
                                            <table class="text-sm">
                                                <tbody>
                                                    {MODE_SHORTCUTS.iter().map(|(key, desc)| {
                                                        view! {
                                                            <tr>
                                                                <td class="py-1 pr-5 font-mono text-accent whitespace-nowrap">{*key}</td>
                                                                <td class="py-1 text-fg whitespace-nowrap">{*desc}</td>
                                                            </tr>
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </tbody>
                                            </table>
                                        </div>
                                        <div>
                                            <h3 class="text-xs font-semibold text-subtle uppercase tracking-wider mb-2">"File & Edit"</h3>
                                            <table class="text-sm">
                                                <tbody>
                                                    {FILE_SHORTCUTS.iter().map(|(key, desc)| {
                                                        view! {
                                                            <tr>
                                                                <td class="py-1 pr-5 font-mono text-accent whitespace-nowrap">{*key}</td>
                                                                <td class="py-1 text-fg whitespace-nowrap">{*desc}</td>
                                                            </tr>
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </tbody>
                                            </table>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </>
                    }.into_view()
                } else {
                    view! {}.into_view()
                }
            }}
        </div>
    }
}
