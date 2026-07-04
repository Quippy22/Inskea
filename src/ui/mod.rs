use leptos::*;

#[component]
pub fn StatusBar(
    cursor_screen: RwSignal<(f64, f64)>,
    cursor_world: RwSignal<(f64, f64)>,
) -> impl IntoView {
    view! {
        <div class="fixed top-0 inset-x-0 flex justify-center pointer-events-none z-50 p-2">
            <div class="flex gap-6 rounded-lg bg-surface/80 px-4 py-1.5 text-xs font-mono text-subtle backdrop-blur-sm border border-border pointer-events-auto">
                <span>
                    "screen: "
                    <span class="text-accent">
                        {move || {
                            format!("{:.1}, {:.1}", cursor_screen.get().0, cursor_screen.get().1)
                        }}
                    </span>
                </span>
                <span>
                    "world: "
                    <span class="text-green">
                        {move || {
                            format!("{:.1}, {:.1}", cursor_world.get().0, cursor_world.get().1)
                        }}
                    </span>
                </span>
            </div>
        </div>
    }
}
