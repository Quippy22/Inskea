use std::rc::Rc;

use crate::canvas::{Canvas, CanvasMode, CropExportCallback, Viewport};
use crate::model::{ElementId, Scene, ShapeColor};
use crate::tauri_bridge;
use crate::ui::dock::{Dock, Tool};
use crate::canvas::settings::{CanvasBg, CanvasSettings, CenterStyle, GridSize, GridStyle};
use crate::ui::settings::{from_toml, to_toml};
use crate::ui::{SettingsPanel, ToolBar};
use leptos::ev;
use leptos::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

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

    let push_snapshot2 = push_snapshot.clone();
    let handle_shortcut = move |key: &str, ctrl: bool, _shift: bool| {
        match (ctrl, key) {
            (true, "z") => {
                if _shift { do_redo(); } else { do_undo(); }
            }
            (false, "s") => canvas_mode.set(CanvasMode::Select),
            (false, "a") => canvas_mode.set(CanvasMode::Pan),
            (false, "e") => canvas_mode.set(CanvasMode::Erase),
            (false, "d") => canvas_mode.set(CanvasMode::Draw),
            (false, "Escape") => {}  // handled inside Canvas component
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
        </div>
    }
}
