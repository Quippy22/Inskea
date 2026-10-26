mod svgs;

use crate::canvas::settings::Theme;
use crate::canvas::{CanvasMode, DragOperation};
use crate::ui::dock::Tool;

fn to_cursor(svg: &str, x: u32, y: u32, fallback: &str, stroke: &str) -> String {
    let svg = svg.replace(['\n', '\r'], "").replace("white", stroke);
    format!(
        r#"url("data:image/svg+xml,{}") {} {}, {}"#,
        svg, x, y, fallback
    )
}

#[allow(clippy::too_many_arguments)]
pub fn cursor_style(
    mode: CanvasMode,
    tool: Tool,
    drag_action: Option<DragOperation>,
    eraser_active: bool,
    pan_dragging: bool,
    crop_active: bool,
    hovered_handle: Option<DragOperation>,
    theme: Theme,
) -> String {
    let stroke = theme.cursor();

    if crop_active {
        return to_cursor(svgs::CROSSHAIR, 16, 16, "crosshair", stroke);
    }

    if eraser_active {
        return to_cursor(svgs::ERASER_CURSOR, 4, 18, "default", stroke);
    }

    if let Some(handle) = drag_action {
        return match handle {
            DragOperation::Resize(rh) => rh.css_cursor().to_string(),
            DragOperation::Move => "move".to_string(),
            DragOperation::Rotate => "grabbing".to_string(),
            DragOperation::PathPoint(_) | DragOperation::PathMidpoint(_) => "pointer".to_string(),
        };
    }

    if let Some(handle) = hovered_handle {
        return match handle {
            DragOperation::Resize(rh) => rh.css_cursor().to_string(),
            DragOperation::Move => "move".to_string(),
            DragOperation::Rotate => "grab".to_string(),
            DragOperation::PathPoint(_) | DragOperation::PathMidpoint(_) => "pointer".to_string(),
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
        CanvasMode::Draw => draw_cursor(tool, stroke),
        CanvasMode::Erase => to_cursor(svgs::ERASER_CURSOR, 4, 18, "default", stroke),
    }
}

fn draw_cursor(tool: Tool, stroke: &str) -> String {
    match tool {
        Tool::Rectangle | Tool::Ellipse | Tool::Line | Tool::Arrow => {
            to_cursor(svgs::CROSSHAIR, 16, 16, "crosshair", stroke)
        }
        Tool::Text => to_cursor(svgs::TEXT_CURSOR, 16, 22, "text", stroke),
        Tool::Freehand => to_cursor(svgs::FREEHAND_CURSOR, 7, 24, "crosshair", stroke),
    }
}
