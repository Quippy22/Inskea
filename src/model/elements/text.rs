use leptos::IntoView;
use std::sync::OnceLock;
use wasm_bindgen::JsCast;

use super::{ElementData, ShapeColor};
use super::{
    Bounds, FromDrag, HitTest, Offset, Render, Resize, ResizeContext, Rotate, SnapToGrid,
    UpdateDrag,
};
use super::rect::MIN_ELEMENT_SIZE;

pub(crate) const MIN_FONT_SIZE: f64 = 12.0;
pub(crate) const TEXT_ASCENT_RATIO: f64 = 0.85;

/// A piece of text rendered as SVG, with automatic word-wrapping.
#[derive(Clone, Debug)]
pub struct Text {
    /// Position, font size, and fill colour.
    pub data: ElementData,
    /// The raw text content (no hard line-breaks; wrapping is re-computed at render).
    pub content: String,
}

impl FromDrag for Text {
    fn from_drag(
        anchor: (f64, f64),
        _current: (f64, f64),
        color: ShapeColor,
        _shift: bool,
    ) -> Self {
        let mut data = ElementData::new(0);
        data.x = anchor.0;
        data.y = anchor.1;
        data.font_size = 24.0;
        data.width = 0.0;
        data.height = 0.0;
        data.stroke_color = color;
        Self {
            data,
            content: String::new(),
        }
    }
}

impl UpdateDrag for Text {
    fn update_drag(&mut self, _current: (f64, f64), _anchor: (f64, f64), _shift: bool) {
        // Text elements are placed on click, not dragged to size.
    }
}

impl Render for Text {
    fn render(&self, zoom: f64) -> leptos::View {
        let font_size = self.data.font_size.max(MIN_FONT_SIZE);
        let lines = wrap_lines(&self.content, self.data.width, font_size, zoom);
        let x = self.data.x;
        let baseline = self.data.y + font_size * TEXT_ASCENT_RATIO;
        let fill = self
            .data
            .fill_color
            .map(|c| c.to_hex())
            .unwrap_or_else(|| self.data.stroke_color.to_hex());

        let inner = if lines.len() <= 1 {
            leptos::view! {
                <text
                    x={x.to_string()}
                    y={baseline.to_string()}
                    fill=fill
                    font-size={font_size.to_string()}
                    font-family="sans-serif"
                    pointer-events="none"
                    style="user-select: none; -webkit-user-select: none;"
                >
                    {lines.first().cloned().unwrap_or_default()}
                </text>
            }
            .into_view()
        } else {
            leptos::view! {
                <text
                    x={x.to_string()}
                    y={baseline.to_string()}
                    fill=fill
                    font-size={font_size.to_string()}
                    font-family="sans-serif"
                    pointer-events="none"
                    style="user-select: none; -webkit-user-select: none;"
                >
                    {lines
                        .iter()
                        .enumerate()
                        .map(|(i, line)| {
                            let dy = if i == 0 { "0" } else { "1.2em" };
                            leptos::view! {
                                <tspan x={x.to_string()} dy=dy>
                                    {line.to_string()}
                                </tspan>
                            }
                        })
                        .collect::<Vec<_>>()}
                </text>
            }
            .into_view()
        };

        if self.data.rotation == 0.0 {
            inner
        } else {
            let cx = x + self.data.width / 2.0;
            let cy = self.data.y + self.data.height / 2.0;
            let deg = self.data.rotation.to_degrees();
            leptos::view! {
                <g transform={format!("rotate({} {} {})", deg, cx, cy)}>{inner}</g>
            }
            .into_view()
        }
    }
}

impl HitTest for Text {
    fn hit_test(&self, point: (f64, f64), margin: f64) -> bool {
        let (px, py) = point;
        px >= self.data.x - margin
            && px <= self.data.x + self.data.width + margin
            && py >= self.data.y - margin
            && py <= self.data.y + self.data.height + margin
    }
}

impl Bounds for Text {
    fn bounds(&self) -> (f64, f64, f64, f64) {
        (self.data.x, self.data.y, self.data.width, self.data.height)
    }
}

impl Offset for Text {
    fn offset(&mut self, dx: f64, dy: f64) {
        self.data.x += dx;
        self.data.y += dy;
    }
}

impl SnapToGrid for Text {
    fn snap_to_grid(&mut self, grid: f64) {
        let cx = self.data.x + self.data.width / 2.0;
        let cy = self.data.y + self.data.height / 2.0;
        let snapped_cx = (cx / grid).round() * grid;
        let snapped_cy = (cy / grid).round() * grid;
        self.data.x += snapped_cx - cx;
        self.data.y += snapped_cy - cy;
    }
}

