use leptos::*;
use crate::canvas::Canvas;
use crate::ui::StatusBar;

#[component]
pub fn App() -> impl IntoView {
    let cursor_screen = create_rw_signal((0.0_f64, 0.0_f64));
    let cursor_world = create_rw_signal((0.0_f64, 0.0_f64));

    view! {
        <div class="w-screen h-screen bg-bg text-fg">
            <Canvas cursor_screen=cursor_screen cursor_world=cursor_world/>
            <StatusBar cursor_screen=cursor_screen cursor_world=cursor_world/>
        </div>
    }
}
