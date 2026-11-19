/// Floating tool dock with drawing tools and utilities.
mod drawing;

pub use drawing::Tool;

use crate::canvas::CanvasMode;
use crate::ui::icon;
use crate::ui::panel_zone::{
    animate_snap_transition, detect_snap_zone, grip_line_horizontal, grip_line_vertical,
    tools_container_class, DraggedPanel, PanelZone,
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
    let start_pos = create_rw_signal((0.0f64, 0.0f64));
    let dock_ref = create_node_ref::<html::Div>();

    let on_grip_pointerdown = move |ev: web_sys::PointerEvent| {
        if ev.button() != 0 {
            return;
        }
        ev.prevent_default();
        ev.stop_propagation();
        is_dragging.set(true);
        start_pos.set((ev.client_x() as f64, ev.client_y() as f64));
        active_drag.set(Some((DraggedPanel::Tools, None)));
    };

    let _ = window_event_listener(leptos::ev::pointermove, move |ev: web_sys::PointerEvent| {
        if !is_dragging.get_untracked() {
            return;
        }
        let (sx, sy) = start_pos.get_untracked();
        let cx = ev.client_x() as f64;
        let cy = ev.client_y() as f64;
        let dx = cx - sx;
        let dy = cy - sy;

        if let Some(div) = dock_ref.get() {
            let raw: &web_sys::HtmlElement = &div;
            let style = raw.style();
            let _ = style.set_property("transform", &format!("translate3d({dx}px, {dy}px, 0)"));
            let _ = style.set_property("transition", "none");
            let _ = style.set_property("z-index", "60");
        }

        let win_size = crate::util::window_size();
        let snap = detect_snap_zone(cx, cy, win_size.x, win_size.y);
        active_drag.set(Some((DraggedPanel::Tools, snap)));
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
            active_drag.set(None);

            if let Some(div) = dock_ref.get() {
                let raw: &web_sys::HtmlElement = &div;
                let from_rect = raw.get_bounding_client_rect();
                let _ = raw.style().remove_property("z-index");

                if let Some(new_zone) = target_zone {
                    if new_zone != zone.get_untracked() {
                        zone.set(new_zone);
                        if let Some(ref cb) = on_drop_snap {
                            cb();
                        }
                    }
                }

                animate_snap_transition(raw, &from_rect);
            }
        });
    }

    view! {
        <div
            node_ref=dock_ref
            class=move || tools_container_class(zone.get(), shapes_zone.get())
        >
            <div class=move || {
                let layout = if zone.get().is_horizontal() {
                    "flex flex-row items-center gap-0.5"
                } else {
                    "flex flex-col items-center gap-0.5"
                };
                format!("{} p-0.5 {}", styles::PANEL, layout)
            }>
                <div
                    class="cursor-grab active:cursor-grabbing p-1.5 text-subtle/60 hover:text-fg select-none rounded flex items-center justify-center transition-colors group"
                    on:pointerdown=on_grip_pointerdown
                    title="Drag to move panel"
                >
                    {move || {
                        if zone.get().is_horizontal() {
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
