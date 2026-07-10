/// Floating tool dock with categories and panels.
///
/// The dock sits on the left side of the screen and provides access to
/// drawing tools, colours, grouping, and page management via expandable
/// category panels.
mod colors;
mod drawing;
mod group;
mod pages;

pub use colors::ColorsPanel;
pub use drawing::{DrawingPanel, Tool};
pub use group::GroupPanel;
pub use pages::PagesPanel;

use crate::canvas::CanvasMode;
use crate::model::elements::path::CurveMode;
use crate::model::{Element, ElementId, PathPoints, Scene, ShapeColor};
use crate::ui::classes;
use crate::ui::icon;
use leptos::*;

/// Dock category variant used to select which panel is shown.
#[derive(Clone, Copy, PartialEq)]
enum Category {
    Drawing,
    Colors,
    Group,
    Pages,
}

/// Floating dock with tool categories, colour palette, and eraser.
///
/// Clicking a category while collapsed expands the dock and opens that
/// category's panel. The eraser button sits at the bottom of the category
/// list with the same highlight style.
///
/// When exactly one `Line` or `Arrow` is selected, a curve-mode toggle
/// button appears that switches between Straight and Curved rendering.
#[component]
pub fn Dock(
    selected_tool: RwSignal<Tool>,
    selected_color: RwSignal<ShapeColor>,
    canvas_mode: RwSignal<CanvasMode>,
    eraser_active: RwSignal<bool>,
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
) -> impl IntoView {
    let collapsed = create_rw_signal(true);
    let active = create_rw_signal(Category::Drawing);
    let show_panel = create_rw_signal(false);

    let toggle_collapse = move |_| {
        let was_collapsed = collapsed.get();
        collapsed.set(!was_collapsed);
        show_panel.set(was_collapsed);
        eraser_active.set(false);
    };

    let select_category = move |cat: Category| {
        eraser_active.set(false);
        if collapsed.get() {
            active.set(cat);
            collapsed.set(false);
            show_panel.set(true);
        } else {
            active.set(cat);
        }
    };

    let panel_visible = move || !collapsed.get() || show_panel.get();

    let panel = move || match active.get() {
        Category::Drawing => {
            view! { <DrawingPanel selected_tool=selected_tool canvas_mode=canvas_mode /> }
                .into_view()
        }
        Category::Colors => view! { <ColorsPanel selected_color=selected_color /> }.into_view(),
        Category::Group => view! { <GroupPanel /> }.into_view(),
        Category::Pages => view! { <PagesPanel /> }.into_view(),
    };

    let btn_class = move |cat: Category| -> &'static str {
        if !eraser_active.get() && panel_visible() && active.get() == cat {
            classes::BTN_CAT_ACTIVE
        } else {
            classes::BTN_CAT_INACTIVE
        }
    };

    let toggle_eraser = move |_| {
        let new = !eraser_active.get();
        eraser_active.set(new);
        if new {
            collapsed.set(true);
            show_panel.set(false);
            canvas_mode.set(CanvasMode::Draw);
        }
    };

    // Determine whether a single Line or Arrow is selected and its curve mode
    let path_info = move || -> Option<CurveMode> {
        let ids = selected_ids.get();
        if ids.len() != 1 {
            return None;
        }
        let els = scene.get().elements;
        let el = els.iter().find(|e| e.id() == ids[0])?;
        el.path_points()?; // ensure it's a path element
        match el {
            Element::Line(l) => Some(l.curve_mode),
            Element::Arrow(a) => Some(a.curve_mode),
            _ => None,
        }
    };

    let toggle_curve_mode = move |_| {
        let ids = selected_ids.get();
        if ids.len() != 1 {
            return;
        }
        let id = ids[0];
        scene.update(|s| {
            if let Some(el) = s.elements.iter_mut().find(|e| e.id() == id) {
                match el {
                    Element::Line(l) => {
                        l.curve_mode = match l.curve_mode {
                            CurveMode::Straight => CurveMode::Curved,
                            CurveMode::Curved => CurveMode::Straight,
                        };
                    }
                    Element::Arrow(a) => {
                        a.curve_mode = match a.curve_mode {
                            CurveMode::Straight => CurveMode::Curved,
                            CurveMode::Curved => CurveMode::Straight,
                        };
                    }
                    _ => {}
                }
            }
        });
    };

    view! {
        // Wrapper centers everything
        <div class=classes::CONTAINER_DOCK>
            // Collapse button — separate floating object above the dock
            <div class=classes::PANEL>
                <button class=classes::BTN_COLLAPSE on:click=toggle_collapse>
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
                <div class=classes::PANEL>
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
                        style=move || {
                            if selected_color.get() != ShapeColor::White {
                                format!("color: {}", selected_color.get().to_hex())
                            } else {
                                String::new()
                            }
                        }
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
                            if eraser_active.get() {
                                classes::BTN_CAT_ACTIVE
                            } else {
                                classes::BTN_CAT_INACTIVE
                            }
                        }
                        on:click=toggle_eraser
                        title="Eraser"
                    >
                        {icon::eraser()}
                    </button>

                    // Curve mode toggle — only shown when a single Line/Arrow is selected
                    {move || {
                        path_info().map(|_mode| {
                            view! {
                                <div class="mt-2 border-t border-border pt-2">
                                    <button
                                        class=move || {
                                            if let Some(m) = path_info() {
                                                if m == CurveMode::Curved {
                                                    classes::BTN_CAT_ACTIVE
                                                } else {
                                                    classes::BTN_CAT_INACTIVE
                                                }
                                            } else {
                                                classes::BTN_CAT_INACTIVE
                                            }
                                        }
                                        on:click=toggle_curve_mode
                                        title="Toggle curve mode"
                                    >
                                        {move || {
                                            if let Some(m) = path_info() {
                                                match m {
                                                    CurveMode::Straight => icon::line().into_view(),
                                                    CurveMode::Curved => icon::freehand().into_view(),
                                                }
                                            } else {
                                                icon::line().into_view()
                                            }
                                        }}
                                    </button>
                                </div>
                            }
                        })
                    }}
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
