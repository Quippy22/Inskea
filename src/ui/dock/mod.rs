mod colors;
mod drawing;
mod group;
mod pages;

pub use colors::ColorsPanel;
pub use drawing::DrawingPanel;
pub use group::GroupPanel;
pub use pages::PagesPanel;

use crate::model::ShapeColor;
use crate::ui::icon;
use leptos::*;

#[derive(Clone, Copy, PartialEq)]
enum Category {
    Drawing,
    Colors,
    Group,
    Pages,
}

#[component]
pub fn Dock() -> impl IntoView {
    let collapsed = create_rw_signal(true);
    let active = create_rw_signal(Category::Drawing);
    let show_panel = create_rw_signal(false);
    let eraser_active = create_rw_signal(false);
    let selected_color = create_rw_signal(ShapeColor::White);

    let toggle_collapse = move |_| {
        let was_collapsed = collapsed.get();
        collapsed.set(!was_collapsed);
        show_panel.set(was_collapsed);
        eraser_active.set(false);
    };

    let select_category = move |cat: Category| {
        eraser_active.set(false);
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
        Category::Drawing => view! { <DrawingPanel /> }.into_view(),
        Category::Colors => view! { <ColorsPanel selected_color=selected_color /> }.into_view(),
        Category::Group => view! { <GroupPanel /> }.into_view(),
        Category::Pages => view! { <PagesPanel /> }.into_view(),
    };

    let btn_class = move |cat: Category| -> String {
        let base = "flex items-center justify-center h-9 w-9 transition-colors";
        if !eraser_active.get() && panel_visible() && active.get() == cat {
            format!("{base} text-accent bg-accent/10 border-l-2 border-accent")
        } else {
            format!("{base} text-subtle hover:text-fg hover:bg-surface/50 border-l-2 border-transparent")
        }
    };

    let toggle_eraser = move |_| {
        let new = !eraser_active.get();
        eraser_active.set(new);
        if new {
            show_panel.set(false);
        }
    };

    view! {
        // Wrapper centers everything
        <div class="fixed left-4 top-1/2 -translate-y-1/2 z-40 flex flex-col items-center gap-0.5">
            // Collapse button — separate floating object above the dock
            <div class="rounded-lg bg-panel/80 backdrop-blur-sm border border-border shadow-lg pointer-events-auto">
                <button
                    class="flex items-center justify-center h-9 w-9 text-subtle hover:text-fg hover:bg-surface/50 transition-colors"
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
            </div>

            // Main dock
            <div class="relative flex">
                <div class="flex flex-col rounded-lg bg-panel/80 backdrop-blur-sm border border-border shadow-lg pointer-events-auto">
                    <button
                        class=move || btn_class(Category::Drawing)
                        on:click=move |_| select_category(Category::Drawing)
                        title="Drawing"
                    >
                        {icon::pencil()}
                    </button>
                    <button
                        class=move || btn_class(Category::Colors)
                        on:click=move |_| select_category(Category::Colors)
                        title="Colors"
                        style=move || format!("color: {}", selected_color.get().to_hex())
                    >
                        {icon::palette()}
                    </button>
                    <button
                        class=move || btn_class(Category::Group)
                        on:click=move |_| select_category(Category::Group)
                        title="Group"
                    >
                        {icon::group()}
                    </button>
                    <button
                        class=move || btn_class(Category::Pages)
                        on:click=move |_| select_category(Category::Pages)
                        title="Pages"
                    >
                        {icon::pages()}
                    </button>

                    // Eraser
                    <button
                        class=move || {
                            let base = "flex items-center justify-center h-9 w-9 transition-colors";
                            if eraser_active.get() {
                                format!("{base} text-accent bg-accent/10 border-l-2 border-accent")
                            } else {
                                format!(
                                    "{base} text-subtle hover:text-fg hover:bg-surface/50 border-l-2 border-transparent",
                                )
                            }
                        }
                        on:click=toggle_eraser
                        title="Eraser"
                    >
                        {icon::eraser()}
                    </button>
                </div>

                // Right column — panel
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
                    }
                }}
            </div>
        </div>
    }
}
