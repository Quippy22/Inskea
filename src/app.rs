use crate::canvas::{Canvas, Viewport};
use crate::model::{Scene, ShapeColor};
use crate::ui::dock::{Dock, Tool};
use crate::ui::StatusBar;
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    let cursor_screen = create_rw_signal((0.0_f64, 0.0_f64));
    let cursor_world = create_rw_signal((0.0_f64, 0.0_f64));
    let viewport = create_rw_signal(Viewport::default());

    let selected_tool = create_rw_signal(Tool::Rectangle);
    let selected_color = create_rw_signal(ShapeColor::White);

    let scene = create_rw_signal(Scene::new());

    view! {
        <div class="w-screen h-screen bg-bg text-fg">
            <Canvas
                cursor_screen=cursor_screen
                cursor_world=cursor_world
                viewport=viewport
                selected_tool=selected_tool
                selected_color=selected_color
                scene=scene
            />
            <StatusBar cursor_screen=cursor_screen cursor_world=cursor_world viewport=viewport />
            <Dock selected_tool=selected_tool selected_color=selected_color />
        </div>
    }
}