impl Rotate for Text {
    fn rotate_around(&mut self, _cx: f64, _cy: f64, delta: f64) {
        self.data.rotation += delta;
    }
}

impl Resize for Text {
    fn resize(&mut self, ctx: &ResizeContext) {
        let rctx = ctx;
        let (mut nx, mut ny, mut nw, mut nh) = match rctx.handle {
            0 => (rctx.bx + rctx.dx, rctx.by + rctx.dy, rctx.bw - rctx.dx, rctx.bh - rctx.dy),
            1 => (rctx.bx, rctx.by + rctx.dy, rctx.bw, rctx.bh - rctx.dy),
            2 => (rctx.bx, rctx.by + rctx.dy, rctx.bw + rctx.dx, rctx.bh - rctx.dy),
            3 => (rctx.bx + rctx.dx, rctx.by, rctx.bw - rctx.dx, rctx.bh),
            4 => (rctx.bx, rctx.by, rctx.bw + rctx.dx, rctx.bh),
            5 => (rctx.bx + rctx.dx, rctx.by, rctx.bw - rctx.dx, rctx.bh + rctx.dy),
            6 => (rctx.bx, rctx.by, rctx.bw, rctx.bh + rctx.dy),
            7 => (rctx.bx, rctx.by, rctx.bw + rctx.dx, rctx.bh + rctx.dy),
            _ => return,
        };
        if rctx.shift {
            let ratio = rctx.bw / rctx.bh;
            let nratio = nw / nh;
            if nratio > ratio {
                nh = nw / ratio;
            } else {
                nw = nh * ratio;
            }
            match rctx.handle {
                0 => { nx = rctx.bx + rctx.bw - nw; ny = rctx.by + rctx.bh - nh; }
                1 => { ny = rctx.by + rctx.bh - nh; }
                2 => { ny = rctx.by + rctx.bh - nh; }
                3 => { nx = rctx.bx + rctx.bw - nw; }
                5 => { nx = rctx.bx + rctx.bw - nw; }
                _ => {}
            }
        }
        if nw < MIN_ELEMENT_SIZE || nh < MIN_ELEMENT_SIZE {
            return;
        }
        if rctx.multi {
            if let super::Element::Text(orig) = rctx.orig {
                let obw = rctx.bw.max(MIN_ELEMENT_SIZE);
                let obh = rctx.bh.max(MIN_ELEMENT_SIZE);
                let sx = nw / obw;
                let sy = nh / obh;
                self.data.x = (orig.data.x - rctx.bx) * sx + nx;
                self.data.y = (orig.data.y - rctx.by) * sy + ny;
                self.data.width = (orig.data.width * sx).max(MIN_ELEMENT_SIZE);
                self.data.height = (orig.data.height * sy).max(MIN_ELEMENT_SIZE);
            }
        } else {
            self.data.x = nx;
            self.data.y = ny;
            self.data.width = nw;
            self.data.height = nh;
        }
    }
}

// ── Text measurement helpers (canvas-based) ────────────────────────────

fn text_ctx() -> &'static web_sys::CanvasRenderingContext2d {
    static CTX: OnceLock<web_sys::CanvasRenderingContext2d> = OnceLock::new();
    CTX.get_or_init(|| {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("no document");
        let canvas = document
            .create_element("canvas")
            .expect("create canvas")
            .unchecked_into::<web_sys::HtmlCanvasElement>();
        canvas
            .get_context("2d")
            .ok()
            .flatten()
            .expect("get 2d context")
            .unchecked_into::<web_sys::CanvasRenderingContext2d>()
    })
}

fn text_width_px(text: &str, font_size: f64) -> f64 {
    let ctx = text_ctx();
    ctx.set_font(&format!("{}px sans-serif", font_size));
    ctx.measure_text(text)
        .ok()
        .map(|m| m.width())
        .unwrap_or_else(|| text.len() as f64 * font_size * 0.55)
}

fn wrap_lines(content: &str, max_world_width: f64, font_size: f64, zoom: f64) -> Vec<String> {
    if max_world_width <= 0.0 {
        return content.split('\n').map(|s| s.to_string()).collect();
    }
    let max_px = max_world_width * zoom;
    let mut out = Vec::new();
    for line in content.split('\n') {
        if line.is_empty() {
            out.push(String::new());
            continue;
        }
        let mut cur = String::new();
        for word in line.split(' ') {
            if cur.is_empty() {
                cur = word.to_string();
            } else if text_width_px(&format!("{} {}", cur, word), font_size) <= max_px {
                cur.push(' ');
                cur.push_str(word);
            } else {
                out.push(cur);
                cur = word.to_string();
            }
        }
        if !cur.is_empty() {
            out.push(cur);
        }
    }
    out
}
