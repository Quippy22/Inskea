mod viewport;
pub use viewport::Viewport;

use crate::ui::dock::Tool;
use leptos::ev;
use leptos::svg::Svg;
use leptos::*;

const PREVIEW_SIZE: f64 = 10.0;
const PREVIEW_OFFSET: f64 = 6.0;

#[component]
pub fn Canvas(
    cursor_screen: RwSignal<(f64, f64)>,
    cursor_world: RwSignal<(f64, f64)>,
    viewport: RwSignal<Viewport>,
    selected_tool: RwSignal<Tool>,
) -> impl IntoView {
    fn window_size() -> (f64, f64) {
        let window = web_sys::window().expect("no global `window` exists");
        let w = window
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let h = window
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        (w, h)
    }

    let screen_size = create_rw_signal(window_size());
    let svg_ref = create_node_ref::<Svg>();

    let _ = window_event_listener(ev::resize, move |_| {
        screen_size.set(window_size());
    });

    let on_pointer_move = move |ev: ev::PointerEvent| {
        let screen = (ev.offset_x() as f64, ev.offset_y() as f64);
        cursor_screen.set(screen);
        let world = viewport.get().screen_to_world(screen, screen_size.get());
        cursor_world.set(world);
    };

    let on_wheel = move |ev: ev::WheelEvent| {
        ev.prevent_default();
        let screen = cursor_screen.get();
        let (sw, sh) = screen_size.get();
        let factor = if ev.delta_y() < 0.0 { 1.1 } else { 1.0 / 1.1 };

        viewport.update(|vp| {
            let world = vp.screen_to_world(screen, (sw, sh));
            vp.zoom = (vp.zoom * factor).clamp(0.1, 20.0);
            vp.offset_x = world.0 - (screen.0 - sw / 2.0) / vp.zoom;
            vp.offset_y = world.1 - (screen.1 - sh / 2.0) / vp.zoom;
        });
    };

    let view_box = move || {
        let (w, h) = screen_size.get();
        viewport.get().to_view_box(w, h)
    };

    let preview = move || {
        let tool = selected_tool.get();
        let (cx, cy) = cursor_world.get();
        let px = cx + PREVIEW_OFFSET;
        let py = cy - PREVIEW_OFFSET;
        let s = PREVIEW_SIZE;

        match tool {
            Tool::Rectangle => view! {
                <rect
                    x=px
                    y=py
                    width=s
                    height=s
                    fill="none"
                    stroke="#7aa2f7"
                    stroke-width="1.5"
                    opacity="0.6"
                    pointer-events="none"
                />
            }
            .into_view(),
            Tool::Ellipse => view! {
                <ellipse
                    cx=(px + s / 2.0)
                    cy=(py + s / 2.0)
                    rx=(s / 2.0)
                    ry=(s / 2.0)
                    fill="none"
                    stroke="#7aa2f7"
                    stroke-width="1.5"
                    opacity="0.6"
                    pointer-events="none"
                />
            }
            .into_view(),
            Tool::Line => view! {
                <line
                    x1=px
                    y1=(py + s)
                    x2=(px + s)
                    y2=py
                    stroke="#7aa2f7"
                    stroke-width="1.5"
                    opacity="0.6"
                    pointer-events="none"
                />
            }
            .into_view(),
            Tool::Arrow => view! {
                <g
                    opacity="0.6"
                    pointer-events="none"
                    stroke="#7aa2f7"
                    stroke-width="1.5"
                    fill="none"
                >
                    <line x1=px y1=(py + s) x2=(px + s) y2=py />
                    <polyline points=format!(
                        "{},{} {},{} {},{}",
                        px + s,
                        py,
                        px + s - 3.0,
                        py,
                        px + s,
                        py + 3.0,
                    ) />
                </g>
            }
            .into_view(),
            Tool::Text => view! {
                <text
                    x=(px + s / 2.0)
                    y=(py + s / 2.0)
                    fill="#7aa2f7"
                    opacity="0.6"
                    pointer-events="none"
                    font-size="8"
                    font-family="sans-serif"
                    dominant-baseline="central"
                    text-anchor="middle"
                >
                    "Aa"
                </text>
            }
            .into_view(),
            Tool::Freehand => view! {
                <path
                    d=format!(
                        "M{} {} Q{} {} {} {}",
                        px,
                        py + s,
                        px + s / 2.0,
                        py,
                        px + s,
                        py + s / 2.0,
                    )
                    fill="none"
                    stroke="#7aa2f7"
                    stroke-width="1.5"
                    opacity="0.6"
                    pointer-events="none"
                />
            }
            .into_view(),
        }
    };

    view! {
        <svg
            _ref=svg_ref
            width="100%"
            height="100%"
            style="display: block;"
            viewBox=view_box
            on:pointermove=on_pointer_move
            on:wheel=on_wheel
        >
            <defs>
                <pattern id="grid" width="40" height="40" patternUnits="userSpaceOnUse">
                    <circle cx="0" cy="0" r="1.5" fill="#d1d5db" fill-opacity="0.25" />
                </pattern>
            </defs>

            <rect x="-100000" y="-100000" width="200000" height="200000" fill="url(#grid)" />

            <path d="M-12,0 L12,0 M0,-12 L0,12" stroke="#7aa2f7" stroke-width="2" />

            {preview}
        </svg>
    }
}
