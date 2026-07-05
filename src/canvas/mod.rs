mod viewport;
pub use viewport::Viewport;

use crate::model::{Element, ElementData, Point, Scene, ShapeColor};
use crate::ui::dock::Tool;
use leptos::ev;
use leptos::svg::Svg;
use leptos::*;

#[derive(Clone)]
struct DrawingState {
    anchor: (f64, f64),
    tool: Tool,
    color: ShapeColor,
}

fn build_element(
    anchor: (f64, f64),
    current: (f64, f64),
    tool: Tool,
    color: ShapeColor,
    shift: bool,
) -> Element {
    let (ax, ay) = anchor;
    let (cx, cy) = current;
    let mut data = ElementData::new(0);
    data.stroke_color = color;

    match tool {
        Tool::Rectangle | Tool::Ellipse => {
            let mut x = ax.min(cx);
            let mut y = ay.min(cy);
            let mut w = (cx - ax).abs();
            let mut h = (cy - ay).abs();
            if shift {
                let s = w.max(h);
                w = s;
                h = s;
                if cx < ax {
                    x = ax - s;
                }
                if cy < ay {
                    y = ay - s;
                }
            }
            if w < 1.0 {
                w = 1.0;
            }
            if h < 1.0 {
                h = 1.0;
            }
            data.x = x;
            data.y = y;
            data.width = w;
            data.height = h;
            if tool == Tool::Rectangle {
                Element::Rectangle(data)
            } else {
                Element::Ellipse(data)
            }
        }
        Tool::Line | Tool::Arrow => {
            let (mut ex, mut ey) = (cx, cy);
            if shift {
                let dx = cx - ax;
                let dy = cy - ay;
                let angle = dy.atan2(dx);
                let snapped =
                    (angle / (std::f64::consts::TAU / 8.0)).round() * (std::f64::consts::TAU / 8.0);
                let dist = (dx * dx + dy * dy).sqrt();
                ex = ax + dist * snapped.cos();
                ey = ay + dist * snapped.sin();
            }
            let a = Point { x: ax, y: ay };
            let b = Point { x: ex, y: ey };
            if tool == Tool::Line {
                Element::Line(data, a, b)
            } else {
                Element::Arrow(data, a, b)
            }
        }
        Tool::Text => Element::Text(data, "Text".into()),
        Tool::Freehand => Element::Freehand(data, vec![Point { x: cx, y: cy }]),
    }
}

fn update_drawing(element: &mut Element, current: (f64, f64), anchor: (f64, f64), shift: bool) {
    let (ax, ay) = anchor;
    let (cx, cy) = current;
    match element {
        Element::Rectangle(data) | Element::Ellipse(data) => {
            let mut x = ax.min(cx);
            let mut y = ay.min(cy);
            let mut w = (cx - ax).abs();
            let mut h = (cy - ay).abs();
            if shift {
                let s = w.max(h);
                w = s;
                h = s;
                if cx < ax {
                    x = ax - s;
                }
                if cy < ay {
                    y = ay - s;
                }
            }
            if w < 1.0 {
                w = 1.0;
            }
            if h < 1.0 {
                h = 1.0;
            }
            data.x = x;
            data.y = y;
            data.width = w;
            data.height = h;
        }
        Element::Line(_, a, b) | Element::Arrow(_, a, b) => {
            let (mut ex, mut ey) = (cx, cy);
            if shift {
                let dx = cx - ax;
                let dy = cy - ay;
                let angle = dy.atan2(dx);
                let snapped =
                    (angle / (std::f64::consts::TAU / 8.0)).round() * (std::f64::consts::TAU / 8.0);
                let dist = (dx * dx + dy * dy).sqrt();
                ex = ax + dist * snapped.cos();
                ey = ay + dist * snapped.sin();
            }
            a.x = ax;
            a.y = ay;
            b.x = ex;
            b.y = ey;
        }
        Element::Freehand(_, pts) => {
            pts.push(Point { x: cx, y: cy });
        }
        Element::Text(..) => {}
    }
}

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
    view! {
        <rect
            x=x
            y=y
            width=w
            height=h
            fill=fill
            stroke=stroke
            stroke-width=sw
            pointer-events="none"
        />
    }
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
    view! {
        <ellipse
            cx=cx
            cy=cy
            rx=rx
            ry=ry
            fill=fill
            stroke=stroke
            stroke-width=sw
            pointer-events="none"
        />
    }
    .into_view()
}

