use leptos::*;
use serde::{Deserialize, Serialize};

/// The four designated snap zones around the canvas edges.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanelZone {
    Left,
    Right,
    Top,
    Bottom,
}

impl PanelZone {
    #[allow(dead_code)]
    pub const ALL: [PanelZone; 4] = [
        PanelZone::Left,
        PanelZone::Right,
        PanelZone::Top,
        PanelZone::Bottom,
    ];

    #[allow(dead_code)]
    pub fn is_vertical(self) -> bool {
        matches!(self, PanelZone::Left | PanelZone::Right)
    }

    pub fn is_horizontal(self) -> bool {
        matches!(self, PanelZone::Top | PanelZone::Bottom)
    }
}

/// Identifies which floating panel is being dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraggedPanel {
    Tools,
    Shapes,
    Toolbar,
}

/// Edge distance threshold in pixels for activating a snap zone.
pub const SNAP_EDGE_THRESHOLD: f64 = 160.0;

/// Detects if cursor position `(x, y)` is close enough to any of the 4 edges
/// to trigger snapping.
pub fn detect_snap_zone(x: f64, y: f64, width: f64, height: f64) -> Option<PanelZone> {
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    // Adaptive threshold for small viewports so zones don't overlap.
    let threshold = SNAP_EDGE_THRESHOLD.min(width * 0.28).min(height * 0.28);

    let dist_left = x;
    let dist_right = width - x;
    let dist_top = y;
    let dist_bottom = height - y;

    let min_dist = dist_left.min(dist_right).min(dist_top).min(dist_bottom);

    if min_dist <= threshold {
        if min_dist == dist_left {
            Some(PanelZone::Left)
        } else if min_dist == dist_right {
            Some(PanelZone::Right)
        } else if min_dist == dist_top {
            Some(PanelZone::Top)
        } else {
            Some(PanelZone::Bottom)
        }
    } else {
        None
    }
}

/// Computes the fixed positioning CSS classes for the main Toolbar.
pub fn toolbar_container_class(zone: PanelZone) -> &'static str {
    match zone {
        PanelZone::Top => "fixed top-4 left-1/2 -translate-x-1/2 z-50 pointer-events-auto",
        PanelZone::Bottom => "fixed bottom-4 left-1/2 -translate-x-1/2 z-50 pointer-events-auto",
        PanelZone::Left => "fixed left-4 top-1/2 -translate-y-1/2 z-50 pointer-events-auto",
        PanelZone::Right => "fixed right-4 top-1/2 -translate-y-1/2 z-50 pointer-events-auto",
    }
}

/// Computes the fixed positioning CSS classes for the Tools dock.
pub fn tools_container_class(
    zone: PanelZone,
    _shapes_zone: PanelZone,
    toolbar_zone: PanelZone,
) -> &'static str {
    match zone {
        PanelZone::Left => {
            if toolbar_zone == PanelZone::Left {
                "fixed left-[4.25rem] top-1/2 -translate-y-1/2 z-40 flex flex-col items-center gap-0.5"
            } else {
                "fixed left-4 max-sm:left-2 top-1/2 -translate-y-1/2 z-40 flex flex-col items-center gap-0.5"
            }
        }
        PanelZone::Right => {
            if toolbar_zone == PanelZone::Right {
                "fixed right-[4.25rem] top-1/2 -translate-y-1/2 z-40 flex flex-col items-center gap-0.5"
            } else {
                "fixed right-4 max-sm:right-2 top-1/2 -translate-y-1/2 z-40 flex flex-col items-center gap-0.5"
            }
        }
        PanelZone::Top => {
            if toolbar_zone == PanelZone::Top {
                "fixed top-16 left-1/2 -translate-x-1/2 z-40 flex flex-row items-center gap-0.5"
            } else {
                "fixed top-4 left-1/2 -translate-x-1/2 z-40 flex flex-row items-center gap-0.5"
            }
        }
        PanelZone::Bottom => {
            if toolbar_zone == PanelZone::Bottom {
                "fixed bottom-16 left-1/2 -translate-x-1/2 z-40 flex flex-row items-center gap-0.5"
            } else {
                "fixed bottom-4 left-1/2 -translate-x-1/2 z-40 flex flex-row items-center gap-0.5"
            }
        }
    }
}

