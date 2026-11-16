//! Native vector PDF document generation for Inskea.
//!
//! Produces compliant, self-contained PDF 1.4 documents containing one or more pages
//! with native vector graphics, selectable/searchable text, and embedded raster images.
//! Supports arbitrary per-page paper dimensions (e.g. ISO A2, A3, A4, B-series, or custom
//! freehand sizes).
#![allow(clippy::write_with_newline)]

use crate::canvas::settings::Theme;
use crate::model::Color;
use crate::model::elements::chart::ChartSceneGraph;
use crate::model::elements::path::segment_cubic_bezier;
use crate::model::elements::text::{MIN_FONT_SIZE, TEXT_ASCENT_RATIO};
use crate::model::elements::utils::arrowhead_polyline;
use crate::model::elements::{CurveMode, EdgeStyle, Element, StrokeStyle};
use crate::model::Rect;
use std::io::Write;

/// Convert CSS / Canvas pixels (at 96 DPI) to PDF points (at 72 DPI).
pub fn px_to_pt(px: f64) -> f64 {
    px * 72.0 / 96.0
}

/// A raster image embedded in a PDF page.
#[derive(Clone, Debug, PartialEq)]
pub struct PdfImage {
    pub id: usize,
    pub width: u32,
    pub height: u32,
    pub jpeg_data: Vec<u8>,
}

/// A single page in the PDF document.
#[derive(Clone, Debug, PartialEq)]
pub struct PdfPage {
    /// Page width in PDF points (1 pt = 1/72 inch).
    pub width_pt: f64,
    /// Page height in PDF points.
    pub height_pt: f64,
    /// PDF content stream containing native vector operators.
    pub contents: Vec<u8>,
    /// Any embedded raster images referenced by `/Im{id}`.
    pub images: Vec<PdfImage>,
    /// Unique opacity values (0.0 to 1.0) used on this page for `/ExtGState`.
    pub opacities: Vec<f64>,
}

// ── Text metrics for standard PDF Helvetica ─────────────────────────────

fn helvetica_char_width(c: char) -> f64 {
    match c {
        'i' | 'l' | '!' | '|' | ':' | ';' | '.' | ',' | '\'' => 0.222,
        'j' | 'r' | 't' | 'f' | '(' | ')' | '[' | ']' | '-' => 0.278,
        ' ' => 0.278,
        's' | 'c' | 'z' | 'k' | 'v' => 0.444,
        'a' | 'b' | 'd' | 'e' | 'g' | 'h' | 'n' | 'o' | 'p' | 'q' | 'u' | 'x' | 'y' => 0.556,
        '0'..='9' => 0.556,
        'w' | 'm' => 0.778,
        'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'G' | 'H' | 'K' | 'N' | 'O' | 'P' | 'Q' | 'R'
        | 'T' | 'U' | 'V' | 'X' | 'Y' | 'Z' => 0.667,
        'I' => 0.278,
        'J' => 0.500,
        'L' => 0.556,
        'M' | 'W' => 0.833,
        _ => 0.500,
    }
}

fn helvetica_text_width(text: &str, font_size_pt: f64) -> f64 {
    text.chars().map(helvetica_char_width).sum::<f64>() * font_size_pt
}

fn escape_pdf_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for b in s.as_bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'(' => out.push_str("\\("),
            b')' => out.push_str("\\)"),
            b'\r' => out.push_str("\\r"),
            b'\n' => out.push_str("\\n"),
            32..=126 => out.push(*b as char),
            _ => {
                use std::fmt::Write;
                let _ = write!(out, "\\{:03o}", b);
            }
        }
    }
    out
}

// ── Image extraction helpers ───────────────────────────────────────────

fn jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    let mut i = 0;
    while i + 8 < data.len() {
        if data[i] == 0xFF {
            let marker = data[i + 1];
            if marker == 0xC0 || marker == 0xC1 || marker == 0xC2 {
                let h = u16::from_be_bytes([data[i + 5], data[i + 6]]) as u32;
                let w = u16::from_be_bytes([data[i + 7], data[i + 8]]) as u32;
                return Some((w, h));
            }
            if marker == 0xD8 || marker == 0xD9 {
                i += 2;
                continue;
            }
            let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
            i += 2 + len;
        } else {
            i += 1;
        }
    }
    None
}

// ── SVG Path tokenizer & converter to PDF operators ────────────────────

enum PathToken {
    Cmd(char),
    Num(f64),
}

fn tokenize_svg_path(d: &str) -> Vec<PathToken> {
    let mut tokens = Vec::new();
    let bytes = d.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_whitespace() || b == b',' {
            i += 1;
            continue;
        }
        if b.is_ascii_alphabetic() {
            tokens.push(PathToken::Cmd(b as char));
            i += 1;
            continue;
        }
        let start = i;
        if bytes[i] == b'-' || bytes[i] == b'+' {
            i += 1;
        }
        while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
            i += 1;
        }
        if let Ok(val) = std::str::from_utf8(&bytes[start..i]).unwrap_or("").parse::<f64>() {
            tokens.push(PathToken::Num(val));
        }
    }
    tokens
}

