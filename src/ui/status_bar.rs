use crate::canvas::{CanvasMode, Viewport};
use crate::ui::icon;
use leptos::*;

#[component]
pub fn StatusBar(viewport: RwSignal<Viewport>, canvas_mode: RwSignal<CanvasMode>) -> impl IntoView {
    let on_home = move |_| {
        viewport.set(Viewport::default());
    };

    let btn = move |mode: CanvasMode| -> String {
        let base = "flex items-center justify-center h-8 w-8 rounded-md transition-colors";
        if canvas_mode.get() == mode {
            format!("{base} text-accent bg-accent/10")
        } else {
            format!("{base} text-subtle hover:text-fg hover:bg-surface/50")
        }
    };

    view! {
        <div class="fixed top-4 inset-x-0 flex justify-center pointer-events-none z-50">
            <div class="flex items-center gap-0.5 rounded-lg bg-panel/80 backdrop-blur-sm border border-border shadow-[0_6px_12px_-4px_rgba(122,162,247,0.35)] pointer-events-auto p-0.5">
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
                <div class="w-px h-5 bg-border mx-1" />
                <button
                    class="flex items-center justify-center h-8 w-8 rounded-md text-subtle hover:text-fg hover:bg-surface/50 transition-colors"
                    on:click=on_home
                    title="Home"
                >
                    {icon::home()}
                </button>
                <button
                    class="flex items-center justify-center h-8 w-8 rounded-md text-subtle hover:text-fg hover:bg-surface/50 transition-colors"
                    title="Undo"
                >
                    {icon::undo()}
                </button>
                <button
                    class="flex items-center justify-center h-8 w-8 rounded-md text-subtle hover:text-fg hover:bg-surface/50 transition-colors"
                    title="Redo"
                >
                    {icon::redo()}
                </button>
                <div class="w-px h-5 bg-border mx-1" />
                <button
                    class="flex items-center justify-center h-8 w-8 rounded-md text-subtle hover:text-fg hover:bg-surface/50 transition-colors"
                    title="Menu"
                >
                    {icon::menu()}
                </button>
            </div>
        </div>
    }
}
