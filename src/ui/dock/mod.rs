/// Floating tool dock with drawing tools and utilities.
mod drawing;

pub use drawing::Tool;

use crate::canvas::CanvasMode;
use crate::ui::icon;
use crate::ui::panel_zone::{
    align_dragged_panel, animate_snap_transition, clean_dragged_panel, detect_snap_zone,
    grip_line_horizontal, grip_line_vertical, tools_container_class, DraggedPanel, PanelZone,
};
use crate::ui::styles;
use leptos::*;
use std::rc::Rc;

/// Floating dock with drawing tools, eraser, and drag grip handle.
///
/// The dock can be positioned in any of the four designated zones (Left, Right,
/// Top, Bottom) and adapts its orientation (vertical for sides, horizontal for top/bottom).
#[component]
pub fn Dock(
    selected_tool: RwSignal<Tool>,
    canvas_mode: RwSignal<CanvasMode>,
    eraser_active: RwSignal<bool>,
    #[prop(default = create_rw_signal(PanelZone::Left))] zone: RwSignal<PanelZone>,
    #[prop(default = Signal::derive(|| PanelZone::Left))] shapes_zone: Signal<PanelZone>,
    #[prop(default = Signal::derive(|| PanelZone::Top))] toolbar_zone: Signal<PanelZone>,
    #[prop(default = create_rw_signal(None))] active_drag: RwSignal<
        Option<(DraggedPanel, Option<PanelZone>)>,
    >,
    #[prop(optional)] on_drop_snap: Option<Rc<dyn Fn()>>,
) -> impl IntoView {
    let select_tool = move |tool: Tool| {
        selected_tool.set(tool);
        canvas_mode.set(CanvasMode::Draw);
        eraser_active.set(false);
    };

    let eraser_on =
        Signal::derive(move || eraser_active.get() || canvas_mode.get() == CanvasMode::Erase);

    let toggle_eraser = move |_| {
        let on = eraser_on.get_untracked();
        eraser_active.set(!on);
        canvas_mode.set(CanvasMode::Draw);
    };

    let tool_class = move |tool: Tool| -> &'static str {
        if canvas_mode.get() == CanvasMode::Draw && selected_tool.get() == tool {
            styles::BTN_TBAR_ACTIVE
        } else {
            styles::BTN_TBAR_INACTIVE
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
            Tool::Chart => "Chart",
            Tool::Frame => "Frame",
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
            Tool::Chart => icon::chart().into_view(),
            Tool::Frame => icon::frame().into_view(),
        }
    }

    let is_dragging = create_rw_signal(false);
    let grab_ratio = create_rw_signal((0.5f64, 0.5f64));
    let dock_ref = create_node_ref::<html::Div>();
    let grip_ref = create_node_ref::<html::Div>();

    let effective_zone = Signal::derive(move || {
        if let Some((DraggedPanel::Tools, Some(target))) = active_drag.get() {
            target
        } else {
            zone.get()
        }
    });

    let on_grip_pointerdown = move |ev: web_sys::PointerEvent| {
        if ev.button() != 0 {
            return;
        }
        ev.prevent_default();
        ev.stop_propagation();

        let cx = ev.client_x() as f64;
        let cy = ev.client_y() as f64;

        if let (Some(panel_div), Some(grip_div)) = (dock_ref.get(), grip_ref.get()) {
            let grip_raw: &web_sys::HtmlElement = &grip_div;
            let panel_raw: &web_sys::HtmlElement = &panel_div;
            let grip_rect = grip_raw.get_bounding_client_rect();
            let rx = if grip_rect.width() > 0.0 {
                ((cx - grip_rect.left()) / grip_rect.width()).clamp(0.0, 1.0)
            } else {
                0.5
            };
            let ry = if grip_rect.height() > 0.0 {
                ((cy - grip_rect.top()) / grip_rect.height()).clamp(0.0, 1.0)
            } else {
                0.5
            };
            grab_ratio.set((rx, ry));
            align_dragged_panel(panel_raw, grip_raw, cx, cy, rx, ry);
        }

        is_dragging.set(true);
        active_drag.set(Some((DraggedPanel::Tools, None)));
    };

    let _ = window_event_listener(leptos::ev::pointermove, move |ev: web_sys::PointerEvent| {
        if !is_dragging.get_untracked() {
            return;
        }
        let cx = ev.client_x() as f64;
        let cy = ev.client_y() as f64;
        let (rx, ry) = grab_ratio.get_untracked();

        let win_size = crate::util::window_size();
        let snap = detect_snap_zone(cx, cy, win_size.x, win_size.y);
        let prev_snap = match active_drag.get_untracked() {
            Some((DraggedPanel::Tools, s)) => s,
            _ => None,
        };
        let snap_changed = snap != prev_snap;

        if let (Some(panel_div), Some(grip_div)) = (dock_ref.get(), grip_ref.get()) {
            let panel_raw: &web_sys::HtmlElement = &panel_div;
            let grip_raw: &web_sys::HtmlElement = &grip_div;
            align_dragged_panel(panel_raw, grip_raw, cx, cy, rx, ry);

            if snap_changed {
                active_drag.set(Some((DraggedPanel::Tools, snap)));
                let panel_clone = panel_raw.clone();
                let grip_clone = grip_raw.clone();
                leptos::leptos_dom::helpers::request_animation_frame(move || {
                    align_dragged_panel(&panel_clone, &grip_clone, cx, cy, rx, ry);
                });
            }
        } else if snap_changed {
            active_drag.set(Some((DraggedPanel::Tools, snap)));
        }
    });

    {
        let on_drop_snap = on_drop_snap.clone();
        let _ = window_event_listener(leptos::ev::pointerup, move |_ev: web_sys::PointerEvent| {
            if !is_dragging.get_untracked() {
                return;
            }
            is_dragging.set(false);

            let target_zone = match active_drag.get_untracked() {
                Some((DraggedPanel::Tools, Some(target))) => Some(target),
                _ => None,
            };

            if let Some(div) = dock_ref.get() {
                let raw: &web_sys::HtmlElement = &div;
                let from_rect = clean_dragged_panel(raw);

                if let Some(new_zone) = target_zone {
                    if new_zone != zone.get_untracked() {
                        zone.set(new_zone);
                        if let Some(ref cb) = on_drop_snap {
                            cb();
                        }
                    }
                }
                active_drag.set(None);

                animate_snap_transition(raw, &from_rect);
            } else {
                active_drag.set(None);
            }
        });
    }

    view! {
        <div
            node_ref=dock_ref
            class=move || tools_container_class(zone.get(), shapes_zone.get(), toolbar_zone.get())
        >
            <div class=move || {
                let layout = if effective_zone.get().is_horizontal() {
                    "flex flex-row items-center gap-0.5"
                } else {
                    "flex flex-col items-center gap-0.5"
                };
                let anim = if is_dragging.get() { "" } else { "transition-all duration-200" };
                format!("{} p-0.5 {} {}", styles::PANEL, layout, anim)
            }>
                <div
                    node_ref=grip_ref
                    class="cursor-grab active:cursor-grabbing p-1.5 text-subtle/60 hover:text-fg select-none rounded flex items-center justify-center transition-colors group"
                    on:pointerdown=on_grip_pointerdown
                    title="Drag to move panel"
                >
                    {move || {
                        if effective_zone.get().is_horizontal() {
                            grip_line_vertical().into_view()
                        } else {
                            grip_line_horizontal().into_view()
                        }
                    }}
                </div>
                {ALL_TOOLS
                    .iter()
                    .map(|&tool| {
                        view! {
                            <button
                                class=move || tool_class(tool)
                                on:click=move |_| select_tool(tool)
                                title=tool_title(tool)
                            >
                                {tool_icon(tool)}
                            </button>
                        }
                    })
                    .collect::<Vec<_>>()}
                <button
                    class=move || {
                        if eraser_on.get() {
                            styles::BTN_TBAR_ACTIVE
                        } else {
                            styles::BTN_TBAR_INACTIVE
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
