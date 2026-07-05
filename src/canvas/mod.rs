mod viewport;
pub use viewport::Viewport;

use crate::model::{Element, ElementData, Point, Scene, ShapeColor};
use crate::ui::dock::Tool;
use leptos::ev;
use leptos::svg::Svg;
use leptos::*;

const PREVIEW_SIZE: f64 = 10.0;
const PREVIEW_OFFSET: f64 = 6.0;

fn render_element(element: &Element) -> leptos::View {
    match element {
        Element::Rectangle(data) => render_rect(data),
        Element::Ellipse(data) => render_ellipse(data),
        Element::Line(data, a, b) => render_line(data, a, b),
        Element::Arrow(data, a, b) => render_arrow(data, a, b),
        Element::Text(data, content) => render_text(data, content),
        Element::Freehand(data, pts) => render_freehand(data, pts),
    }
}

fn fill_hex(fill: &Option<ShapeColor>) -> &'static str {
    match fill {
        Some(_) => "currentColor",
        None => "none",
    }
}

fn stroke_hex(stroke: ShapeColor) -> &'static str {
    stroke.to_hex()
}

fn render_rect(data: &ElementData) -> leptos::View {
    let x = data.x;
    let y = data.y;
    let w = data.width;
    let h = data.height;
    let sw = data.stroke_width;
    let fill = fill_hex(&data.fill_color);
    let stroke = stroke_hex(data.stroke_color);
    view! { <rect x=x y=y width=w height=h fill=fill stroke=stroke stroke-width=sw /> }
    .into_view()
}

fn render_ellipse(data: &ElementData) -> leptos::View {
    let x = data.x;
    let y = data.y;
    let w = data.width;
    let h = data.height;
    let sw = data.stroke_width;
    let fill = fill_hex(&data.fill_color);
    let stroke = stroke_hex(data.stroke_color);
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let rx = w / 2.0;
    let ry = h / 2.0;
    view! { <ellipse cx=cx cy=cy rx=rx ry=ry fill=fill stroke=stroke stroke-width=sw /> }
    .into_view()
}

fn render_line(data: &ElementData, a: &Point, b: &Point) -> leptos::View {
    let sw = data.stroke_width;
    let stroke = stroke_hex(data.stroke_color);
    let (x1, y1) = (a.x, a.y);
    let (x2, y2) = (b.x, b.y);
    view! { <line x1=x1 y1=y1 x2=x2 y2=y2 stroke=stroke stroke-width=sw /> }
    .into_view()
}

fn render_arrow(data: &ElementData, a: &Point, b: &Point) -> leptos::View {
    let sw = data.stroke_width;
    let hex = stroke_hex(data.stroke_color);
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    let (ux, uy) = if len > 0.0 {
        (dx / len, dy / len)
    } else {
        (1.0, 0.0)
    };
    let head_size = 8.0;
    let tip_x = b.x;
    let tip_y = b.y;
    let lx = tip_x - ux * head_size - uy * head_size * 0.4;
    let ly = tip_y - uy * head_size + ux * head_size * 0.4;
    let rx = tip_x - ux * head_size + uy * head_size * 0.4;
    let ry = tip_y - uy * head_size - ux * head_size * 0.4;
    let (ax, ay) = (a.x, a.y);
    let (bx, by) = (b.x, b.y);
    let points = format!("{},{} {},{} {},{}", tip_x, tip_y, lx, ly, rx, ry);
    view! {
        <g stroke=hex stroke-width=sw fill=hex>
            <line x1=ax y1=ay x2=bx y2=by />
            <polyline points=points />
        </g>
    }
    .into_view()
}

fn render_text(data: &ElementData, content: &str) -> leptos::View {
    let x = data.x;
    let y = data.y;
    let font_size = data.width.max(12.0);
    let fill = data
        .fill_color
        .map(|c| c.to_hex())
        .unwrap_or_else(|| data.stroke_color.to_hex());
    let content = content.to_string();
    view! {
        <text x=x y=y fill=fill font-size=font_size font-family="sans-serif">
            {content}
        </text>
    }
    .into_view()
}

fn render_freehand(data: &ElementData, pts: &[Point]) -> leptos::View {
    let sw = data.stroke_width;
    let stroke = stroke_hex(data.stroke_color);
    let d = if pts.is_empty() {
        String::new()
    } else {
        let mut d = format!("M{} {}", pts[0].x, pts[0].y);
        for p in &pts[1..] {
            use std::fmt::Write;
            let _ = write!(d, " L{} {}", p.x, p.y);
        }
        d
    };
    view! { <path d=d fill="none" stroke=stroke stroke-width=sw /> }
    .into_view()
}

#[component]
pub fn Canvas(
    cursor_screen: RwSignal<(f64, f64)>,
    cursor_world: RwSignal<(f64, f64)>,
    viewport: RwSignal<Viewport>,
    selected_tool: RwSignal<Tool>,
    selected_color: RwSignal<ShapeColor>,
    scene: RwSignal<Scene>,
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

    let on_pointer_down = move |ev: ev::PointerEvent| {
        let screen = (ev.offset_x() as f64, ev.offset_y() as f64);
        let (sw, sh) = screen_size.get();
        let world = viewport.get().screen_to_world(screen, (sw, sh));
        let (wx, wy) = world;

        let tool = selected_tool.get();
        let color = selected_color.get();

        scene.update(|s| {
            let id = s.next_id();
            let mut data = ElementData::new(id);
            data.x = wx;
            data.y = wy;
            data.stroke_color = color;

            let element = match tool {
                Tool::Rectangle => Element::Rectangle(data),
                Tool::Ellipse => Element::Ellipse(data),
                Tool::Line => Element::Line(
                    data,
                    Point { x: wx, y: wy },
                    Point {
                        x: wx + 100.0,
                        y: wy + 100.0,
                    },
                ),
                Tool::Arrow => Element::Arrow(
                    data,
                    Point { x: wx, y: wy },
                    Point {
                        x: wx + 100.0,
                        y: wy - 50.0,
                    },
                ),
                Tool::Text => Element::Text(data, "Hello".into()),
                Tool::Freehand => Element::Freehand(data, vec![Point { x: wx, y: wy }]),
            };

            s.add_element(element);
        });
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
                    cx=px + s / 2.0
                    cy=py + s / 2.0
                    rx=s / 2.0
                    ry=s / 2.0
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
                    y1=py + s
                    x2=px + s
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
                    <line x1=px y1=py + s x2=px + s y2=py />
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
                    x=px + s / 2.0
                    y=py + s / 2.0
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
            on:pointerdown=on_pointer_down
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

            {move || { scene.get().elements.iter().map(render_element).collect_view() }}

            {preview}
        </svg>
    }
}
