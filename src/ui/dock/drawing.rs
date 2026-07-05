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
pub fn DrawingPanel(selected_tool: RwSignal<Tool>) -> impl IntoView {
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
                on:click=move |_| selected_tool.set(Tool::Rectangle)
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
                on:click=move |_| selected_tool.set(Tool::Ellipse)
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
                on:click=move |_| selected_tool.set(Tool::Line)
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
                on:click=move |_| selected_tool.set(Tool::Arrow)
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
                on:click=move |_| selected_tool.set(Tool::Text)
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
                on:click=move |_| selected_tool.set(Tool::Freehand)
                title="Freehand"
            >
                {icon::freehand()}
            </button>
        </div>
    }
}
