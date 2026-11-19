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

/// Computes the fixed positioning CSS classes for the Tools dock.
pub fn tools_container_class(zone: PanelZone, shapes_zone: PanelZone) -> &'static str {
    match zone {
        PanelZone::Left => {
            "fixed left-4 max-sm:left-2 top-1/2 -translate-y-1/2 z-40 flex flex-col items-center gap-0.5"
        }
        PanelZone::Right => {
            "fixed right-4 max-sm:right-2 top-1/2 -translate-y-1/2 z-40 flex flex-col items-center gap-0.5"
        }
        PanelZone::Top => {
            if shapes_zone == PanelZone::Top {
                "fixed top-16 left-[calc(50%-8.5rem)] -translate-x-1/2 z-40 flex flex-row items-center gap-0.5"
            } else {
                "fixed top-16 left-1/2 -translate-x-1/2 z-40 flex flex-row items-center gap-0.5"
            }
        }
        PanelZone::Bottom => {
            if shapes_zone == PanelZone::Bottom {
                "fixed bottom-4 left-[calc(50%-8.5rem)] -translate-x-1/2 z-40 flex flex-row items-center gap-0.5"
            } else {
                "fixed bottom-4 left-1/2 -translate-x-1/2 z-40 flex flex-row items-center gap-0.5"
            }
        }
    }
}

/// Computes the fixed positioning CSS classes for the Shapes / Styling panel.
pub fn shapes_container_class(zone: PanelZone, tools_zone: PanelZone) -> &'static str {
    match zone {
        PanelZone::Left => {
            if tools_zone == PanelZone::Left {
                "fixed left-[4.5rem] top-1/2 -translate-y-1/2 z-40 max-h-[85vh] overflow-y-auto"
            } else {
                "fixed left-4 top-1/2 -translate-y-1/2 z-40 max-h-[85vh] overflow-y-auto"
            }
        }
        PanelZone::Right => {
            if tools_zone == PanelZone::Right {
                "fixed right-[4.5rem] top-1/2 -translate-y-1/2 z-40 max-h-[85vh] overflow-y-auto"
            } else {
                "fixed right-4 top-1/2 -translate-y-1/2 z-40 max-h-[85vh] overflow-y-auto"
            }
        }
        PanelZone::Top => {
            if tools_zone == PanelZone::Top {
                "fixed top-16 left-[calc(50%+8.5rem)] -translate-x-1/2 z-40 max-h-[80vh] overflow-y-auto"
            } else {
                "fixed top-16 left-1/2 -translate-x-1/2 z-40 max-h-[80vh] overflow-y-auto"
            }
        }
        PanelZone::Bottom => {
            if tools_zone == PanelZone::Bottom {
                "fixed bottom-4 left-[calc(50%+8.5rem)] -translate-x-1/2 z-40 max-h-[80vh] overflow-y-auto"
            } else {
                "fixed bottom-4 left-1/2 -translate-x-1/2 z-40 max-h-[80vh] overflow-y-auto"
            }
        }
    }
}