/// Computes the fixed positioning CSS classes for the Shapes / Styling panel.
pub fn shapes_container_class(
    zone: PanelZone,
    tools_zone: PanelZone,
    toolbar_zone: PanelZone,
) -> &'static str {
    match zone {
        PanelZone::Left => {
            if toolbar_zone == PanelZone::Left && tools_zone == PanelZone::Left {
                "fixed left-[7.5rem] top-1/2 -translate-y-1/2 z-40 max-h-[85vh] overflow-y-auto"
            } else if toolbar_zone == PanelZone::Left || tools_zone == PanelZone::Left {
                "fixed left-[4.5rem] top-1/2 -translate-y-1/2 z-40 max-h-[85vh] overflow-y-auto"
            } else {
                "fixed left-4 top-1/2 -translate-y-1/2 z-40 max-h-[85vh] overflow-y-auto"
            }
        }
        PanelZone::Right => {
            if toolbar_zone == PanelZone::Right && tools_zone == PanelZone::Right {
                "fixed right-[7.5rem] top-1/2 -translate-y-1/2 z-40 max-h-[85vh] overflow-y-auto"
            } else if toolbar_zone == PanelZone::Right || tools_zone == PanelZone::Right {
                "fixed right-[4.5rem] top-1/2 -translate-y-1/2 z-40 max-h-[85vh] overflow-y-auto"
            } else {
                "fixed right-4 top-1/2 -translate-y-1/2 z-40 max-h-[85vh] overflow-y-auto"
            }
        }
        PanelZone::Top => {
            let is_tb_top = toolbar_zone == PanelZone::Top;
            let is_tools_top = tools_zone == PanelZone::Top;
            match (is_tb_top, is_tools_top) {
                (true, true) => {
                    "fixed top-28 left-1/2 -translate-x-1/2 z-40 max-w-[95vw] overflow-x-auto"
                }
                (true, false) | (false, true) => {
                    "fixed top-16 left-1/2 -translate-x-1/2 z-40 max-w-[95vw] overflow-x-auto"
                }
                (false, false) => {
                    "fixed top-4 left-1/2 -translate-x-1/2 z-40 max-w-[95vw] overflow-x-auto"
                }
            }
        }
        PanelZone::Bottom => {
            let is_tb_bot = toolbar_zone == PanelZone::Bottom;
            let is_tools_bot = tools_zone == PanelZone::Bottom;
            match (is_tb_bot, is_tools_bot) {
                (true, true) => {
                    "fixed bottom-28 left-1/2 -translate-x-1/2 z-40 max-w-[95vw] overflow-x-auto"
                }
                (true, false) | (false, true) => {
                    "fixed bottom-16 left-1/2 -translate-x-1/2 z-40 max-w-[95vw] overflow-x-auto"
                }
                (false, false) => {
                    "fixed bottom-4 left-1/2 -translate-x-1/2 z-40 max-w-[95vw] overflow-x-auto"
                }
            }
        }
    }
}

