use crate::canvas::Viewport;
use leptos::*;

#[component]
pub fn StatusBar(
    cursor_screen: RwSignal<(f64, f64)>,
    cursor_world: RwSignal<(f64, f64)>,
    viewport: RwSignal<Viewport>,
) -> impl IntoView {
    view! {
        <div class="fixed top-0 inset-x-0 flex justify-center pointer-events-none z-50 p-4">
            <div class="flex gap-6 rounded-lg bg-surface/80 px-6 py-2.5 text-sm font-mono text-subtle backdrop-blur-sm border border-border pointer-events-auto select-none">
                <span>
                    "screen: "
                    <span class="text-accent">
                        {move || format!("{:.1}, {:.1}", cursor_screen.get().0, cursor_screen.get().1)}
                    </span>
                </span>
                <span>
                    "world: "
                    <span class="text-green">
                        {move || format!("{:.1}, {:.1}", cursor_world.get().0, cursor_world.get().1)}
                    </span>
                </span>
                <span>
                    "zoom: "
                    <span class="text-yellow">
                        {move || format!("{:.0}%", viewport.get().zoom * 100.0)}
                    </span>
                </span>
            </div>
        </div>
    }
}
