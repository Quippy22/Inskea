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
            <button
                class=move || {
                    if selected_tool.get() == Tool::Rectangle {
                        classes::BTN_TOOL_ACTIVE
                    } else {
                        classes::BTN_TOOL_INACTIVE
                    }
                }
                on:click=move |_| select_tool(Tool::Rectangle)
                title="Rectangle"
            >
                {icon::rect()}
            </button>
            <button
                class=move || {
                    if selected_tool.get() == Tool::Ellipse {
                        classes::BTN_TOOL_ACTIVE
                    } else {
                        classes::BTN_TOOL_INACTIVE
                    }
                }
                on:click=move |_| select_tool(Tool::Ellipse)
                title="Ellipse"
            >
                {icon::ellipse()}
            </button>
            <button
                class=move || {
                    if selected_tool.get() == Tool::Line {
                        classes::BTN_TOOL_ACTIVE
                    } else {
                        classes::BTN_TOOL_INACTIVE
                    }
                }
                on:click=move |_| select_tool(Tool::Line)
                title="Line"
            >
                {icon::line()}
            </button>
            <button
                class=move || {
                    if selected_tool.get() == Tool::Arrow {
                        classes::BTN_TOOL_ACTIVE
                    } else {
                        classes::BTN_TOOL_INACTIVE
                    }
                }
                on:click=move |_| select_tool(Tool::Arrow)
                title="Arrow"
            >
                {icon::arrow()}
            </button>
            <button
                class=move || {
                    if selected_tool.get() == Tool::Text {
                        classes::BTN_TOOL_ACTIVE
                    } else {
                        classes::BTN_TOOL_INACTIVE
                    }
                }
                on:click=move |_| select_tool(Tool::Text)
                title="Text"
            >
                {icon::text()}
            </button>
            <button
                class=move || {
                    if selected_tool.get() == Tool::Freehand {
                        classes::BTN_TOOL_ACTIVE
                    } else {
                        classes::BTN_TOOL_INACTIVE
                    }
                }
                on:click=move |_| select_tool(Tool::Freehand)
                title="Freehand"
            >
                {icon::freehand()}
            </button>
        </div>
    }
}