/// Computes the ghost outline highlight classes for the preview shape.
pub fn ghost_highlight_class(
    panel: DraggedPanel,
    target_zone: PanelZone,
    tools_zone: PanelZone,
    _shapes_zone: PanelZone,
    toolbar_zone: PanelZone,
) -> &'static str {
    match panel {
        DraggedPanel::Toolbar => match target_zone {
            PanelZone::Top => {
                "fixed top-4 left-1/2 -translate-x-1/2 z-30 w-[340px] h-10 rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
            }
            PanelZone::Bottom => {
                "fixed bottom-4 left-1/2 -translate-x-1/2 z-30 w-[340px] h-10 rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
            }
            PanelZone::Left => {
                "fixed left-4 top-1/2 -translate-y-1/2 z-30 w-11 h-[320px] rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
            }
            PanelZone::Right => {
                "fixed right-4 top-1/2 -translate-y-1/2 z-30 w-11 h-[320px] rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
            }
        },
        DraggedPanel::Tools => match target_zone {
            PanelZone::Left => {
                if toolbar_zone == PanelZone::Left {
                    "fixed left-[4.25rem] top-1/2 -translate-y-1/2 z-30 w-11 h-[272px] rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                } else {
                    "fixed left-4 top-1/2 -translate-y-1/2 z-30 w-11 h-[272px] rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                }
            }
            PanelZone::Right => {
                if toolbar_zone == PanelZone::Right {
                    "fixed right-[4.25rem] top-1/2 -translate-y-1/2 z-30 w-11 h-[272px] rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                } else {
                    "fixed right-4 top-1/2 -translate-y-1/2 z-30 w-11 h-[272px] rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                }
            }
            PanelZone::Top => {
                if toolbar_zone == PanelZone::Top {
                    "fixed top-16 left-1/2 -translate-x-1/2 z-30 w-[272px] h-11 rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                } else {
                    "fixed top-4 left-1/2 -translate-x-1/2 z-30 w-[272px] h-11 rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                }
            }
            PanelZone::Bottom => {
                if toolbar_zone == PanelZone::Bottom {
                    "fixed bottom-16 left-1/2 -translate-x-1/2 z-30 w-[272px] h-11 rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                } else {
                    "fixed bottom-4 left-1/2 -translate-x-1/2 z-30 w-[272px] h-11 rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                }
            }
        },
        DraggedPanel::Shapes => match target_zone {
            PanelZone::Left => {
                let is_tb = toolbar_zone == PanelZone::Left;
                let is_tools = tools_zone == PanelZone::Left;
                if is_tb && is_tools {
                    "fixed left-[7.5rem] top-1/2 -translate-y-1/2 z-30 w-[240px] h-[340px] rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                } else if is_tb || is_tools {
                    "fixed left-[4.5rem] top-1/2 -translate-y-1/2 z-30 w-[240px] h-[340px] rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                } else {
                    "fixed left-4 top-1/2 -translate-y-1/2 z-30 w-[240px] h-[340px] rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                }
            }
            PanelZone::Right => {
                let is_tb = toolbar_zone == PanelZone::Right;
                let is_tools = tools_zone == PanelZone::Right;
                if is_tb && is_tools {
                    "fixed right-[7.5rem] top-1/2 -translate-y-1/2 z-30 w-[240px] h-[340px] rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                } else if is_tb || is_tools {
                    "fixed right-[4.5rem] top-1/2 -translate-y-1/2 z-30 w-[240px] h-[340px] rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                } else {
                    "fixed right-4 top-1/2 -translate-y-1/2 z-30 w-[240px] h-[340px] rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                }
            }
            PanelZone::Top => {
                let is_tb_top = toolbar_zone == PanelZone::Top;
                let is_tools_top = tools_zone == PanelZone::Top;
                match (is_tb_top, is_tools_top) {
                    (true, true) => {
                        "fixed top-28 left-1/2 -translate-x-1/2 z-30 w-[560px] max-w-[90vw] h-12 rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                    }
                    (true, false) | (false, true) => {
                        "fixed top-16 left-1/2 -translate-x-1/2 z-30 w-[560px] max-w-[90vw] h-12 rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                    }
                    (false, false) => {
                        "fixed top-4 left-1/2 -translate-x-1/2 z-30 w-[560px] max-w-[90vw] h-12 rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                    }
                }
            }
            PanelZone::Bottom => {
                let is_tb_bot = toolbar_zone == PanelZone::Bottom;
                let is_tools_bot = tools_zone == PanelZone::Bottom;
                match (is_tb_bot, is_tools_bot) {
                    (true, true) => {
                        "fixed bottom-28 left-1/2 -translate-x-1/2 z-30 w-[560px] max-w-[90vw] h-12 rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                    }
                    (true, false) | (false, true) => {
                        "fixed bottom-16 left-1/2 -translate-x-1/2 z-30 w-[560px] max-w-[90vw] h-12 rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                    }
                    (false, false) => {
                        "fixed bottom-4 left-1/2 -translate-x-1/2 z-30 w-[560px] max-w-[90vw] h-12 rounded-lg border-2 border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse"
                    }
                }
            }
        },
    }
}

/// Highlight component rendered when a panel is being dragged close to a snap zone.
#[component]
pub fn PanelSnapHighlight(
    active_drag: Signal<Option<(DraggedPanel, Option<PanelZone>)>>,
    tools_zone: Signal<PanelZone>,
    shapes_zone: Signal<PanelZone>,
    #[prop(default = Signal::derive(|| PanelZone::Top))] toolbar_zone: Signal<PanelZone>,
) -> impl IntoView {
    view! {
        {move || {
            let (panel, target_zone) = active_drag.get()?;
            let target = target_zone?;
            let class = ghost_highlight_class(panel, target, tools_zone.get(), shapes_zone.get(), toolbar_zone.get());
            Some(view! {
                <div class=class></div>
            })
        }}
    }
}

/// Calculates the top-left viewport coordinate for a panel so that the point inside
/// its grip handle where the user grabbed remains fixed at (cursor_x, cursor_y).
#[allow(clippy::too_many_arguments)]
pub fn calculate_drag_target(
    cursor_x: f64,
    cursor_y: f64,
    grip_x: f64,
    grip_y: f64,
    grip_w: f64,
    grip_h: f64,
    panel_x: f64,
    panel_y: f64,
    grab_ratio_x: f64,
    grab_ratio_y: f64,
) -> (f64, f64) {
    let grip_rel_panel_x = grip_x - panel_x;
    let grip_rel_panel_y = grip_y - panel_y;

    let grab_in_grip_x = grab_ratio_x * grip_w;
    let grab_in_grip_y = grab_ratio_y * grip_h;

    let grab_in_panel_x = grip_rel_panel_x + grab_in_grip_x;
    let grab_in_panel_y = grip_rel_panel_y + grab_in_grip_y;

    (cursor_x - grab_in_panel_x, cursor_y - grab_in_panel_y)
}

/// Positions a dragged floating panel so that the point on the grip handle where the user
/// clicked remains locked directly underneath the mouse cursor, even when the panel rotates
/// between horizontal and vertical orientations.
pub fn align_dragged_panel(
    panel_el: &web_sys::HtmlElement,
    grip_el: &web_sys::HtmlElement,
    cursor_x: f64,
    cursor_y: f64,
    grab_ratio_x: f64,
    grab_ratio_y: f64,
) {
    let grip_rect = grip_el.get_bounding_client_rect();
    let panel_rect = panel_el.get_bounding_client_rect();

    let (target_left, target_top) = calculate_drag_target(
        cursor_x,
        cursor_y,
        grip_rect.left(),
        grip_rect.top(),
        grip_rect.width(),
        grip_rect.height(),
        panel_rect.left(),
        panel_rect.top(),
        grab_ratio_x,
        grab_ratio_y,
    );

    let style = panel_el.style();
    let _ = style.set_property("left", "0px");
    let _ = style.set_property("top", "0px");
    let _ = style.set_property("right", "auto");
    let _ = style.set_property("bottom", "auto");
    let _ = style.set_property(
        "transform",
        &format!("translate3d({target_left}px, {target_top}px, 0)"),
    );
    let _ = style.set_property("transition", "none");
    let _ = style.set_property("z-index", "60");
}

/// Cleans up inline drag styles from a panel and returns its current bounding client rect for FLIP animation.
pub fn clean_dragged_panel(panel_el: &web_sys::HtmlElement) -> web_sys::DomRect {
    let from_rect = panel_el.get_bounding_client_rect();
    let style = panel_el.style();
    let _ = style.remove_property("left");
    let _ = style.remove_property("top");
    let _ = style.remove_property("right");
    let _ = style.remove_property("bottom");
    let _ = style.remove_property("transform");
    let _ = style.remove_property("transition");
    let _ = style.remove_property("z-index");
    from_rect
}

/// Animates a panel smoothly from its current visual bounding box `from_rect`
/// to its new layout position using GPU-accelerated CSS transform transition (FLIP).
pub fn animate_snap_transition(element: &web_sys::HtmlElement, from_rect: &web_sys::DomRect) {
    let el = element.clone();
    let from_left = from_rect.left();
    let from_top = from_rect.top();

    leptos::leptos_dom::helpers::request_animation_frame(move || {
        let to_rect = el.get_bounding_client_rect();
        let dx = from_left - to_rect.left();
        let dy = from_top - to_rect.top();

        if dx.abs() < 1.0 && dy.abs() < 1.0 {
            let _ = el.style().remove_property("transform");
            let _ = el.style().remove_property("transition");
            return;
        }

        let class_name = el.class_name();
        let is_h = class_name.contains("-translate-x-1/2");
        let is_v = class_name.contains("-translate-y-1/2");

        let (start_transform, target_transform) = if is_h {
            (
                format!("translate3d(calc(-50% + {dx}px), {dy}px, 0)"),
                "translate3d(-50%, 0px, 0)".to_string(),
            )
        } else if is_v {
            (
                format!("translate3d({dx}px, calc(-50% + {dy}px), 0)"),
                "translate3d(0px, -50%, 0)".to_string(),
            )
        } else {
            (
                format!("translate3d({dx}px, {dy}px, 0)"),
                "translate3d(0px, 0px, 0)".to_string(),
            )
        };

        let _ = el.style().set_property("transform", &start_transform);
        let _ = el.style().set_property("transition", "none");

        // Force reflow
        let _ = el.offset_height();

        let el_clone = el.clone();
        leptos::leptos_dom::helpers::request_animation_frame(move || {
            let _ = el_clone.style().set_property(
                "transition",
                "transform 320ms cubic-bezier(0.16, 1, 0.3, 1)",
            );
            let _ = el_clone
                .style()
                .set_property("transform", &target_transform);

            #[cfg(target_arch = "wasm32")]
            leptos::set_timeout(
                move || {
                    let _ = el_clone.style().remove_property("transform");
                    let _ = el_clone.style().remove_property("transition");
                },
                std::time::Duration::from_millis(350),
            );
        });
    });
}

/// Horizontal pill drag handle line.
pub fn grip_line_horizontal() -> impl IntoView {
    view! {
        <div class="w-6 h-1 rounded-full bg-subtle/40 group-hover:bg-accent/80 transition-colors pointer-events-none" />
    }
}

/// Vertical pill drag handle line.
pub fn grip_line_vertical() -> impl IntoView {
    view! {
        <div class="w-1 h-6 rounded-full bg-subtle/40 group-hover:bg-accent/80 transition-colors pointer-events-none" />
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_snap_zone_left() {
        assert_eq!(
            detect_snap_zone(50.0, 400.0, 1000.0, 800.0),
            Some(PanelZone::Left)
        );
    }

    #[test]
    fn test_detect_snap_zone_right() {
        assert_eq!(
            detect_snap_zone(950.0, 400.0, 1000.0, 800.0),
            Some(PanelZone::Right)
        );
    }

    #[test]
    fn test_detect_snap_zone_top() {
        assert_eq!(
            detect_snap_zone(500.0, 50.0, 1000.0, 800.0),
            Some(PanelZone::Top)
        );
    }

    #[test]
    fn test_detect_snap_zone_bottom() {
        assert_eq!(
            detect_snap_zone(500.0, 750.0, 1000.0, 800.0),
            Some(PanelZone::Bottom)
        );
    }

    #[test]
    fn test_detect_snap_zone_center_returns_none() {
        assert_eq!(detect_snap_zone(500.0, 400.0, 1000.0, 800.0), None);
    }

    #[test]
    fn test_tools_container_class_orientation() {
        let left_cls = tools_container_class(PanelZone::Left, PanelZone::Left, PanelZone::Top);
        assert!(left_cls.contains("flex-col"));

        let top_cls = tools_container_class(PanelZone::Top, PanelZone::Left, PanelZone::Top);
        assert!(top_cls.contains("flex-row"));
    }

    #[test]
    fn test_shapes_container_class_coexistence() {
        let both_left = shapes_container_class(PanelZone::Left, PanelZone::Left, PanelZone::Top);
        assert!(both_left.contains("left-[4.5rem]"));

        let shapes_only_left =
            shapes_container_class(PanelZone::Left, PanelZone::Right, PanelZone::Top);
        assert!(shapes_only_left.contains("left-4"));

        let all_three_left =
            shapes_container_class(PanelZone::Left, PanelZone::Left, PanelZone::Left);
        assert!(all_three_left.contains("left-[7.5rem]"));
    }

    #[test]
    fn test_toolbar_container_class() {
        assert!(toolbar_container_class(PanelZone::Top).contains("top-4"));
        assert!(toolbar_container_class(PanelZone::Bottom).contains("bottom-4"));
        assert!(toolbar_container_class(PanelZone::Left).contains("left-4"));
        assert!(toolbar_container_class(PanelZone::Right).contains("right-4"));
    }

    #[test]
    fn test_horizontal_shapes_container_and_ghost() {
        let shapes_top = shapes_container_class(PanelZone::Top, PanelZone::Left, PanelZone::Top);
        assert!(shapes_top.contains("top-16"));
        assert!(shapes_top.contains("max-w-[95vw]"));

        let shapes_top_all = shapes_container_class(PanelZone::Top, PanelZone::Top, PanelZone::Top);
        assert!(shapes_top_all.contains("top-28"));

        let ghost_shapes_top = ghost_highlight_class(
            DraggedPanel::Shapes,
            PanelZone::Top,
            PanelZone::Left,
            PanelZone::Left,
            PanelZone::Top,
        );
        assert!(ghost_shapes_top.contains("w-[560px]"));
        assert!(ghost_shapes_top.contains("top-16"));
    }

    #[test]
    fn test_calculate_drag_target_preserves_cursor_point() {
        // Test vertical panel: user grabbed center of grip handle at (100, 200)
        let (left, top) = calculate_drag_target(
            100.0, 200.0, // cursor
            90.0, 185.0, // grip pos
            20.0, 30.0, // grip size
            80.0, 180.0, // panel pos
            0.5, 0.5, // grabbed center of grip
        );
        assert_eq!(left, 80.0);
        assert_eq!(top, 180.0);

        // When panel rotates to horizontal: grip dimensions & offset in panel change
        // Suppose new horizontal panel has grip at relative (10.0, 12.0) with size (30.0, 20.0)
        let (rot_left, rot_top) = calculate_drag_target(
            100.0, 200.0, // same cursor position
            10.0, 12.0, // grip at panel-relative (10, 12)
            30.0, 20.0, // horizontal grip size
            0.0, 0.0, // panel origin
            0.5, 0.5, // grabbed center of grip
        );
        // Grabbed point inside horizontal panel = 10 + 0.5*30 = 25, 12 + 0.5*20 = 22
        // Panel top-left should be cursor (100, 200) - (25, 22) = (75, 178)
        assert_eq!(rot_left, 75.0);
        assert_eq!(rot_top, 178.0);
        // Verifying: panel top-left + grabbed point in panel = cursor!
        assert_eq!(rot_left + 25.0, 100.0);
        assert_eq!(rot_top + 22.0, 200.0);
    }
}
