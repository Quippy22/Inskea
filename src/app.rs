use crate::canvas::{Canvas, Viewport};
use crate::ui::dock::Dock;
use crate::ui::StatusBar;
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    let cursor_screen = create_rw_signal((0.0_f64, 0.0_f64));
    let cursor_world = create_rw_signal((0.0_f64, 0.0_f64));
    let viewport = create_rw_signal(Viewport::default());

    view! {
        <div class="w-screen h-screen bg-bg text-fg">
            <Canvas cursor_screen=cursor_screen cursor_world=cursor_world viewport=viewport/>
            <StatusBar cursor_screen=cursor_screen cursor_world=cursor_world viewport=viewport/>
            <Dock/>
        </div>
    }
}