fn render_line(data: &ElementData, a: &Point, b: &Point) -> leptos::View {
    let sw = data.stroke_width;
    let stroke = stroke_hex(data.stroke_color);
    let (x1, y1) = (a.x, a.y);
    let (x2, y2) = (b.x, b.y);
    view! { <line x1=x1 y1=y1 x2=x2 y2=y2 stroke=stroke stroke-width=sw pointer-events="none" /> }
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
        <g stroke=hex stroke-width=sw fill=hex pointer-events="none">
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
        <text
            x=x
            y=y
            fill=fill
            font-size=font_size
            font-family="sans-serif"
            pointer-events="none"
            style="user-select: none;"
        >
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
    view! { <path d=d fill="none" stroke=stroke stroke-width=sw pointer-events="none" /> }
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
    let drawing = create_rw_signal(None::<DrawingState>);
    let freehand_anchor = create_rw_signal(None::<(f64, f64)>);
    let shift_pressed = create_rw_signal(false);

    let _ = window_event_listener(ev::resize, move |_| screen_size.set(window_size()));
    let _ = window_event_listener(ev::keydown, move |ev: ev::KeyboardEvent| {
        if ev.key() == "Shift" {
            shift_pressed.set(true);
        }
    });
    let _ = window_event_listener(ev::keyup, move |ev: ev::KeyboardEvent| {
        if ev.key() == "Shift" {
            shift_pressed.set(false);
        }
    });

    let update_world = move |ev: &ev::PointerEvent| {
        let screen = (ev.offset_x() as f64, ev.offset_y() as f64);
        cursor_screen.set(screen);
        let world = viewport.get().screen_to_world(screen, screen_size.get());
        cursor_world.set(world);
        world
    };

    let on_pointer_move = move |ev: ev::PointerEvent| {
        let world = update_world(&ev);
        if let Some(ref state) = drawing.get() {
            if state.tool == Tool::Freehand {
                if let Some(anchor) = freehand_anchor.get() {
                    scene.update(|s| {
                        if let Some(el) = s.elements.last_mut() {
                            update_drawing(el, world, anchor, ev.shift_key());
                        }
                    });
                }
            }
        }
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
        let world = update_world(&ev);
        let tool = selected_tool.get();
        let color = selected_color.get();

        if tool == Tool::Text {
            scene.update(|s| {
                let id = s.next_id();
                let mut data = ElementData::new(id);
                data.x = world.0;
                data.y = world.1;
                data.stroke_color = color;
                s.add_element(Element::Text(data, "Text".into()));
            });
            return;
        }

        if tool == Tool::Freehand {
            freehand_anchor.set(Some(world));
            scene.update(|s| {
                let id = s.next_id();
                let mut data = ElementData::new(id);
                data.stroke_color = color;
                s.add_element(Element::Freehand(
                    data,
                    vec![Point {
                        x: world.0,
                        y: world.1,
                    }],
                ));
            });
            drawing.set(Some(DrawingState {
                anchor: world,
                tool,
                color,
            }));
            return;
        }

        drawing.set(Some(DrawingState {
            anchor: world,
            tool,
            color,
        }));
    };

    let on_pointer_up = move |ev: ev::PointerEvent| {
        if let Some(state) = drawing.get() {
            if state.tool == Tool::Freehand {
                freehand_anchor.set(None);
                drawing.set(None);
                return;
            }

            let world = update_world(&ev);
            let el = build_element(
                state.anchor,
                world,
                state.tool,
                state.color,
                shift_pressed.get(),
            );
            scene.update(|s| {
                let mut el = el;
                let id = s.next_id();
                match &mut el {
                    Element::Rectangle(d)
                    | Element::Ellipse(d)
                    | Element::Line(d, ..)
                    | Element::Arrow(d, ..)
                    | Element::Text(d, ..)
                    | Element::Freehand(d, ..) => d.id = id,
                }
                s.elements.push(el);
            });
            drawing.set(None);
        }
    };

    let drawing_preview = move || {
        let state = drawing.get()?;
        if state.tool == Tool::Freehand {
            return None;
        }
        let world = cursor_world.get();
        let shift = shift_pressed.get();
        let el = build_element(state.anchor, world, state.tool, state.color, shift);
        Some(render_element(&el))
    };

    view! {
        <svg
            _ref=svg_ref
            width="100%"
            height="100%"
            style="display: block; user-select: none;"
            viewBox=view_box
            on:pointerdown=on_pointer_down
            on:pointermove=on_pointer_move
            on:pointerup=on_pointer_up
            on:wheel=on_wheel
        >
            <defs>
                <pattern id="grid" width="40" height="40" patternUnits="userSpaceOnUse">
                    <circle cx="0" cy="0" r="1.5" fill="#d1d5db" fill-opacity="0.25" />
                </pattern>
            </defs>

            <rect x="-100000" y="-100000" width="200000" height="200000" fill="url(#grid)" />

            <path d="M-12,0 L12,0 M0,-12 L0,12" stroke="#7aa2f7" stroke-width="2" />

            {move || scene.get().elements.iter().map(render_element).collect_view()}

            {move || drawing_preview()}
        </svg>
    }
}