fn render_svg_path(d: &str, view: Rect, out: &mut Vec<u8>) {
    let tokens = tokenize_svg_path(d);
    let map_x = |x: f64| -> f64 { px_to_pt(x - view.x) };
    let map_y = |y: f64| -> f64 { px_to_pt(view.h - (y - view.y)) };

    let mut cur_x = 0.0;
    let mut cur_y = 0.0;
    let mut start_x = 0.0;
    let mut start_y = 0.0;

    let mut i = 0;
    let mut current_cmd = ' ';

    while i < tokens.len() {
        match tokens[i] {
            PathToken::Cmd(c) => {
                current_cmd = c;
                i += 1;
            }
            PathToken::Num(_) => {
                // If numbers follow, repeat the previous command (except M becomes L)
                if current_cmd == 'M' {
                    current_cmd = 'L';
                } else if current_cmd == 'm' {
                    current_cmd = 'l';
                }
            }
        }

        match current_cmd {
            'M' => {
                if i + 1 < tokens.len() {
                    if let (PathToken::Num(x), PathToken::Num(y)) = (&tokens[i], &tokens[i + 1]) {
                        let _ = write!(out, "{:.2} {:.2} m\n", map_x(*x), map_y(*y));
                        cur_x = *x;
                        cur_y = *y;
                        start_x = *x;
                        start_y = *y;
                        i += 2;
                    }
                }
            }
            'm' => {
                if i + 1 < tokens.len() {
                    if let (PathToken::Num(dx), PathToken::Num(dy)) = (&tokens[i], &tokens[i + 1]) {
                        cur_x += *dx;
                        cur_y += *dy;
                        let _ = write!(out, "{:.2} {:.2} m\n", map_x(cur_x), map_y(cur_y));
                        start_x = cur_x;
                        start_y = cur_y;
                        i += 2;
                    }
                }
            }
            'L' => {
                if i + 1 < tokens.len() {
                    if let (PathToken::Num(x), PathToken::Num(y)) = (&tokens[i], &tokens[i + 1]) {
                        let _ = write!(out, "{:.2} {:.2} l\n", map_x(*x), map_y(*y));
                        cur_x = *x;
                        cur_y = *y;
                        i += 2;
                    }
                }
            }
            'l' => {
                if i + 1 < tokens.len() {
                    if let (PathToken::Num(dx), PathToken::Num(dy)) = (&tokens[i], &tokens[i + 1]) {
                        cur_x += *dx;
                        cur_y += *dy;
                        let _ = write!(out, "{:.2} {:.2} l\n", map_x(cur_x), map_y(cur_y));
                        i += 2;
                    }
                }
            }
            'C' => {
                if i + 5 < tokens.len() {
                    if let (
                        PathToken::Num(x1),
                        PathToken::Num(y1),
                        PathToken::Num(x2),
                        PathToken::Num(y2),
                        PathToken::Num(x),
                        PathToken::Num(y),
                    ) = (
                        &tokens[i],
                        &tokens[i + 1],
                        &tokens[i + 2],
                        &tokens[i + 3],
                        &tokens[i + 4],
                        &tokens[i + 5],
                    ) {
                        let _ = write!(
                            out,
                            "{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n",
                            map_x(*x1),
                            map_y(*y1),
                            map_x(*x2),
                            map_y(*y2),
                            map_x(*x),
                            map_y(*y)
                        );
                        cur_x = *x;
                        cur_y = *y;
                        i += 6;
                    }
                }
            }
            'Q' => {
                if i + 3 < tokens.len() {
                    if let (
                        PathToken::Num(x1),
                        PathToken::Num(y1),
                        PathToken::Num(x),
                        PathToken::Num(y),
                    ) = (&tokens[i], &tokens[i + 1], &tokens[i + 2], &tokens[i + 3]) {
                        let c1_x = cur_x + (2.0 / 3.0) * (*x1 - cur_x);
                        let c1_y = cur_y + (2.0 / 3.0) * (*y1 - cur_y);
                        let c2_x = *x + (2.0 / 3.0) * (*x1 - *x);
                        let c2_y = *y + (2.0 / 3.0) * (*y1 - *y);
                        let _ = write!(
                            out,
                            "{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n",
                            map_x(c1_x),
                            map_y(c1_y),
                            map_x(c2_x),
                            map_y(c2_y),
                            map_x(*x),
                            map_y(*y)
                        );
                        cur_x = *x;
                        cur_y = *y;
                        i += 4;
                    }
                }
            }
            'A' | 'a' => {
                if i + 6 < tokens.len() {
                    if let (
                        PathToken::Num(rx),
                        PathToken::Num(ry),
                        PathToken::Num(_rot),
                        PathToken::Num(large),
                        PathToken::Num(sweep),
                        PathToken::Num(mut end_x),
                        PathToken::Num(mut end_y),
                    ) = (
                        &tokens[i],
                        &tokens[i + 1],
                        &tokens[i + 2],
                        &tokens[i + 3],
                        &tokens[i + 4],
                        &tokens[i + 5],
                        &tokens[i + 6],
                    ) {
                        if current_cmd == 'a' {
                            end_x += cur_x;
                            end_y += cur_y;
                        }
                        let r = (rx.abs() + ry.abs()) / 2.0;
                        let dx = end_x - cur_x;
                        let dy = end_y - cur_y;
                        let d = dx.hypot(dy);
                        if d > 1e-4 && r >= d / 2.0 {
                            let h = (r * r - (d / 2.0) * (d / 2.0)).max(0.0).sqrt();
                            let perp_x = -dy / d;
                            let perp_y = dx / d;
                            let sign = if (*large > 0.5) == (*sweep > 0.5) {
                                -1.0
                            } else {
                                1.0
                            };
                            let cx = (cur_x + end_x) / 2.0 + sign * h * perp_x;
                            let cy = (cur_y + end_y) / 2.0 + sign * h * perp_y;
                            let mut a1 = (cur_y - cy).atan2(cur_x - cx);
                            let mut a2 = (end_y - cy).atan2(end_x - cx);
                            if *sweep > 0.5 && a2 < a1 {
                                a2 += std::f64::consts::TAU;
                            } else if *sweep <= 0.5 && a1 < a2 {
                                a1 += std::f64::consts::TAU;
                            }
                            let sweep_angle = a2 - a1;
                            let segs = ((sweep_angle.abs() / (std::f64::consts::FRAC_PI_2)).ceil()
                                as usize)
                                .max(1);
                            let dt = sweep_angle / segs as f64;
                            for s in 0..segs {
                                let t1 = a1 + s as f64 * dt;
                                let t2 = t1 + dt;
                                let alpha = (4.0 / 3.0) * ((t2 - t1) / 4.0).tan();
                                let (sin1, cos1) = t1.sin_cos();
                                let (sin2, cos2) = t2.sin_cos();
                                let p1x = cx + r * cos1;
                                let p1y = cy + r * sin1;
                                let p2x = cx + r * cos2;
                                let p2y = cy + r * sin2;
                                let c1x = p1x - alpha * r * sin1;
                                let c1y = p1y + alpha * r * cos1;
                                let c2x = p2x + alpha * r * sin2;
                                let c2y = p2y - alpha * r * cos2;
                                let _ = write!(
                                    out,
                                    "{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n",
                                    map_x(c1x),
                                    map_y(c1y),
                                    map_x(c2x),
                                    map_y(c2y),
                                    map_x(p2x),
                                    map_y(p2y)
                                );
                            }
                        } else {
                            let _ = write!(out, "{:.2} {:.2} l\n", map_x(end_x), map_y(end_y));
                        }
                        cur_x = end_x;
                        cur_y = end_y;
                        i += 7;
                    }
                }
            }
            'Z' | 'z' => {
                let _ = write!(out, "h\n");
                cur_x = start_x;
                cur_y = start_y;
            }
            _ => {
                i += 1;
            }
        }
    }
}

