mod svgs;

use crate::canvas::{CanvasMode, Handle};
use crate::ui::dock::Tool;

fn to_cursor(svg: &str, x: u32, y: u32, fallback: &str) -> String {
    let svg = svg.replace(['\n', '\r'], "");
    format!(
        r#"url("data:image/svg+xml,{}") {} {}, {}"#,
        svg, x, y, fallback
    )
}

pub fn cursor_style(
    mode: CanvasMode,
    tool: Tool,
    drag_action: Option<Handle>,
    eraser_active: bool,
    pan_dragging: bool,
    crop_active: bool,
) -> String {
    if crop_active {
        return to_cursor(svgs::CROSSHAIR, 16, 16, "crosshair");
    }

    if eraser_active {
        return to_cursor(svgs::ERASER_CURSOR, 4, 18, "default");
    }

    if let Some(handle) = drag_action {
        return match handle {
            Handle::Resize(rh) => rh.css_cursor().to_string(),
            Handle::Move => "move".to_string(),
            Handle::Rotate => "grab".to_string(),
            Handle::PathPoint(_) | Handle::PathMidpoint(_) => "pointer".to_string(),
        };
    }

    match mode {
        CanvasMode::Select => "default".to_string(),
        CanvasMode::Pan => {
            if pan_dragging {
                "grabbing".to_string()
            } else {
                "grab".to_string()
            }
        }
        CanvasMode::Draw => draw_cursor(tool),
        CanvasMode::Erase => to_cursor(svgs::ERASER_CURSOR, 4, 18, "default"),
    }
}

fn draw_cursor(tool: Tool) -> String {
    match tool {
        Tool::Rectangle | Tool::Ellipse | Tool::Line | Tool::Arrow => {
            to_cursor(svgs::CROSSHAIR, 16, 16, "crosshair")
        }
        Tool::Text => to_cursor(svgs::TEXT_CURSOR, 16, 22, "text"),
        Tool::Freehand => to_cursor(svgs::FREEHAND_CURSOR, 7, 24, "crosshair"),
    }
}
