use std::rc::Rc;

use crate::canvas::{Canvas, CanvasMode, Viewport};
use crate::model::{Scene, ShapeColor};
use crate::tauri_bridge;
use crate::ui::dock::{Dock, Tool};
use crate::ui::settings::{from_toml, to_toml, CanvasBg, CenterStyle, GridSize, GridStyle};
use crate::ui::{SettingsPanel, ToolBar};
use leptos::ev;
use leptos::*;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn App() -> impl IntoView {
    let cursor_screen = create_rw_signal((0.0_f64, 0.0_f64));
    let cursor_world = create_rw_signal((0.0_f64, 0.0_f64));
    let viewport = create_rw_signal(Viewport::default());

    let selected_tool = create_rw_signal(Tool::Rectangle);
    let selected_color = create_rw_signal(ShapeColor::White);
    let canvas_mode = create_rw_signal(CanvasMode::Hand);

    let scene = create_rw_signal(Scene::new());
    let eraser_active = create_rw_signal(false);

    // Crop-export state: when active the canvas lets you drag a rectangle,
    // and on release the region is exported via this callback.
    let export_crop_active = create_rw_signal(false);
    let on_crop_export = create_rw_signal::<Option<Rc<dyn Fn((f64, f64, f64, f64))>>>(None);

    let center_style = create_rw_signal(CenterStyle::Crosshair);
    let grid_style = create_rw_signal(GridStyle::Dot);
    let grid_size = create_rw_signal(GridSize::Px30);
    let autosave = create_rw_signal(false);
    let canvas_bg = create_rw_signal(CanvasBg::Dark);

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

    let _ = window_event_listener(ev::keydown, move |ev: ev::KeyboardEvent| {
        if ev.ctrl_key() && ev.key() == "z" {
            if ev.shift_key() {
                do_redo();
            } else {
                do_undo();
            }
        }
    });

    // ── Settings persistence ───────────────────────────────────────────────
    let initialized = create_rw_signal(false);
    let is_tauri = tauri_bridge::is_tauri();

    if is_tauri {
        spawn_local(async move {
            if let Ok(content) = tauri_bridge::load_settings().await {
                if let Some((cs, gs, gz, auto, bg)) = from_toml(&content) {
                    center_style.set(cs);
                    grid_style.set(gs);
                    grid_size.set(gz);
                    autosave.set(auto);
                    canvas_bg.set(bg);
                }
            }
            initialized.set(true);
        });
    } else {
        initialized.set(true);
    }

    create_effect(move |_| {
        let _ = (
            center_style.get(),
            grid_style.get(),
            grid_size.get(),
            autosave.get(),
            canvas_bg.get(),
        );
        if initialized.get() && is_tauri {
            let content = to_toml(
                center_style.get(),
                grid_style.get(),
                grid_size.get(),
                autosave.get(),
                canvas_bg.get(),
            );
            spawn_local(async move {
                let _ = tauri_bridge::save_settings(&content).await;
            });
        }
    });

    view! {
        <div class=move || {
            let bg = if canvas_bg.get() == CanvasBg::Dark { "bg-bg" } else { "bg-white" };
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
                center_style=center_style
                grid_style=grid_style
                grid_size=grid_size
                push_snapshot=push_snapshot
                export_crop_active=export_crop_active
                on_crop_export=on_crop_export
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
            />
            <SettingsPanel
                center_style=center_style
                grid_style=grid_style
                grid_size=grid_size
                autosave=autosave
                canvas_bg=canvas_bg
            />
        </div>
    }
}