fn render_polyline(pts: &str, view: Rect, out: &mut Vec<u8>) {
    let mut coords = Vec::new();
    for token in pts.split_whitespace() {
        if let Some((x_str, y_str)) = token.split_once(',') {
            if let (Ok(x), Ok(y)) = (x_str.parse::<f64>(), y_str.parse::<f64>()) {
                coords.push((x, y));
            }
        }
    }
    if coords.len() < 2 {
        return;
    }
    let (x0, y0) = coords[0];
    let _ = write!(
        out,
        "{:.2} {:.2} m\n",
        px_to_pt(x0 - view.x),
        px_to_pt(view.h - (y0 - view.y))
    );
    for &(x, y) in &coords[1..] {
        let _ = write!(
            out,
            "{:.2} {:.2} l\n",
            px_to_pt(x - view.x),
            px_to_pt(view.h - (y - view.y))
        );
    }
    let _ = write!(out, "S\n");
}

// ── Context for emitting a vector PDF page ─────────────────────────────

struct PageBuilder {
    view: Rect,
    out: Vec<u8>,
    images: Vec<PdfImage>,
    opacities: Vec<f64>,
}

impl PageBuilder {
    fn new(view: Rect) -> Self {
        let mut out = Vec::new();
        // Page bounds clipping
        let w_pt = px_to_pt(view.w);
        let h_pt = px_to_pt(view.h);
        let _ = write!(out, "0 0 {:.2} {:.2} re W n\n", w_pt, h_pt);

        Self {
            view,
            out,
            images: Vec::new(),
            opacities: vec![1.0],
        }
    }

    fn map_x(&self, x: f64) -> f64 {
        px_to_pt(x - self.view.x)
    }

    fn map_y(&self, y: f64) -> f64 {
        px_to_pt(self.view.h - (y - self.view.y))
    }

    fn register_opacity(&mut self, op: f64) -> usize {
        let op_clamped = (op.clamp(0.0, 1.0) * 100.0).round() / 100.0;
        if let Some(pos) = self
            .opacities
            .iter()
            .position(|&o| (o - op_clamped).abs() < 1e-3)
        {
            pos
        } else {
            self.opacities.push(op_clamped);
            self.opacities.len() - 1
        }
    }

    fn set_opacity(&mut self, op: f64) {
        if (op - 1.0).abs() > 1e-3 {
            let idx = self.register_opacity(op);
            let _ = write!(self.out, "/GS{} gs\n", idx);
        } else {
            let _ = write!(self.out, "/GS0 gs\n");
        }
    }

    fn set_stroke_style(&mut self, style: StrokeStyle) {
        match style {
            StrokeStyle::Solid => {
                let _ = write!(self.out, "[] 0 d\n");
            }
            StrokeStyle::Dashed => {
                let _ = write!(self.out, "[{:.2} {:.2}] 0 d\n", px_to_pt(6.0), px_to_pt(3.0));
            }
            StrokeStyle::Dotted => {
                let _ = write!(self.out, "[{:.2} {:.2}] 0 d\n", px_to_pt(2.0), px_to_pt(4.0));
            }
        }
    }

    fn set_edge_style(&mut self, style: EdgeStyle) {
        match style {
            EdgeStyle::Sharp => {
                let _ = write!(self.out, "0 j 0 J\n");
            }
            EdgeStyle::Rounded => {
                let _ = write!(self.out, "1 j 1 J\n");
            }
        }
    }

    fn set_stroke_color(&mut self, color: Color) {
        let (r, g, b) = color.to_pdf_rgb();
        let _ = write!(self.out, "{:.3} {:.3} {:.3} RG\n", r, g, b);
    }

    fn set_fill_color(&mut self, color: Color) {
        let (r, g, b) = color.to_pdf_rgb();
        let _ = write!(self.out, "{:.3} {:.3} {:.3} rg\n", r, g, b);
    }

