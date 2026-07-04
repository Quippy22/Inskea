mod colors;
mod drawing;

pub use colors::ColorsPanel;
pub use drawing::DrawingPanel;

use crate::ui::icon;
use leptos::*;

#[derive(Clone, Copy, PartialEq)]
enum Category {
    Colors,
    Drawing,
}

#[component]
pub fn Dock() -> impl IntoView {
    let collapsed = create_rw_signal(true);
    let active = create_rw_signal(Category::Drawing);
    let show_panel = create_rw_signal(false);

    let toggle_collapse = move |_| {
        let was_collapsed = collapsed.get();
        collapsed.set(!was_collapsed);
        show_panel.set(was_collapsed);
    };

    let select_category = move |cat: Category| {
        if collapsed.get() {
            if active.get() == cat {
                show_panel.update(|s| *s = !*s);
            } else {
                active.set(cat);
                show_panel.set(true);
            }
        } else {
            active.set(cat);
        }
    };

    let panel_visible = move || !collapsed.get() || show_panel.get();

    let panel = move || match active.get() {
        Category::Colors => view! { <ColorsPanel/> }.into_view(),
        Category::Drawing => view! { <DrawingPanel/> }.into_view(),
    };

    view! {
        // Wrapper centers the left column. The panel is absolute so it never
        // affects layout — the left column's position stays fixed.
        <div class="fixed left-4 top-1/2 -translate-y-1/2 z-40">
            <div class="relative flex">
                // Left column — always visible
                <div class=move || {
                    if panel_visible() {
                        "flex flex-col rounded-lg bg-panel/80 backdrop-blur-sm border border-border shadow-lg pointer-events-auto"
                    } else {
                        "flex flex-col rounded-lg bg-panel/80 backdrop-blur-sm border border-border shadow-lg pointer-events-auto"
                    }
                }>
                    <button
                        class="flex items-center justify-center h-9 w-9 text-subtle hover:text-fg hover:bg-surface/50 transition-colors rounded-tl-lg"
                        on:click=toggle_collapse
                    >
                        {move || {
                            if collapsed.get() {
                                icon::chevron_right().into_view()
                            } else {
                                icon::chevron_left().into_view()
                            }
                        }}
                    </button>

                    <div class="flex flex-col border-t border-border">
                        <button
                            class=move || {
                                let base = "flex items-center justify-center h-9 w-9 transition-colors";
                                if panel_visible() && active.get() == Category::Drawing {
                                    format!("{base} text-accent bg-accent/10 border-l-2 border-accent")
                                } else {
                                    format!(
                                        "{base} text-subtle hover:text-fg hover:bg-surface/50 border-l-2 border-transparent",
                                    )
                                }
                            }
                            on:click=move |_| select_category(Category::Drawing)
                            title="Drawing"
                        >
                            {icon::pencil()}
                        </button>

                        <button
                            class=move || {
                                let base = "flex items-center justify-center h-9 w-9 transition-colors";
                                if panel_visible() && active.get() == Category::Colors {
                                    format!("{base} text-accent bg-accent/10 border-l-2 border-accent")
                                } else {
                                    format!(
                                        "{base} text-subtle hover:text-fg hover:bg-surface/50 border-l-2 border-transparent",
                                    )
                                }
                            }
                            on:click=move |_| select_category(Category::Colors)
                            title="Colors"
                        >
                            {icon::palette()}
                        </button>
                    </div>
                </div>

                // Right column — panel, positioned absolutely so the left column never shifts
                {move || {
                    if panel_visible() {
                        Some(
                            view! {
                                <div class="absolute left-[calc(100%+2px)] top-1/2 -translate-y-1/2 rounded-lg bg-panel/80 backdrop-blur-sm border border-border shadow-lg pointer-events-auto">
                                    {panel()}
                                </div>
                            },
                        )
                    } else {
                        None
                    }}
                }
            </div>
        </div>
    }
}
