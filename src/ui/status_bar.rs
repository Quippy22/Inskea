use crate::canvas::{CanvasMode, Viewport};
use crate::ui::classes;
use crate::ui::icon;
use leptos::*;

#[component]
pub fn StatusBar(viewport: RwSignal<Viewport>, canvas_mode: RwSignal<CanvasMode>) -> impl IntoView {
    let on_home = move |_| viewport.set(Viewport::default());

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
                <button class=classes::BTN_GHOST title="Menu">
                    {icon::menu()}
                </button>
            </div>
        </div>
    }
}