    fn push_rotation(&mut self, cx: f64, cy: f64, rot: f64) {
        if rot.abs() < 1e-4 {
            return;
        }
        let cx_pt = self.map_x(cx);
        let cy_pt = self.map_y(cy);
        let cos = rot.cos();
        let sin = rot.sin();
        let a = cos;
        let b = -sin;
        let c = sin;
        let d = cos;
        let e = cx_pt - cx_pt * cos - cy_pt * sin;
        let f = cy_pt + cx_pt * sin - cy_pt * cos;
        let _ = write!(
            self.out,
            "q\n{:.6} {:.6} {:.6} {:.6} {:.2} {:.2} cm\n",
            a, b, c, d, e, f
        );
    }

    fn pop_rotation(&mut self, rot: f64) {
        if rot.abs() >= 1e-4 {
            let _ = write!(self.out, "Q\n");
        }
    }

    fn render_rect_path(&mut self, x: f64, y: f64, w: f64, h: f64, roundness: f64) {
        let x_pt = self.map_x(x);
        let y_pt = self.map_y(y + h);
        let w_pt = px_to_pt(w);
        let h_pt = px_to_pt(h);

        let r = px_to_pt(roundness).min(w_pt / 2.0).min(h_pt / 2.0);
        if r <= 0.5 {
            let _ = write!(self.out, "{:.2} {:.2} {:.2} {:.2} re\n", x_pt, y_pt, w_pt, h_pt);
        } else {
            let k = 0.55228475;
            let rk = r * k;
            let _ = write!(
                self.out,
                "{:.2} {:.2} m\n{:.2} {:.2} l\n{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n{:.2} {:.2} l\n{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n{:.2} {:.2} l\n{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n{:.2} {:.2} l\n{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\nh\n",
                x_pt + r, y_pt,
                x_pt + w_pt - r, y_pt,
                x_pt + w_pt - r + rk, y_pt, x_pt + w_pt, y_pt + r - rk, x_pt + w_pt, y_pt + r,
                x_pt + w_pt, y_pt + h_pt - r,
                x_pt + w_pt, y_pt + h_pt - r + rk, x_pt + w_pt - r + rk, y_pt + h_pt, x_pt + w_pt - r, y_pt + h_pt,
                x_pt + r, y_pt + h_pt,
                x_pt + r - rk, y_pt + h_pt, x_pt, y_pt + h_pt - r + rk, x_pt, y_pt + h_pt - r,
                x_pt, y_pt + r,
                x_pt, y_pt + r - rk, x_pt + r - rk, y_pt, x_pt + r, y_pt
            );
        }
    }

    fn render_ellipse_path(&mut self, cx: f64, cy: f64, rx: f64, ry: f64) {
        let cx_pt = self.map_x(cx);
        let cy_pt = self.map_y(cy);
        let rx_pt = px_to_pt(rx);
        let ry_pt = px_to_pt(ry);
        let kx = rx_pt * 0.55228475;
        let ky = ry_pt * 0.55228475;

        let _ = write!(
            self.out,
            "{:.2} {:.2} m\n{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\nh\n",
            cx_pt + rx_pt, cy_pt,
            cx_pt + rx_pt, cy_pt + ky, cx_pt + kx, cy_pt + ry_pt, cx_pt, cy_pt + ry_pt,
            cx_pt - kx, cy_pt + ry_pt, cx_pt - rx_pt, cy_pt + ky, cx_pt - rx_pt, cy_pt,
            cx_pt - rx_pt, cy_pt - ky, cx_pt - kx, cy_pt - ry_pt, cx_pt, cy_pt - ry_pt,
            cx_pt + kx, cy_pt - ry_pt, cx_pt + rx_pt, cy_pt - ky, cx_pt + rx_pt, cy_pt
        );
    }
}

// ── Element render dispatch ────────────────────────────────────────────

