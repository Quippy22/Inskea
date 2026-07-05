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

/// Vertical panel of drawing-tool buttons (Rectangle, Ellipse, Line, etc.).
///
/// The currently selected tool is highlighted with the accent colour.
#[component]
pub fn DrawingPanel(selected_tool: RwSignal<Tool>) -> impl IntoView {
    view! {
        <div class="flex flex-col p-1 gap-0.5">
            <button
                class=move || {
                    let base = "flex items-center justify-center h-9 w-9 rounded-md transition-colors";
                    if selected_tool.get() == Tool::Rectangle {
                        format!("{base} text-accent bg-accent/10")
                    } else {
                        format!("{base} text-subtle hover:text-fg hover:bg-surface/50")
                    }
                }
                on:click=move |_| selected_tool.set(Tool::Rectangle)
                title="Rectangle"
            >
                {icon::rect()}
            </button>
            <button
                class=move || {
                    let base = "flex items-center justify-center h-9 w-9 rounded-md transition-colors";
                    if selected_tool.get() == Tool::Ellipse {
                        format!("{base} text-accent bg-accent/10")
                    } else {
                        format!("{base} text-subtle hover:text-fg hover:bg-surface/50")
                    }
                }
                on:click=move |_| selected_tool.set(Tool::Ellipse)
                title="Ellipse"
            >
                {icon::ellipse()}
            </button>
            <button
                class=move || {
                    let base = "flex items-center justify-center h-9 w-9 rounded-md transition-colors";
                    if selected_tool.get() == Tool::Line {
                        format!("{base} text-accent bg-accent/10")
                    } else {
                        format!("{base} text-subtle hover:text-fg hover:bg-surface/50")
                    }
                }
                on:click=move |_| selected_tool.set(Tool::Line)
                title="Line"
            >
                {icon::line()}
            </button>
            <button
                class=move || {
                    let base = "flex items-center justify-center h-9 w-9 rounded-md transition-colors";
                    if selected_tool.get() == Tool::Arrow {
                        format!("{base} text-accent bg-accent/10")
                    } else {
                        format!("{base} text-subtle hover:text-fg hover:bg-surface/50")
                    }
                }
                on:click=move |_| selected_tool.set(Tool::Arrow)
                title="Arrow"
            >
                {icon::arrow()}
            </button>
            <button
                class=move || {
                    let base = "flex items-center justify-center h-9 w-9 rounded-md transition-colors";
                    if selected_tool.get() == Tool::Text {
                        format!("{base} text-accent bg-accent/10")
                    } else {
                        format!("{base} text-subtle hover:text-fg hover:bg-surface/50")
                    }
                }
                on:click=move |_| selected_tool.set(Tool::Text)
                title="Text"
            >
                {icon::text()}
            </button>
            <button
                class=move || {
                    let base = "flex items-center justify-center h-9 w-9 rounded-md transition-colors";
                    if selected_tool.get() == Tool::Freehand {
                        format!("{base} text-accent bg-accent/10")
                    } else {
                        format!("{base} text-subtle hover:text-fg hover:bg-surface/50")
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
