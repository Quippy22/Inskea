use crate::canvas::{Canvas, CanvasMode, Viewport};
use crate::model::{Scene, ShapeColor};
use crate::tauri_bridge;
use crate::ui::dock::{Dock, Tool};
use crate::ui::settings::{from_toml, to_toml, CenterStyle, GridSize, GridStyle};
use crate::ui::{SettingsPanel, ToolBar};
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

    let center_style = create_rw_signal(CenterStyle::Crosshair);
    let grid_style = create_rw_signal(GridStyle::Dot);
    let grid_size = create_rw_signal(GridSize::Px30);
    let autosave = create_rw_signal(false);

    let initialized = create_rw_signal(false);
    let is_tauri = tauri_bridge::is_tauri();

    if is_tauri {
        spawn_local(async move {
            if let Ok(content) = tauri_bridge::load_settings().await {
                if let Some((cs, gs, gz, auto)) = from_toml(&content) {
                    center_style.set(cs);
                    grid_style.set(gs);
                    grid_size.set(gz);
                    autosave.set(auto);
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
        );
        if initialized.get() && is_tauri {
            let content = to_toml(
                center_style.get(),
                grid_style.get(),
                grid_size.get(),
                autosave.get(),
            );
            spawn_local(async move {
                let _ = tauri_bridge::save_settings(&content).await;
            });
        }
    });

    view! {
        <div class="w-screen h-screen bg-bg text-fg">
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
            />
            <ToolBar scene=scene viewport=viewport canvas_mode=canvas_mode />
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
            />
        </div>
    }
}