fn render_element(el: &Element, b: &mut PageBuilder) {
    match el {
        Element::Rectangle(r) => {
            let cx = r.data.world_point.x + r.data.width / 2.0;
            let cy = r.data.world_point.y + r.data.height / 2.0;
            b.push_rotation(cx, cy, r.data.rotation);
            b.set_opacity(r.data.style.opacity);
            b.set_stroke_style(r.data.style.stroke_style);
            b.set_edge_style(r.data.style.edge_style);
            let sw = px_to_pt(r.data.style.stroke_width);
            let _ = write!(b.out, "{:.2} w\n", sw);
            b.set_stroke_color(r.data.style.stroke_color);

            let roundness = match r.data.style.edge_style {
                EdgeStyle::Rounded => r.data.style.roundness,
                EdgeStyle::Sharp => 0.0,
            };
            b.render_rect_path(
                r.data.world_point.x,
                r.data.world_point.y,
                r.data.width,
                r.data.height,
                roundness,
            );

            if let Some(fill) = r.data.style.fill_color {
                b.set_fill_color(fill);
                let _ = write!(b.out, "B\n");
            } else {
                let _ = write!(b.out, "S\n");
            }
            b.pop_rotation(r.data.rotation);
        }
        Element::Ellipse(e) => {
            let cx = e.data.world_point.x + e.data.width / 2.0;
            let cy = e.data.world_point.y + e.data.height / 2.0;
            b.push_rotation(cx, cy, e.data.rotation);
            b.set_opacity(e.data.style.opacity);
            b.set_stroke_style(e.data.style.stroke_style);
            b.set_edge_style(e.data.style.edge_style);
            let sw = px_to_pt(e.data.style.stroke_width);
            let _ = write!(b.out, "{:.2} w\n", sw);
            b.set_stroke_color(e.data.style.stroke_color);

            b.render_ellipse_path(cx, cy, e.data.width / 2.0, e.data.height / 2.0);

            if let Some(fill) = e.data.style.fill_color {
                b.set_fill_color(fill);
                let _ = write!(b.out, "B\n");
            } else {
                let _ = write!(b.out, "S\n");
            }
            b.pop_rotation(e.data.rotation);
        }
        Element::Line(l) => {
            b.set_opacity(l.data.style.opacity);
            b.set_stroke_style(l.data.style.stroke_style);
            b.set_edge_style(l.data.style.edge_style);
            let sw = px_to_pt(l.data.style.stroke_width);
            let _ = write!(b.out, "{:.2} w\n", sw);
            b.set_stroke_color(l.data.style.stroke_color);

            let n = l.points.len();
            if n >= 2 {
                if n == 2 || l.line_style.curve_mode == CurveMode::Straight {
                    let _ = write!(
                        b.out,
                        "{:.2} {:.2} m\n",
                        b.map_x(l.points[0].x),
                        b.map_y(l.points[0].y)
                    );
                    for p in &l.points[1..] {
                        let _ = write!(b.out, "{:.2} {:.2} l\n", b.map_x(p.x), b.map_y(p.y));
                    }
                    let _ = write!(b.out, "S\n");
                } else {
                    let _ = write!(
                        b.out,
                        "{:.2} {:.2} m\n",
                        b.map_x(l.points[0].x),
                        b.map_y(l.points[0].y)
                    );
                    for i in 0..n - 1 {
                        if let Some((_, b1, b2, p1)) = segment_cubic_bezier(&l.points, i) {
                            let _ = write!(
                                b.out,
                                "{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n",
                                b.map_x(b1.x),
                                b.map_y(b1.y),
                                b.map_x(b2.x),
                                b.map_y(b2.y),
                                b.map_x(p1.x),
                                b.map_y(p1.y)
                            );
                        }
                    }
                    let _ = write!(b.out, "S\n");
                }

                // Arrowheads
                let raw_sw = l.data.style.stroke_width;
                if l.line_style.has_start_arrowhead {
                    let pts = arrowhead_polyline(&l.points[1], &l.points[0], raw_sw);
                    render_polyline(&pts, b.view, &mut b.out);
                }
                if l.line_style.has_end_arrowhead {
                    let pts = arrowhead_polyline(&l.points[n - 2], &l.points[n - 1], raw_sw);
                    render_polyline(&pts, b.view, &mut b.out);
                }
            }
        }
        Element::Connector(c) => {
            b.set_opacity(c.data.style.opacity);
            b.set_stroke_style(c.data.style.stroke_style);
            b.set_edge_style(c.data.style.edge_style);
            let sw = px_to_pt(c.data.style.stroke_width);
            let _ = write!(b.out, "{:.2} w\n", sw);
            b.set_stroke_color(c.data.style.stroke_color);

            let d = c.path_d();
            if !d.is_empty() {
                render_svg_path(&d, b.view, &mut b.out);
                let _ = write!(b.out, "S\n");
            }

            let raw_sw = c.data.style.stroke_width;
            let (start_arrow, end_arrow) = c.arrowheads(raw_sw);
            for pts in [start_arrow, end_arrow].into_iter().flatten() {
                render_polyline(&pts, b.view, &mut b.out);
            }
        }
        Element::Text(t) => {
            let cx = t.data.world_point.x + t.data.width / 2.0;
            let cy = t.data.world_point.y + t.data.height / 2.0;
            b.push_rotation(cx, cy, t.data.rotation);
            b.set_opacity(t.data.style.opacity);

            let fs = t.data.style.font_size.max(MIN_FONT_SIZE);
            let fs_pt = px_to_pt(fs);
            b.set_fill_color(t.data.style.stroke_color);

            let base_y0 = t.data.world_point.y + fs * TEXT_ASCENT_RATIO;
            for (i, line) in t.wrapped.lines.iter().enumerate() {
                let y_screen = base_y0 + i as f64 * (fs * 1.2);
                let y_pt = b.map_y(y_screen);
                let line_w = helvetica_text_width(line, fs_pt);

                let x_pt = match t.align {
                    crate::model::TextAlign::Left => b.map_x(t.data.world_point.x),
                    crate::model::TextAlign::Center => {
                        b.map_x(t.data.world_point.x + t.data.width / 2.0) - line_w / 2.0
                    }
                    crate::model::TextAlign::Right => {
                        b.map_x(t.data.world_point.x + t.data.width) - line_w
                    }
                };

                let escaped = escape_pdf_text(line);
                let _ = write!(
                    b.out,
                    "BT\n/F1 {:.2} Tf\n1 0 0 1 {:.2} {:.2} Tm\n({}) Tj\nET\n",
                    fs_pt, x_pt, y_pt, escaped
                );
            }

            b.pop_rotation(t.data.rotation);
        }
        Element::Freehand(f) => {
            b.set_opacity(f.data.style.opacity);
            b.set_stroke_style(f.data.style.stroke_style);
            b.set_edge_style(f.data.style.edge_style);
            let sw = px_to_pt(f.data.style.stroke_width);
            let _ = write!(b.out, "{:.2} w\n", sw);
            b.set_stroke_color(f.data.style.stroke_color);

            let d = crate::model::elements::freehand::build_smooth_path(&f.points);
            if !d.is_empty() {
                render_svg_path(&d, b.view, &mut b.out);
                let _ = write!(b.out, "S\n");
            }
        }
        Element::Image(i) => {
            let cx = i.data.world_point.x + i.data.width / 2.0;
            let cy = i.data.world_point.y + i.data.height / 2.0;
            b.push_rotation(cx, cy, i.data.rotation);
            b.set_opacity(i.data.style.opacity);

            let x_pt = b.map_x(i.data.world_point.x);
            let y_pt = b.map_y(i.data.world_point.y + i.data.height);
            let w_pt = px_to_pt(i.data.width);
            let h_pt = px_to_pt(i.data.height);

            if let Some(base64_data) = i.src.strip_prefix("data:image/jpeg;base64,") {
                if let Ok(bytes) = base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    base64_data,
                ) {
                    if let Some((w_px, h_px)) = jpeg_dimensions(&bytes) {
                        let img_id = b.images.len() + 1;
                        b.images.push(PdfImage {
                            id: img_id,
                            width: w_px,
                            height: h_px,
                            jpeg_data: bytes,
                        });
                        let _ = write!(
                            b.out,
                            "q\n{:.2} 0 0 {:.2} {:.2} {:.2} cm\n/Im{} Do\nQ\n",
                            w_pt, h_pt, x_pt, y_pt, img_id
                        );
                    }
                }
            } else {
                // Non-JPEG image placeholder box
                let _ = write!(
                    b.out,
                    "[] 0 d\n0.8 0.8 0.8 RG\n1.0 w\n{:.2} {:.2} {:.2} {:.2} re\nS\n",
                    x_pt, y_pt, w_pt, h_pt
                );
            }

            b.pop_rotation(i.data.rotation);
        }
        Element::Chart(c) => {
            let cx = c.data.world_point.x + c.data.width / 2.0;
            let cy = c.data.world_point.y + c.data.height / 2.0;
            b.push_rotation(cx, cy, c.data.rotation);
            b.set_opacity(c.data.style.opacity);

            let sg: ChartSceneGraph = c.scene_graph();

            // Card background
            if let Some(r) = sg.card {
                b.set_opacity(r.fill_opacity);
                if let Some(fill) = Color::try_new(&r.fill) {
                    b.set_fill_color(fill);
                }
                if let Some(stroke) = Color::try_new(&r.stroke) {
                    b.set_stroke_color(stroke);
                }
                let _ = write!(b.out, "{:.2} w\n", px_to_pt(r.stroke_width));
                b.render_rect_path(r.x, r.y, r.w, r.h, r.rx);
                let _ = write!(b.out, "B\n");
            }

            // Lines
            for l in sg.lines {
                b.set_opacity(l.opacity);
                if let Some(stroke) = Color::try_new(&l.stroke) {
                    b.set_stroke_color(stroke);
                }
                let _ = write!(b.out, "{:.2} w\n", px_to_pt(l.stroke_width));
                if l.stroke_dasharray == "6 3" {
                    let _ = write!(b.out, "[{:.2} {:.2}] 0 d\n", px_to_pt(6.0), px_to_pt(3.0));
                } else {
                    let _ = write!(b.out, "[] 0 d\n");
                }
                let _ = write!(
                    b.out,
                    "{:.2} {:.2} m\n{:.2} {:.2} l\nS\n",
                    b.map_x(l.x1),
                    b.map_y(l.y1),
                    b.map_x(l.x2),
                    b.map_y(l.y2)
                );
            }

            // Rects (bars)
            for r in sg.rects {
                b.set_opacity(r.fill_opacity);
                if let Some(fill) = Color::try_new(&r.fill) {
                    b.set_fill_color(fill);
                }
                if let Some(stroke) = Color::try_new(&r.stroke) {
                    b.set_stroke_color(stroke);
                }
                let _ = write!(b.out, "{:.2} w\n", px_to_pt(r.stroke_width));
                b.render_rect_path(r.x, r.y, r.w, r.h, r.rx);
                if r.stroke_width > 0.0 {
                    let _ = write!(b.out, "B\n");
                } else {
                    let _ = write!(b.out, "f\n");
                }
            }

            // Paths (area fills, lines, donut slices)
            for p in sg.paths {
                let has_fill = p.fill != "none" && !p.fill.is_empty();
                let has_stroke = p.stroke != "none" && !p.stroke.is_empty() && p.stroke_width > 0.0;

                if has_fill {
                    b.set_opacity(p.fill_opacity);
                    if let Some(fill) = Color::try_new(&p.fill) {
                        b.set_fill_color(fill);
                    }
                }
                if has_stroke {
                    b.set_opacity(1.0);
                    if let Some(stroke) = Color::try_new(&p.stroke) {
                        b.set_stroke_color(stroke);
                    }
                    let _ = write!(b.out, "{:.2} w\n", px_to_pt(p.stroke_width));
                    if p.stroke_dasharray == "6 3" {
                        let _ = write!(b.out, "[{:.2} {:.2}] 0 d\n", px_to_pt(6.0), px_to_pt(3.0));
                    } else {
                        let _ = write!(b.out, "[] 0 d\n");
                    }
                }

                render_svg_path(&p.d, b.view, &mut b.out);
                if has_fill && has_stroke {
                    let _ = write!(b.out, "B\n");
                } else if has_fill {
                    let _ = write!(b.out, "f\n");
                } else if has_stroke {
                    let _ = write!(b.out, "S\n");
                }
            }

            // Circles (scatter/line points, legend dots)
            for ci in sg.circles {
                if let Some(fill) = Color::try_new(&ci.fill) {
                    b.set_fill_color(fill);
                }
                let has_stroke = ci.stroke != "none" && ci.stroke_width > 0.0;
                if has_stroke {
                    if let Some(stroke) = Color::try_new(&ci.stroke) {
                        b.set_stroke_color(stroke);
                    }
                    let _ = write!(b.out, "{:.2} w\n", px_to_pt(ci.stroke_width));
                }
                b.render_ellipse_path(ci.cx, ci.cy, ci.r, ci.r);
                if has_stroke {
                    let _ = write!(b.out, "B\n");
                } else {
                    let _ = write!(b.out, "f\n");
                }
            }

            // Texts (labels, titles, values)
            for t in sg.texts {
                b.set_opacity(t.opacity);
                if let Some(color) = Color::try_new(&t.fill) {
                    b.set_fill_color(color);
                }
                let fs_pt = px_to_pt(t.font_size);
                let text_w = helvetica_text_width(&t.text, fs_pt);
                let x_pt = match t.anchor {
                    "middle" => b.map_x(t.x) - text_w / 2.0,
                    "end" => b.map_x(t.x) - text_w,
                    _ => b.map_x(t.x),
                };
                let y_pt = b.map_y(t.y);
                let font = if t.bold { "/F2" } else { "/F1" };
                let escaped = escape_pdf_text(&t.text);
                let _ = write!(
                    b.out,
                    "BT\n{} {:.2} Tf\n1 0 0 1 {:.2} {:.2} Tm\n({}) Tj\nET\n",
                    font, fs_pt, x_pt, y_pt, escaped
                );
            }

            b.pop_rotation(c.data.rotation);
        }
        Element::Frame(f) => {
            // Frame background fill (if any)
            if let Some(fill) = f.data.style.fill_color {
                b.set_opacity(f.data.style.opacity);
                b.set_fill_color(fill);
                b.render_rect_path(
                    f.data.world_point.x,
                    f.data.world_point.y,
                    f.data.width,
                    f.data.height,
                    0.0,
                );
                let _ = write!(b.out, "f\n");
            }
        }
    }
}

