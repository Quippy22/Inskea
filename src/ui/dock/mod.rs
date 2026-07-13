/// Floating tool dock with categories and panels.
///
/// The dock sits on the left side of the screen and provides access to
/// drawing tools and page management via an expandable category panel.
mod drawing;
mod pages;

pub use drawing::Tool;
pub use pages::PagesPanel;

use crate::canvas::CanvasMode;
use crate::model::elements::path::CurveMode;
use crate::model::{ElementId, PathPoints, Scene};
use crate::ui::classes;
use crate::ui::icon;
use leptos::*;

/// Floating dock with tool buttons and category panels.
///
/// The dock sits on the left side of the screen and provides direct access to
/// drawing tools, with further category panels that slide out to the right.
///
/// When exactly one `Line` or `Arrow` is selected, a curve-mode toggle
/// button appears that switches between Straight and Curved rendering.
#[component]
pub fn Dock(
    selected_tool: RwSignal<Tool>,
    canvas_mode: RwSignal<CanvasMode>,
    eraser_active: RwSignal<bool>,
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
) -> impl IntoView {
    let show_panel = create_rw_signal(false);

    let toggle_pages = move |_| {
        eraser_active.set(false);
        show_panel.update(|v| *v = !*v);
    };

    let select_tool = move |tool: Tool| {
        selected_tool.set(tool);
        canvas_mode.set(CanvasMode::Draw);
        eraser_active.set(false);
    };

    let toggle_eraser = move |_| {
        let new = !eraser_active.get();
        eraser_active.set(new);
        if new {
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
        let els = scene.get().elements().to_vec();
        let el = els.iter().find(|e| e.id() == ids[0])?;
        el.path_points()?;
        Some(el.curve_mode())
    };

    let toggle_curve_mode = move |_| {
        let ids = selected_ids.get();
        if ids.len() != 1 {
            return;
        }
        let id = ids[0];
        scene.update(|s| {
            if let Some(el) = s.element_by_id_mut(id) {
                if el.path_points().is_some() {
                    let new_mode = match el.curve_mode() {
                        CurveMode::Straight => CurveMode::Curved,
                        CurveMode::Curved => CurveMode::Straight,
                    };
                    el.set_curve_mode(new_mode);
                }
            }
        });
    };

    let tool_class = move |tool: Tool| -> &'static str {
        if canvas_mode.get() == CanvasMode::Draw && selected_tool.get() == tool {
            classes::BTN_CAT_ACTIVE
        } else {
            classes::BTN_CAT_INACTIVE
        }
    };

    let pages_class = move || -> &'static str {
        if show_panel.get() {
            classes::BTN_CAT_ACTIVE
        } else {
            classes::BTN_CAT_INACTIVE
        }
    };

    const ALL_TOOLS: [Tool; 6] = [
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::Line,
        Tool::Arrow,
        Tool::Text,
        Tool::Freehand,
    ];

    fn tool_title(tool: Tool) -> &'static str {
        match tool {
            Tool::Rectangle => "Rectangle",
            Tool::Ellipse => "Ellipse",
            Tool::Line => "Line",
            Tool::Arrow => "Arrow",
            Tool::Text => "Text",
            Tool::Freehand => "Freehand",
        }
    }

    fn tool_icon(tool: Tool) -> leptos::View {
        match tool {
            Tool::Rectangle => icon::rect().into_view(),
            Tool::Ellipse => icon::ellipse().into_view(),
            Tool::Line => icon::line().into_view(),
            Tool::Arrow => icon::arrow().into_view(),
            Tool::Text => icon::text().into_view(),
            Tool::Freehand => icon::freehand().into_view(),
        }
    }

    view! {
        <div class=classes::CONTAINER_DOCK>
            <div class="relative flex">
                <div class=classes::PANEL>
                    // Tool buttons — single vertical column
                    {ALL_TOOLS.iter().map(|&tool| {
                        view! {
                            <button
                                class=move || tool_class(tool)
                                on:click=move |_| select_tool(tool)
                                title=tool_title(tool)
                            >
                                {tool_icon(tool)}
                            </button>
                        }
                    }).collect::<Vec<_>>()}

                    // Separator
                    <div class="w-9 h-px bg-border mx-auto my-1"></div>

                    // Pages
                    <button
                        class=move || pages_class()
                        on:click=toggle_pages
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
                    if show_panel.get() {
                        Some(
                            view! {
                                <div class="absolute left-[calc(100%+2px)] top-1/2 -translate-y-1/2 rounded-lg bg-panel/80 backdrop-blur-sm border border-border shadow-lg pointer-events-auto">
                                    {view! { <PagesPanel /> }}
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
