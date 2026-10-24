use crate::canvas::settings::{CanvasSettings, Theme};
use crate::ui::{icon, styles};
use leptos::*;

/// Modal theme chooser opened from the settings window. Each card previews the
/// real palette, and picking one applies it immediately.
#[component]
pub fn ThemePicker(open: RwSignal<bool>, settings: RwSignal<CanvasSettings>) -> impl IntoView {
    let close = move || open.set(false);

    move || {
        if !open.get() {
            return view! {}.into_view();
        }

        let cards = Theme::ALL
            .iter()
            .map(|&theme| {
                let p = theme.palette();
                let selected = Signal::derive(move || settings.get().theme == theme);

                let card_class = move || {
                    let base = "group text-left rounded-lg border p-3 transition-colors";
                    if selected.get() {
                        format!("{base} border-accent bg-accent/5 ring-2 ring-accent/30")
                    } else {
                        format!("{base} border-border bg-surface/30 hover:border-subtle hover:bg-surface/60")
                    }
                };

                view! {
                    <button
                        class=card_class
                        on:click=move |_| settings.update(|s| s.theme = theme)
                        aria-pressed=move || selected.get().to_string()
                    >
                        <div
                            class="h-24 rounded-md border p-3 flex flex-col justify-between"
                            style=format!("background:{};border-color:{}", p.canvas, p.border)
                        >
                            <div class="flex items-center gap-1.5">
                                <span
                                    class="h-2 w-7 rounded-full"
                                    style=format!("background:{}", p.accent)
                                ></span>
                                <span
                                    class="h-2 w-12 rounded-full"
                                    style=format!("background:{};opacity:0.5", p.fg)
                                ></span>
                                <span
                                    class="ml-auto h-3 w-3 rounded-full border"
                                    style=format!(
                                        "background:{};border-color:{}",
                                        p.panel, p.border,
                                    )
                                ></span>
                            </div>
                            <div class="flex items-end gap-1.5">
                                <div
                                    class="flex-1 rounded-md border p-2"
                                    style=format!(
                                        "background:{};border-color:{}",
                                        p.panel, p.border,
                                    )
                                >
                                    <div
                                        class="h-1.5 w-3/4 rounded-full"
                                        style=format!("background:{};opacity:0.55", p.fg)
                                    ></div>
                                    <div
                                        class="mt-1.5 h-1.5 w-1/2 rounded-full"
                                        style=format!("background:{}", p.accent)
                                    ></div>
                                </div>
                                <div
                                    class="h-7 w-7 rounded-md border"
                                    style=format!(
                                        "background:{};border-color:{}",
                                        p.canvas, p.accent,
                                    )
                                ></div>
                            </div>
                        </div>

                        <div class="mt-3 flex items-start justify-between gap-2">
                            <div>
                                <div class="text-sm font-medium text-fg">{theme.label()}</div>
                                <div class="mt-0.5 text-[11px] leading-snug text-subtle">
                                    {theme.description()}
                                </div>
                            </div>
                            <span class="shrink-0 text-accent" class:hidden=move || !selected.get()>
                                {icon::check()}
                            </span>
                        </div>
                    </button>
                }
                .into_view()
            })
            .collect_view();

        view! {
            <>
                <div
                    class="fixed inset-0 z-[60] bg-black/40"
                    on:click=move |_| close()
                ></div>
                <div class="fixed inset-0 z-[61] grid place-items-center pointer-events-none">
                    <div
                        class="pointer-events-auto w-full max-w-3xl mx-4 rounded-xl bg-panel/95 backdrop-blur-sm border border-border shadow-2xl p-6 max-h-[85vh] overflow-y-auto"
                        role="dialog"
                        aria-modal="true"
                        aria-label="Theme"
                    >
                        <div class="flex items-start justify-between gap-4">
                            <div class="flex items-center gap-2.5">
                                <span class="text-accent">{icon::palette()}</span>
                                <div>
                                    <h2 class="text-lg font-semibold text-fg">"Theme"</h2>
                                    <p class="mt-1 text-xs text-subtle">
                                        "Pick the palette for the canvas and interface."
                                    </p>
                                </div>
                            </div>
                            <button
                                class="text-subtle hover:text-fg transition-colors shrink-0"
                                on:click=move |_| close()
                                title="Close"
                                aria-label="Close theme picker"
                            >
                                {icon::close()}
                            </button>
                        </div>

                        <div class="mt-5 grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
                            {cards}
                        </div>

                        <div class="mt-5 flex items-center justify-between gap-4">
                            <span class="text-[11px] text-subtle">
                                "Changes apply right away."
                            </span>
                            <button class=styles::BTN_TEXT_ACTIVE on:click=move |_| close()>
                                "Done"
                            </button>
                        </div>
                    </div>
                </div>
            </>
        }
        .into_view()
    }
}