/// Render a set of elements and canvas background into a native vector [`PdfPage`].
pub fn render_scene_to_pdf_page(elements: &[Element], view: Rect, bg: Theme) -> PdfPage {
    let mut builder = PageBuilder::new(view);

    // 1. Canvas background
    let bg_color = Color::new(bg.canvas());
    builder.set_fill_color(bg_color);
    let w_pt = px_to_pt(view.w);
    let h_pt = px_to_pt(view.h);
    let _ = write!(builder.out, "0 0 {:.2} {:.2} re f\n", w_pt, h_pt);

    // 2. Scene elements
    for el in elements {
        render_element(el, &mut builder);
    }

    PdfPage {
        width_pt: w_pt,
        height_pt: h_pt,
        contents: builder.out,
        images: builder.images,
        opacities: builder.opacities,
    }
}

// ── PDF 1.4 Document Assembler ─────────────────────────────────────────

/// Builds a valid multi-page PDF 1.4 document from a collection of [`PdfPage`]s.
pub fn build_pdf(pages: &[PdfPage], title: &str) -> Vec<u8> {
    if pages.is_empty() {
        return Vec::new();
    }

    let mut buf = Vec::new();
    let mut offsets = Vec::new();

    // 1. PDF Header
    buf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    let num_pages = pages.len();

    // Fixed Objects:
    // 1: Catalog
    // 2: Pages root
    // 3: Font /F1 (Helvetica)
    // 4: Font /F2 (Helvetica-Bold)
    // Dynamic Objects:
    // For each page i in 0..num_pages:
    //   page_obj_id
    //   contents_obj_id
    //   image_obj_ids...

    let mut next_obj_id = 5;
    struct PageLayout {
        page_id: usize,
        contents_id: usize,
        image_ids: Vec<usize>,
    }

    let mut layouts = Vec::with_capacity(num_pages);
    for page in pages {
        let page_id = next_obj_id;
        next_obj_id += 1;
        let contents_id = next_obj_id;
        next_obj_id += 1;

        let mut image_ids = Vec::with_capacity(page.images.len());
        for _ in &page.images {
            image_ids.push(next_obj_id);
            next_obj_id += 1;
        }
        layouts.push(PageLayout {
            page_id,
            contents_id,
            image_ids,
        });
    }

    // 2. Object 1: Catalog
    offsets.push(buf.len());
    let _ = write!(
        buf,
        "1 0 obj\n<<\n  /Type /Catalog\n  /Pages 2 0 R\n>>\nendobj\n"
    );

    // 3. Object 2: Pages root
    offsets.push(buf.len());
    let mut kids = String::new();
    for layout in &layouts {
        kids.push_str(&format!("{} 0 R ", layout.page_id));
    }
    let _ = write!(
        buf,
        "2 0 obj\n<<\n  /Type /Pages\n  /Kids [{kids}]\n  /Count {num_pages}\n>>\nendobj\n"
    );

    // 4. Object 3: Font /F1 (Helvetica)
    offsets.push(buf.len());
    let _ = write!(
        buf,
        "3 0 obj\n<<\n  /Type /Font\n  /Subtype /Type1\n  /BaseFont /Helvetica\n  /Encoding /WinAnsiEncoding\n>>\nendobj\n"
    );

    // 5. Object 4: Font /F2 (Helvetica-Bold)
    offsets.push(buf.len());
    let _ = write!(
        buf,
        "4 0 obj\n<<\n  /Type /Font\n  /Subtype /Type1\n  /BaseFont /Helvetica-Bold\n  /Encoding /WinAnsiEncoding\n>>\nendobj\n"
    );

    // 6. Page objects, contents streams, and image XObjects
    for (i, page) in pages.iter().enumerate() {
        let layout = &layouts[i];

        // ExtGState dictionary entries for page opacities
        let mut ext_gstates = String::new();
        for (idx, &op) in page.opacities.iter().enumerate() {
            ext_gstates.push_str(&format!(
                "      /GS{} << /Type /ExtGState /ca {:.2} /CA {:.2} >>\n",
                idx, op, op
            ));
        }

        // XObject dictionary entries for page images
        let mut xobjects = String::new();
        for (img_idx, img) in page.images.iter().enumerate() {
            let img_obj_id = layout.image_ids[img_idx];
            xobjects.push_str(&format!("      /Im{} {} 0 R\n", img.id, img_obj_id));
        }

        // Page Object
        offsets.push(buf.len());
        let _ = write!(
            buf,
            "{} 0 obj\n<<\n  /Type /Page\n  /Parent 2 0 R\n  /MediaBox [0 0 {:.2} {:.2}]\n  /Resources <<\n    /ProcSet [/PDF /Text /ImageC]\n    /Font <<\n      /F1 3 0 R\n      /F2 4 0 R\n    >>\n    /ExtGState <<\n{}\n    >>\n    /XObject <<\n{}\n    >>\n  >>\n  /Contents {} 0 R\n>>\nendobj\n",
            layout.page_id,
            page.width_pt,
            page.height_pt,
            ext_gstates,
            xobjects,
            layout.contents_id
        );

        // Contents Object
        offsets.push(buf.len());
        let _ = write!(
            buf,
            "{} 0 obj\n<<\n  /Length {}\n>>\nstream\n",
            layout.contents_id,
            page.contents.len()
        );
        buf.extend_from_slice(&page.contents);
        buf.extend_from_slice(b"\nendstream\nendobj\n");

        // Image Objects
        for (img_idx, img) in page.images.iter().enumerate() {
            let img_obj_id = layout.image_ids[img_idx];
            offsets.push(buf.len());
            let _ = write!(
                buf,
                "{} 0 obj\n<<\n  /Type /XObject\n  /Subtype /Image\n  /Width {}\n  /Height {}\n  /ColorSpace /DeviceRGB\n  /BitsPerComponent 8\n  /Filter /DCTDecode\n  /Length {}\n>>\nstream\n",
                img_obj_id,
                img.width,
                img.height,
                img.jpeg_data.len()
            );
            buf.extend_from_slice(&img.jpeg_data);
            buf.extend_from_slice(b"\nendstream\nendobj\n");
        }
    }

    // 7. Cross-Reference Table
    let xref_offset = buf.len();
    let total_objs = next_obj_id; // 1..=total_objs-1
    let _ = write!(buf, "xref\n0 {total_objs}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = write!(buf, "{offset:010} 00000 n \n");
    }

    // 8. Trailer
    let sanitized_title = title.replace(['(', ')', '\\'], "");
    let _ = write!(
        buf,
        "trailer\n<<\n  /Size {total_objs}\n  /Root 1 0 R\n  /Info <<\n    /Title ({sanitized_title})\n    /Producer (Inskea)\n  >>\n>>\nstartxref\n{xref_offset}\n%%EOF\n"
    );

    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::elements::rect::Rectangle;
    use crate::model::elements::text::Text;
    use crate::model::{ElementData, Point};

    #[test]
    fn empty_pages_yields_empty_pdf() {
        assert!(build_pdf(&[], "Empty").is_empty());
    }

    #[test]
    fn single_page_pdf_structure() {
        let page = PdfPage {
            width_pt: 595.28,
            height_pt: 841.89,
            contents: b"0 0 595.28 841.89 re f\n".to_vec(),
            images: Vec::new(),
            opacities: vec![1.0],
        };
        let pdf = build_pdf(&[page], "Test A4");
        let s = String::from_utf8_lossy(&pdf);

        assert!(s.starts_with("%PDF-1.4"));
        assert!(s.contains("/Type /Catalog"));
        assert!(s.contains("/Type /Pages"));
        assert!(s.contains("/MediaBox [0 0 595.28 841.89]"));
        assert!(s.contains("/BaseFont /Helvetica"));
        assert!(s.contains("/Type /ExtGState"));
        assert!(s.contains("startxref"));
        assert!(s.ends_with("%%EOF\n"));
    }

    #[test]
    fn multi_page_pdf_contains_all_pages_and_correct_count() {
        let page1 = PdfPage {
            width_pt: 400.0,
            height_pt: 300.0,
            contents: b"1.0 0.0 0.0 RG 10 10 100 100 re S\n".to_vec(),
            images: Vec::new(),
            opacities: vec![1.0],
        };
        let page2 = PdfPage {
            width_pt: 500.0,
            height_pt: 600.0,
            contents: b"0.0 1.0 0.0 RG 20 20 200 200 re S\n".to_vec(),
            images: Vec::new(),
            opacities: vec![1.0, 0.5],
        };
        let pdf = build_pdf(&[page1, page2], "Gallery");
        let s = String::from_utf8_lossy(&pdf);

        assert!(s.contains("/Count 2"));
        assert!(s.contains("/MediaBox [0 0 400.00 300.00]"));
        assert!(s.contains("/MediaBox [0 0 500.00 600.00]"));
    }

    #[test]
    fn px_to_pt_conversion_ratio() {
        assert_eq!(px_to_pt(96.0), 72.0);
        assert_eq!(px_to_pt(192.0), 144.0);
        assert_eq!(px_to_pt(0.0), 0.0);
    }

    #[test]
    fn vector_pdf_renders_searchable_text_and_shapes() {
        let view = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut rect = Rectangle {
            data: ElementData::new(0),
        };
        rect.data.world_point = Point::new(50.0, 50.0);
        rect.data.width = 200.0;
        rect.data.height = 100.0;
        let mut text_data = ElementData::new(0);
        text_data.world_point = Point::new(50.0, 200.0);
        text_data.width = 200.0;
        text_data.height = 30.0;
        text_data.style.font_size = 16.0;
        text_data.style.stroke_color = Color::new("#ffffff");
        let text = Text {
            data: text_data,
            wrapped: crate::model::elements::text::WrappedText::new("Hello Inskea PDF"),
            align: crate::model::TextAlign::Left,
        };

        let elements = vec![Element::Rectangle(rect), Element::Text(text)];
        let page = render_scene_to_pdf_page(&elements, view, Theme::Storm);
        let pdf = build_pdf(&[page], "Vector Test");
        let s = String::from_utf8_lossy(&pdf);

        assert!(s.contains("Hello Inskea PDF"));
        assert!(s.contains("/F1"));
        assert!(s.contains("BT"));
        assert!(s.contains("ET"));
        assert!(s.contains("re"));
    }
}
