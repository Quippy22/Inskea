/// Floating tool dock with drawing tools and utilities.
mod drawing;

pub use drawing::Tool;

use crate::canvas::CanvasMode;
use crate::model::{ElementId, Scene};
use crate::ui::classes;
use crate::ui::icon;
use leptos::*;

/// Side dock with drawing tools, eraser, and curve-mode toggle.
///
/// The dock sits on the left side of the screen and provides direct access to
/// all drawing tools. When exactly one `Line` or `Arrow` is selected, a
/// curve-mode toggle button appears at the bottom.
#[component]
pub fn Dock(
    selected_tool: RwSignal<Tool>,
    canvas_mode: RwSignal<CanvasMode>,
    eraser_active: RwSignal<bool>,
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
) -> impl IntoView {
    let select_tool = move |tool: Tool| {
        selected_tool.set(tool);
        canvas_mode.set(CanvasMode::Draw);
        eraser_active.set(false);
    };

    let toggle_eraser = move |_| {
        let new = !eraser_active.get();
        eraser_active.set(new);
        if new {
            canvas_mode.set(CanvasMode::Draw);
        }
    };

    let tool_class = move |tool: Tool| -> &'static str {
        if canvas_mode.get() == CanvasMode::Draw && selected_tool.get() == tool {
            classes::BTN_TBAR_ACTIVE
        } else {
            classes::BTN_TBAR_INACTIVE
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
            <div class={format!("{} p-0.5", classes::PANEL)}>
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

                <button
                    class=move || {
                        if eraser_active.get() {
                            classes::BTN_TBAR_ACTIVE
                        } else {
                            classes::BTN_TBAR_INACTIVE
                        }
                    }
                    on:click=toggle_eraser
                    title="Eraser"
                >
                    {icon::eraser()}
                </button>

            </div>
        </div>
    }
}