/// Computes the ghost outline highlight classes for the preview shape.
pub fn ghost_highlight_class(
    panel: DraggedPanel,
    target_zone: PanelZone,
    other_zone: PanelZone,
) -> &'static str {
    match panel {
        DraggedPanel::Tools => match target_zone {
            PanelZone::Left => {
                "fixed left-4 top-1/2 -translate-y-1/2 z-30 w-11 h-[272px] rounded-lg border-2 border-dashed border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse flex flex-col items-center justify-center"
            }
            PanelZone::Right => {
                "fixed right-4 top-1/2 -translate-y-1/2 z-30 w-11 h-[272px] rounded-lg border-2 border-dashed border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse flex flex-col items-center justify-center"
            }
            PanelZone::Top => {
                if other_zone == PanelZone::Top {
                    "fixed top-16 left-[calc(50%-8.5rem)] -translate-x-1/2 z-30 w-[272px] h-11 rounded-lg border-2 border-dashed border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse flex items-center justify-center"
                } else {
                    "fixed top-16 left-1/2 -translate-x-1/2 z-30 w-[272px] h-11 rounded-lg border-2 border-dashed border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse flex items-center justify-center"
                }
            }
            PanelZone::Bottom => {
                if other_zone == PanelZone::Bottom {
                    "fixed bottom-4 left-[calc(50%-8.5rem)] -translate-x-1/2 z-30 w-[272px] h-11 rounded-lg border-2 border-dashed border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse flex items-center justify-center"
                } else {
                    "fixed bottom-4 left-1/2 -translate-x-1/2 z-30 w-[272px] h-11 rounded-lg border-2 border-dashed border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse flex items-center justify-center"
                }
            }
        },
        DraggedPanel::Shapes => match target_zone {
            PanelZone::Left => {
                if other_zone == PanelZone::Left {
                    "fixed left-[4.5rem] top-1/2 -translate-y-1/2 z-30 w-[240px] h-[340px] rounded-lg border-2 border-dashed border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse flex flex-col items-center justify-center"
                } else {
                    "fixed left-4 top-1/2 -translate-y-1/2 z-30 w-[240px] h-[340px] rounded-lg border-2 border-dashed border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse flex flex-col items-center justify-center"
                }
            }
            PanelZone::Right => {
                if other_zone == PanelZone::Right {
                    "fixed right-[4.5rem] top-1/2 -translate-y-1/2 z-30 w-[240px] h-[340px] rounded-lg border-2 border-dashed border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse flex flex-col items-center justify-center"
                } else {
                    "fixed right-4 top-1/2 -translate-y-1/2 z-30 w-[240px] h-[340px] rounded-lg border-2 border-dashed border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse flex flex-col items-center justify-center"
                }
            }
            PanelZone::Top => {
                if other_zone == PanelZone::Top {
                    "fixed top-16 left-[calc(50%+8.5rem)] -translate-x-1/2 z-30 w-[240px] h-[340px] rounded-lg border-2 border-dashed border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse flex flex-col items-center justify-center"
                } else {
                    "fixed top-16 left-1/2 -translate-x-1/2 z-30 w-[240px] h-[340px] rounded-lg border-2 border-dashed border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse flex flex-col items-center justify-center"
                }
            }
            PanelZone::Bottom => {
                if other_zone == PanelZone::Bottom {
                    "fixed bottom-4 left-[calc(50%+8.5rem)] -translate-x-1/2 z-30 w-[240px] h-[340px] rounded-lg border-2 border-dashed border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse flex flex-col items-center justify-center"
                } else {
                    "fixed bottom-4 left-1/2 -translate-x-1/2 z-30 w-[240px] h-[340px] rounded-lg border-2 border-dashed border-accent bg-accent/15 backdrop-blur-sm shadow-lg pointer-events-none transition-all duration-200 animate-pulse flex flex-col items-center justify-center"
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
) -> impl IntoView {
    view! {
        {move || {
            let (panel, target_zone) = active_drag.get()?;
            let target = target_zone?;
            let other_zone = match panel {
                DraggedPanel::Tools => shapes_zone.get(),
                DraggedPanel::Shapes => tools_zone.get(),
            };
            let class = ghost_highlight_class(panel, target, other_zone);
            let label = match panel {
                DraggedPanel::Tools => "Tools",
                DraggedPanel::Shapes => "Properties",
            };
            Some(view! {
                <div class=class>
                    <span class="text-[11px] font-medium tracking-wide uppercase text-accent/80 select-none px-2 py-0.5 rounded bg-accent/10">
                        {label}
                    </span>
                </div>
            })
        }}
    }
}

/// Animates a panel smoothly from its current visual bounding box `from_rect`
/// to its new layout position using GPU-accelerated CSS transform transition (FLIP).
pub fn animate_snap_transition(
    element: &web_sys::HtmlElement,
    from_rect: &web_sys::DomRect,
) {
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

        let _ = el
            .style()
            .set_property("transform", &format!("translate({dx}px, {dy}px)"));
        let _ = el.style().set_property("transition", "none");

        // Force reflow
        let _ = el.offset_height();

        let el_clone = el.clone();
        leptos::leptos_dom::helpers::request_animation_frame(move || {
            let _ = el_clone
                .style()
                .set_property("transition", "transform 320ms cubic-bezier(0.16, 1, 0.3, 1)");
            let _ = el_clone.style().set_property("transform", "translate(0px, 0px)");

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
        assert_eq!(detect_snap_zone(50.0, 400.0, 1000.0, 800.0), Some(PanelZone::Left));
    }

    #[test]
    fn test_detect_snap_zone_right() {
        assert_eq!(detect_snap_zone(950.0, 400.0, 1000.0, 800.0), Some(PanelZone::Right));
    }

    #[test]
    fn test_detect_snap_zone_top() {
        assert_eq!(detect_snap_zone(500.0, 50.0, 1000.0, 800.0), Some(PanelZone::Top));
    }

    #[test]
    fn test_detect_snap_zone_bottom() {
        assert_eq!(detect_snap_zone(500.0, 750.0, 1000.0, 800.0), Some(PanelZone::Bottom));
    }

    #[test]
    fn test_detect_snap_zone_center_returns_none() {
        assert_eq!(detect_snap_zone(500.0, 400.0, 1000.0, 800.0), None);
    }

    #[test]
    fn test_tools_container_class_orientation() {
        let left_cls = tools_container_class(PanelZone::Left, PanelZone::Left);
        assert!(left_cls.contains("flex-col"));

        let top_cls = tools_container_class(PanelZone::Top, PanelZone::Left);
        assert!(top_cls.contains("flex-row"));
    }

    #[test]
    fn test_shapes_container_class_coexistence() {
        let both_left = shapes_container_class(PanelZone::Left, PanelZone::Left);
        assert!(both_left.contains("left-[4.5rem]"));

        let shapes_only_left = shapes_container_class(PanelZone::Left, PanelZone::Right);
        assert!(shapes_only_left.contains("left-4"));
    }
}
