use crate::canvas::CanvasMode;
use crate::ui::classes;
use crate::ui::icon;
use leptos::*;

/// Available drawing tool types.
#[derive(Clone, Copy, PartialEq)]
pub enum Tool {
    Rectangle,
    Ellipse,
    Line,
    Arrow,
    Text,
    Freehand,
}

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

const ALL_TOOLS: [Tool; 6] = [
    Tool::Rectangle,
    Tool::Ellipse,
    Tool::Line,
    Tool::Arrow,
    Tool::Text,
    Tool::Freehand,
];

/// Vertical panel of drawing-tool buttons.
#[component]
pub fn DrawingPanel(
    selected_tool: RwSignal<Tool>,
    canvas_mode: RwSignal<CanvasMode>,
) -> impl IntoView {
    let select_tool = move |tool: Tool| {
        selected_tool.set(tool);
        canvas_mode.set(CanvasMode::Draw);
    };
    view! {
        <div class="flex flex-col p-1 gap-0.5">
            {ALL_TOOLS.iter().map(|&tool| {
                view! {
                    <button
                        class=move || {
                            if selected_tool.get() == tool {
                                classes::BTN_TOOL_ACTIVE
                            } else {
                                classes::BTN_TOOL_INACTIVE
                            }
                        }
                        on:click=move |_| select_tool(tool)
                        title=tool_title(tool)
                    >
                        {tool_icon(tool)}
                    </button>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}
