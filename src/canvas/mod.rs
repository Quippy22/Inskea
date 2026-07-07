mod viewport;
pub use viewport::Viewport;

use crate::model::{Element, ElementData, ElementId, Point, Scene, ShapeColor};
use crate::ui::dock::Tool;
use crate::ui::settings::{CenterStyle, GridSize, GridStyle};
use leptos::ev;
use leptos::svg::Svg;
use leptos::*;
use std::rc::Rc;
use std::sync::OnceLock;
use wasm_bindgen::JsCast;

// ── Constants ──────────────────────────────────────────────────────────────
const HIT_MARGIN: f64 = 6.0;
const CLICK_MARGIN: f64 = 12.0;
const MIN_DRAG_DIST: f64 = 3.0;
const GRID_SIZE: f64 = 40.0;
const MIN_ELEMENT_SIZE: f64 = 5.0;
const HANDLE_RESIZE_RADIUS: f64 = 5.0;
const HANDLE_MOVE_RADIUS: f64 = 6.0;
const HANDLE_ROTATE_RADIUS: f64 = 7.0;
const ROTATE_HANDLE_OFFSET: f64 = 25.0;
const ARROW_HEAD_MULT: f64 = 4.0;
const MIN_FONT_SIZE: f64 = 12.0;
const DEFAULT_FONT_SIZE: f64 = 24.0;
const TEXT_ASCENT_RATIO: f64 = 0.85;
const ZOOM_FACTOR: f64 = 1.1;
const ZOOM_MIN: f64 = 0.1;
const ZOOM_MAX: f64 = 20.0;
const DASH_PREVIEW: &str = "4 2";
const DASH_BOUNDS: &str = "3 2";
const MIN_DIMENSION: f64 = 1.0;
const SNAP_DIVISIONS: f64 = 8.0;
const ROTATE_SNAP_DIVISIONS: f64 = 24.0;
// ────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum CanvasMode {
    Select,
    Hand,
    Draw,
}

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
            if w < MIN_DIMENSION {
                w = MIN_DIMENSION;
            }
            if h < MIN_DIMENSION {
                h = MIN_DIMENSION;
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
                    (angle / (std::f64::consts::TAU / SNAP_DIVISIONS)).round() * (std::f64::consts::TAU / SNAP_DIVISIONS);
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
            if w < MIN_DIMENSION {
                w = MIN_DIMENSION;
            }
            if h < MIN_DIMENSION {
                h = MIN_DIMENSION;
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
                    (angle / (std::f64::consts::TAU / SNAP_DIVISIONS)).round() * (std::f64::consts::TAU / SNAP_DIVISIONS);
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

fn render_element(element: &Element, zoom: f64) -> leptos::View {
    match element {
        Element::Rectangle(data) => render_rect(data),
        Element::Ellipse(data) => render_ellipse(data),
        Element::Line(data, a, b) => render_line(data, a, b),
        Element::Arrow(data, a, b) => render_arrow(data, a, b),
        Element::Text(data, content) => render_text(data, content, zoom),
        Element::Freehand(data, pts) => render_freehand(data, pts),
    }
}

fn fill_paint(fill: &Option<ShapeColor>) -> &'static str {
    match fill {
        Some(_) => "currentColor",
        None => "none",
    }
}

fn stroke_hex(stroke: ShapeColor) -> &'static str {
    stroke.to_hex()
}

/// Check if a world-space point hits an element.
fn hit_test(point: (f64, f64), el: &Element) -> bool {
    let margin = HIT_MARGIN;
    let (px, py) = point;

    match el {
        Element::Rectangle(data) | Element::Ellipse(data) => {
            let has_fill = data.fill_color.is_some();
            if has_fill {
                px >= data.x - margin
                    && px <= data.x + data.width + margin
                    && py >= data.y - margin
                    && py <= data.y + data.height + margin
            } else {
                let cx = data.x + data.width / 2.0;
                let cy = data.y + data.height / 2.0;
                let hw = data.width / 2.0;
                let hh = data.height / 2.0;
                if matches!(el, Element::Rectangle(_)) {
                    let dl = (px - data.x).abs();
                    let dr = (px - (data.x + data.width)).abs();
                    let dt = (py - data.y).abs();
                    let db = (py - (data.y + data.height)).abs();
                    let near_edge = dl.min(dr).min(dt).min(db);
                    near_edge <= margin + data.stroke_width
                        && px >= data.x - margin
                        && px <= data.x + data.width + margin
                        && py >= data.y - margin
                        && py <= data.y + data.height + margin
                } else {
                    let dx = (px - cx) / hw.max(1.0);
                    let dy = (py - cy) / hh.max(1.0);
                    let dist = (dx * dx + dy * dy).sqrt();
                    let edge_dist = (dist - 1.0).abs() * hw.min(hh).max(1.0);
                    edge_dist <= margin + data.stroke_width
                }
            }
        }
        Element::Line(data, a, b) | Element::Arrow(data, a, b) => {
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            let len = (dx * dx + dy * dy).sqrt();
            if len < 1.0 {
                return (px - a.x).hypot(py - a.y) <= margin + data.stroke_width;
            }
            let t = ((px - a.x) * dx + (py - a.y) * dy) / (len * len);
            let t = t.clamp(0.0, 1.0);
            let near_x = a.x + t * dx;
            let near_y = a.y + t * dy;
            (px - near_x).hypot(py - near_y) <= margin + data.stroke_width
        }
        Element::Text(data, _) => {
            px >= data.x - margin
                && px <= data.x + data.width + margin
                && py >= data.y - margin
                && py <= data.y + data.height + margin
        }
        Element::Freehand(data, pts) => {
            if pts.is_empty() {
                return false;
            }
            let tolerance = margin + data.stroke_width;
            for p in pts {
                if (px - p.x).hypot(py - p.y) <= tolerance {
                    return true;
                }
            }
            for i in 1..pts.len() {
                let a = &pts[i - 1];
                let b = &pts[i];
                let dx = b.x - a.x;
                let dy = b.y - a.y;
                let len = (dx * dx + dy * dy).sqrt();
                if len < 1.0 {
                    continue;
                }
                let t = ((px - a.x) * dx + (py - a.y) * dy) / (len * len);
                let t = t.clamp(0.0, 1.0);
                let nx = a.x + t * dx;
                let ny = a.y + t * dy;
                if (px - nx).hypot(py - ny) <= tolerance {
                    return true;
                }
            }
            false
        }
    }
}

/// Erase the topmost element at a world-space point.
fn hit_and_erase(point: (f64, f64), scene: RwSignal<Scene>) {
    let id = scene.with(|s| {
        s.elements
            .iter()
            .rev()
            .find(|el| hit_test(point, el))
            .map(|el| el.id())
    });
    if let Some(id) = id {
        scene.update(|s| s.elements.retain(|e| e.id() != id));
    }
}

fn element_bounds(el: &Element) -> (f64, f64, f64, f64) {
    match el {
        Element::Rectangle(data) | Element::Ellipse(data) => {
            (data.x, data.y, data.width, data.height)
        }
        Element::Line(_, a, b) | Element::Arrow(_, a, b) => {
            let x = a.x.min(b.x);
            let y = a.y.min(b.y);
            let w = (b.x - a.x).abs();
            let h = (b.y - a.y).abs();
            (x, y, w, h)
        }
        Element::Text(data, _content) => {
            (data.x, data.y, data.width, data.height)
        }
        Element::Freehand(_, pts) => {
            if pts.is_empty() {
                return (0.0, 0.0, 0.0, 0.0);
            }
            let min_x = pts.iter().map(|p| p.x).reduce(f64::min).unwrap();
            let min_y = pts.iter().map(|p| p.y).reduce(f64::min).unwrap();
            let max_x = pts.iter().map(|p| p.x).reduce(f64::max).unwrap();
            let max_y = pts.iter().map(|p| p.y).reduce(f64::max).unwrap();
            (min_x, min_y, max_x - min_x, max_y - min_y)
        }
    }
}

fn rect_fully_contains_element(rx: f64, ry: f64, rw: f64, rh: f64, el: &Element) -> bool {
    let (ex, ey, ew, eh) = element_bounds(el);
    ex >= rx && ey >= ry && (ex + ew) <= (rx + rw) && (ey + eh) <= (ry + rh)
}

fn combined_bounds(ids: &[ElementId], elements: &[Element]) -> Option<(f64, f64, f64, f64)> {
    let mut out: Option<(f64, f64, f64, f64)> = None;
    for el in elements {
        if ids.contains(&el.id()) {
            let (ex, ey, ew, eh) = element_bounds(el);
            let (x1, y1, x2, y2) = (ex, ey, ex + ew, ey + eh);
            match out {
                None => out = Some((x1, y1, x2, y2)),
                Some((min_x, min_y, max_x, max_y)) => {
                    out = Some((min_x.min(x1), min_y.min(y1), max_x.max(x2), max_y.max(y2)));
                }
            }
        }
    }
    out.map(|(x1, y1, x2, y2)| (x1, y1, x2 - x1, y2 - y1))
}

fn hit_test_topmost(point: (f64, f64), elements: &[Element]) -> Option<ElementId> {
    elements
        .iter()
        .rev()
        .find(|el| hit_test(point, el))
        .map(|el| el.id())
}

fn point_inside_any_element(point: (f64, f64), elements: &[Element]) -> bool {
    let margin = CLICK_MARGIN;
    elements.iter().any(|el| {
        let (ex, ey, ew, eh) = element_bounds(el);
        let (px, py) = point;
        px >= ex - margin && px <= ex + ew + margin && py >= ey - margin && py <= ey + eh + margin
    })
}

fn offset_element(el: &mut Element, dx: f64, dy: f64) {
    match el {
        Element::Rectangle(data) | Element::Ellipse(data) | Element::Text(data, ..) => {
            data.x += dx;
            data.y += dy;
        }
        Element::Line(_, a, b) | Element::Arrow(_, a, b) => {
            a.x += dx;
            a.y += dy;
            b.x += dx;
            b.y += dy;
        }
        Element::Freehand(data, pts) => {
            data.x += dx;
            data.y += dy;
            for p in pts {
                p.x += dx;
                p.y += dy;
            }
        }
    }
}

/// Which handle a drag operation is acting on.
#[derive(Clone, Copy, PartialEq)]
enum Handle {
    /// Resize corner (0-3) or edge (4-7) handle.
    Resize(usize),
    /// Move via the centre crosshair.
    Move,
    /// Rotate via the top handle.
    Rotate,
}

/// Returns the 10 handle positions for the given bounding box.
///
/// Indices 0-7 are the resize corners/edges in order:
///   top-left, top-centre, top-right, middle-left, middle-right, bottom-left, bottom-centre, bottom-right.
/// Index 8 is the move handle (centre). Index 9 is the rotate handle (above the box).
fn handle_positions(bx: f64, by: f64, bw: f64, bh: f64) -> [(f64, f64); 10] {
    [
        (bx, by),
        (bx + bw / 2.0, by),
        (bx + bw, by),
        (bx, by + bh / 2.0),
        (bx + bw, by + bh / 2.0),
        (bx, by + bh),
        (bx + bw / 2.0, by + bh),
        (bx + bw, by + bh),
        (bx + bw / 2.0, by + bh / 2.0),
        (bx + bw / 2.0, by - ROTATE_HANDLE_OFFSET),
    ]
}

/// Snap `angle` to the nearest `divisions`-th of a full turn.
fn snap_angle(angle: f64, divisions: f64) -> f64 {
    (angle / (std::f64::consts::TAU / divisions)).round() * (std::f64::consts::TAU / divisions)
}

/// Context struct grouping all parameters for `resize_element`.
struct ResizeContext<'a> {
    orig: &'a Element,
    bx: f64,
    by: f64,
    bw: f64,
    bh: f64,
    dx: f64,
    dy: f64,
    fdx: f64,
    fdy: f64,
    handle: usize,
    shift: bool,
    multi: bool,
}

/// Resize `el` so its bounding box changes from `(bx,by,bw,bh)` by the mouse delta `(dx,dy)`.
///
/// `handle` selects which corner/edge is being dragged:
/// - 0 (top-left), 2 (top-right), 5 (bottom-left), 7 (bottom-right): resize on both axes.
/// - 1 (top-centre), 3 (middle-left), 4 (middle-right), 6 (bottom-centre): resize on one axis.
///
/// `fdx`/`fdy` is the per-frame mouse delta (for line endpoint movement).
///
/// When `shift` is held the original aspect ratio is preserved (the handle-opposite corner
/// or edge is pinned).
///
/// When `multi` is true all elements are proportionally scaled from the combined bounds;
/// when false each variant uses its own resize behaviour (e.g. lines move only the dragged endpoint).
///
/// `orig` is a clone of the element captured once at drag-start (used to prevent
/// the scale factor from compounding across frames).
fn resize_element(el: &mut Element, ctx: &ResizeContext) {
    let (mut nx, mut ny, mut nw, mut nh) = match ctx.handle {
        0 => (ctx.bx + ctx.dx, ctx.by + ctx.dy, ctx.bw - ctx.dx, ctx.bh - ctx.dy),
        1 => (ctx.bx, ctx.by + ctx.dy, ctx.bw, ctx.bh - ctx.dy),
        2 => (ctx.bx, ctx.by + ctx.dy, ctx.bw + ctx.dx, ctx.bh - ctx.dy),
        3 => (ctx.bx + ctx.dx, ctx.by, ctx.bw - ctx.dx, ctx.bh),
        4 => (ctx.bx, ctx.by, ctx.bw + ctx.dx, ctx.bh),
        5 => (ctx.bx + ctx.dx, ctx.by, ctx.bw - ctx.dx, ctx.bh + ctx.dy),
        6 => (ctx.bx, ctx.by, ctx.bw, ctx.bh + ctx.dy),
        7 => (ctx.bx, ctx.by, ctx.bw + ctx.dx, ctx.bh + ctx.dy),
        _ => return,
    };
    if ctx.shift {
        let ratio = ctx.bw / ctx.bh;
        let nratio = nw / nh;
        if nratio > ratio {
            nh = nw / ratio;
        } else {
            nw = nh * ratio;
        }
        match ctx.handle {
            0 => {
                nx = ctx.bx + ctx.bw - nw;
                ny = ctx.by + ctx.bh - nh;
            }
            1 => {
                ny = ctx.by + ctx.bh - nh;
            }
            2 => {
                ny = ctx.by + ctx.bh - nh;
            }
            3 => {
                nx = ctx.bx + ctx.bw - nw;
            }
            5 => {
                nx = ctx.bx + ctx.bw - nw;
            }
            _ => {}
        }
    }
    if nw < MIN_ELEMENT_SIZE || nh < MIN_ELEMENT_SIZE {
        return;
    }
    let obw = ctx.bw.max(MIN_ELEMENT_SIZE);
    let obh = ctx.bh.max(MIN_ELEMENT_SIZE);
    let sx = nw / obw;
    let sy = nh / obh;
    if ctx.multi {
        match (el, ctx.orig) {
            (Element::Rectangle(data) | Element::Ellipse(data),
             Element::Rectangle(od) | Element::Ellipse(od)) => {
                data.x = (od.x - ctx.bx) * sx + nx;
                data.y = (od.y - ctx.by) * sy + ny;
                data.width = (od.width * sx).max(MIN_ELEMENT_SIZE);
                data.height = (od.height * sy).max(MIN_ELEMENT_SIZE);
            }
            (Element::Line(_, a, b) | Element::Arrow(_, a, b),
             Element::Line(_, oa, ob) | Element::Arrow(_, oa, ob)) => {
                a.x = (oa.x - ctx.bx) * sx + nx;
                a.y = (oa.y - ctx.by) * sy + ny;
                b.x = (ob.x - ctx.bx) * sx + nx;
                b.y = (ob.y - ctx.by) * sy + ny;
            }
            (Element::Freehand(_, pts), Element::Freehand(_, opts)) => {
                for (p, op) in pts.iter_mut().zip(opts.iter()) {
                    p.x = (op.x - ctx.bx) * sx + nx;
                    p.y = (op.y - ctx.by) * sy + ny;
                }
            }
            (Element::Text(data, _), Element::Text(od, _)) => {
                data.x = (od.x - ctx.bx) * sx + nx;
                data.y = (od.y - ctx.by) * sy + ny;
                data.width = (od.width * sx).max(MIN_ELEMENT_SIZE);
                data.height = (od.height * sy).max(MIN_ELEMENT_SIZE);
            }
            _ => {}
        }
    } else {
        match el {
            Element::Rectangle(data) | Element::Ellipse(data) => {
                data.x = nx;
                data.y = ny;
                data.width = nw;
                data.height = nh;
            }
            Element::Line(_, a, b) | Element::Arrow(_, a, b) => {
                let hpos = handle_positions(ctx.bx, ctx.by, ctx.bw, ctx.bh);
                let (hx, hy) = hpos[ctx.handle];
                let dist_a = (a.x - hx).hypot(a.y - hy);
                let dist_b = (b.x - hx).hypot(b.y - hy);
                if dist_a < dist_b {
                    a.x += ctx.fdx;
                    a.y += ctx.fdy;
                } else {
                    b.x += ctx.fdx;
                    b.y += ctx.fdy;
                }
            }
            Element::Freehand(_, pts) => {
                for p in pts {
                    p.x = (p.x - ctx.bx) * sx + nx;
                    p.y = (p.y - ctx.by) * sy + ny;
                }
            }
            Element::Text(data, _) => {
                data.x = nx;
                data.y = ny;
                data.width = nw;
                data.height = nh;
            }
        }
    }
}

/// Snap an element's position to the nearest `grid`-sized increment.
fn snap_element_to_grid(el: &mut Element, grid: f64) {
    match el {
        Element::Rectangle(d) | Element::Ellipse(d) | Element::Text(d, _) => {
            let cx = d.x + d.width / 2.0;
            let cy = d.y + d.height / 2.0;
            let snapped_cx = (cx / grid).round() * grid;
            let snapped_cy = (cy / grid).round() * grid;
            d.x += snapped_cx - cx;
            d.y += snapped_cy - cy;
        }
        Element::Line(_, a, b) | Element::Arrow(_, a, b) => {
            a.x = (a.x / grid).round() * grid;
            a.y = (a.y / grid).round() * grid;
            b.x = (b.x / grid).round() * grid;
            b.y = (b.y / grid).round() * grid;
        }
        Element::Freehand(_, pts) => {
            for p in pts {
                p.x = (p.x / grid).round() * grid;
                p.y = (p.y / grid).round() * grid;
            }
        }
    }
}

/// Apply an incremental rotation `delta` (radians) to `el` around `(cx,cy)`.
///
/// Rectangles, ellipses and text add `delta` to `data.rotation`.
/// Lines, arrows and freehand points are rotated in-place by `delta`.
fn rotate_element(el: &mut Element, cx: f64, cy: f64, delta: f64) {
    let cos = delta.cos();
    let sin = delta.sin();
    match el {
        Element::Rectangle(data) | Element::Ellipse(data) | Element::Text(data, _) => {
            data.rotation += delta;
        }
        Element::Line(_, a, b) | Element::Arrow(_, a, b) => {
            let dx1 = a.x - cx;
            let dy1 = a.y - cy;
            let dx2 = b.x - cx;
            let dy2 = b.y - cy;
            a.x = cx + dx1 * cos - dy1 * sin;
            a.y = cy + dx1 * sin + dy1 * cos;
            b.x = cx + dx2 * cos - dy2 * sin;
            b.y = cy + dx2 * sin + dy2 * cos;
        }
        Element::Freehand(_, pts) => {
            for p in pts {
                let dx = p.x - cx;
                let dy = p.y - cy;
                p.x = cx + dx * cos - dy * sin;
                p.y = cy + dx * sin + dy * cos;
            }
        }
    }
}

fn render_rect(data: &ElementData) -> leptos::View {
    let x = data.x;
    let y = data.y;
    let w = data.width;
    let h = data.height;
    let sw = data.stroke_width;
    let fill = fill_paint(&data.fill_color);
    let stroke = stroke_hex(data.stroke_color);
    if data.rotation == 0.0 {
        view! { <rect x=x y=y width=w height=h fill=fill stroke=stroke stroke-width=sw /> }
            .into_view()
    } else {
        let cx = x + w / 2.0;
        let cy = y + h / 2.0;
        let deg = data.rotation.to_degrees();
        view! {
            <g transform={format!("rotate({} {} {})", deg, cx, cy)}>
                <rect x=x y=y width=w height=h fill=fill stroke=stroke stroke-width=sw />
            </g>
        }
        .into_view()
    }
}

fn render_ellipse(data: &ElementData) -> leptos::View {
    let x = data.x;
    let y = data.y;
    let w = data.width;
    let h = data.height;
    let sw = data.stroke_width;
    let fill = fill_paint(&data.fill_color);
    let stroke = stroke_hex(data.stroke_color);
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let rx = w / 2.0;
    let ry = h / 2.0;
    if data.rotation == 0.0 {
        view! { <ellipse cx=cx cy=cy rx=rx ry=ry fill=fill stroke=stroke stroke-width=sw /> }
            .into_view()
    } else {
        let deg = data.rotation.to_degrees();
        view! {
            <g transform={format!("rotate({} {} {})", deg, cx, cy)}>
                <ellipse cx=cx cy=cy rx=rx ry=ry fill=fill stroke=stroke stroke-width=sw />
            </g>
        }
        .into_view()
    }
}

fn render_line(data: &ElementData, a: &Point, b: &Point) -> leptos::View {
    let sw = data.stroke_width;
    let stroke = stroke_hex(data.stroke_color);
    let (x1, y1) = (a.x, a.y);
    let (x2, y2) = (b.x, b.y);
    view! { <line x1=x1 y1=y1 x2=x2 y2=y2 stroke=stroke stroke-width=sw /> }.into_view()
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
    let head_size = (sw * ARROW_HEAD_MULT).max(4.0);
    let tip_x = b.x;
    let tip_y = b.y;
    let lx = tip_x - ux * head_size - uy * head_size * 0.4;
    let ly = tip_y - uy * head_size + ux * head_size * 0.4;
    let rx = tip_x - ux * head_size + uy * head_size * 0.4;
    let ry = tip_y - uy * head_size - ux * head_size * 0.4;
    let (ax, ay) = (a.x, a.y);
    let (bx, by) = (b.x, b.y);
    let points = format!("{},{} {},{} {},{}", lx, ly, tip_x, tip_y, rx, ry);
    view! {
        <g stroke=hex stroke-width=sw fill="none" stroke-linejoin="round">
            <line x1=ax y1=ay x2=bx y2=by />
            <polyline points=points />
        </g>
    }
    .into_view()
}

fn text_ctx() -> &'static web_sys::CanvasRenderingContext2d {
    static CTX: OnceLock<web_sys::CanvasRenderingContext2d> = OnceLock::new();
    CTX.get_or_init(|| {
        let canvas = document()
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

fn render_text(data: &ElementData, content: &str, zoom: f64) -> leptos::View {
    let x = data.x;
    let font_size = data.font_size.max(MIN_FONT_SIZE);
    let baseline = data.y + font_size * TEXT_ASCENT_RATIO;
    let fill = data
        .fill_color
        .map(|c| c.to_hex())
        .unwrap_or_else(|| data.stroke_color.to_hex());
    let lines = wrap_lines(content, data.width, font_size, zoom);
    let inner = if lines.len() <= 1 {
        view! {
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
        view! {
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
                        view! {
                            <tspan x={x.to_string()} dy=dy>
                                {line.to_string()}
                            </tspan>
                        }
                    })
                    .collect_view()}
            </text>
        }
            .into_view()
    };
    if data.rotation == 0.0 {
        inner
    } else {
        let cx = x + data.width / 2.0;
        let cy = data.y + data.height / 2.0;
        let deg = data.rotation.to_degrees();
        view! { <g transform={format!("rotate({} {} {})", deg, cx, cy)}>{inner}</g> }.into_view()
    }
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
    view! { <path d=d fill="none" stroke=stroke stroke-width=sw /> }.into_view()
}

#[component]
pub fn Canvas(
    cursor_screen: RwSignal<(f64, f64)>,
    cursor_world: RwSignal<(f64, f64)>,
    viewport: RwSignal<Viewport>,
    selected_tool: RwSignal<Tool>,
    selected_color: RwSignal<ShapeColor>,
    canvas_mode: RwSignal<CanvasMode>,
    scene: RwSignal<Scene>,
    eraser_active: RwSignal<bool>,
    center_style: RwSignal<CenterStyle>,
    grid_style: RwSignal<GridStyle>,
    grid_size: RwSignal<GridSize>,
    push_snapshot: Rc<dyn Fn()>,
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
    let shift_pressed = create_rw_signal(false);
    let pan_anchor = create_rw_signal(None::<(f64, f64)>);
    let select_anchor = create_rw_signal(None::<(f64, f64)>);
    let erasing = create_rw_signal(false);
    let selected_ids = create_rw_signal(Vec::<ElementId>::new());
    let moving_anchor = create_rw_signal(None::<(f64, f64)>); // world position when a drag started
    let drag_action = create_rw_signal(None::<Handle>); // which handle is being dragged
    let drag_bounds = create_rw_signal(None::<(f64, f64, f64, f64)>); // initial combined bounds at drag start
    let drag_angle = create_rw_signal(None::<f64>); // initial mouse angle from centre (for rotation)
    let last_world = create_rw_signal(None::<(f64, f64)>); // world position on the previous pointer-move (for per-frame deltas)
    let drag_originals = create_rw_signal(Vec::<Element>::new()); // clones of selected elements at drag start

    // ── Text editing ─────────────────────────────────────────────────────
    let editing_id = create_rw_signal(None::<ElementId>);
    let edit_text = create_rw_signal(String::new());
    let textarea_ref = create_node_ref::<leptos::html::Textarea>();

    let commit_edit = Rc::new(move || {
        if let Some(id) = editing_id.get() {
            let text = edit_text.get();
            scene.update(|s| {
                if text.is_empty() {
                    s.elements.retain(|e| e.id() != id);
                } else if let Some(el) = s.elements.iter_mut().find(|e| e.id() == id) {
                    if let Element::Text(data, ref mut content) = el {
                        *content = text;
                        if let Some(ta) = textarea_ref.get() {
                            let cw = ta.client_width() as f64;
                            let sh = ta.scroll_height() as f64;
                            let zoom = viewport.get().zoom;
                            if zoom > 0.0 {
                                data.width = cw / zoom;
                                data.height = sh / zoom;
                            }
                        }
                    }
                }
            });
            editing_id.set(None);
            edit_text.set(String::new());
        }
    });

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
        let mode = canvas_mode.get();
        let world = update_world(&ev);

        if eraser_active.get() && erasing.get() {
            hit_and_erase(world, scene);
        }

        match mode {
            CanvasMode::Hand => {
                if let Some((ax, ay)) = pan_anchor.get() {
                    let dx = ev.client_x() as f64 - ax;
                    let dy = ev.client_y() as f64 - ay;
                    viewport.update(|vp| {
                        vp.offset_x -= dx / vp.zoom;
                        vp.offset_y -= dy / vp.zoom;
                    });
                    pan_anchor.set(Some((ev.client_x() as f64, ev.client_y() as f64)));
                }
            }
            CanvasMode::Select => {
                if let Some(anchor) = moving_anchor.get() {
                    let dx = world.0 - anchor.0;
                    let dy = world.1 - anchor.1;
                    let ids = selected_ids.get();
                    match drag_action.get() {
                        Some(Handle::Resize(idx)) => {
                            if let Some((bx, by, bw, bh)) = drag_bounds.get() {
                                let frame_dx = world.0 - last_world.get().unwrap_or(world).0;
                                let frame_dy = world.1 - last_world.get().unwrap_or(world).1;
                                last_world.set(Some(world));
                                let multi = ids.len() > 1;
                                let originals = drag_originals.get();
                                scene.update(|s| {
                                    for el in s.elements.iter_mut() {
                                        if ids.contains(&el.id()) {
                                            if let Some(orig) = originals.iter().find(|o| o.id() == el.id()) {
                                                let ctx = ResizeContext {
                                                    orig,
                                                    bx, by, bw, bh,
                                                    dx, dy,
                                                    fdx: frame_dx, fdy: frame_dy,
                                                    handle: idx,
                                                    shift: shift_pressed.get(),
                                                    multi,
                                                };
                                                resize_element(el, &ctx);
                                            }
                                        }
                                    }
                                });
                            }
                        }
                        Some(Handle::Rotate) => {
                            if let Some((bx, by, bw, bh)) = drag_bounds.get() {
                                let cx = bx + bw / 2.0;
                                let cy = by + bh / 2.0;
                                if let Some(prev) = drag_angle.get() {
                                    let mut cur = (world.1 - cy).atan2(world.0 - cx);
                                    if shift_pressed.get() {
                                        cur = snap_angle(cur, ROTATE_SNAP_DIVISIONS);
                                    }
                                    let delta = cur - prev;
                                    drag_angle.set(Some(cur));
                                    scene.update(|s| {
                                        for el in s.elements.iter_mut() {
                                            if ids.contains(&el.id()) {
                                                rotate_element(el, cx, cy, delta);
                                            }
                                        }
                                    });
                                }
                            }
                        }
                        _ => {
                            scene.update(|s| {
                                for el in s.elements.iter_mut() {
                                    if ids.contains(&el.id()) {
                                        offset_element(el, dx, dy);
                                    }
                                }
                            });
                            moving_anchor.set(Some(world));
                        }
                    }
                }
            }
            CanvasMode::Draw => {
                if let Some(ref state) = drawing.get() {
                    if state.tool == Tool::Freehand {
                        scene.update(|s| {
                            if let Some(el) = s.elements.last_mut() {
                                update_drawing(el, world, state.anchor, ev.shift_key());
                            }
                        });
                    }
                }
            }
        }
    };

    let on_wheel = move |ev: ev::WheelEvent| {
        ev.prevent_default();
        let screen = cursor_screen.get();
        let (sw, sh) = screen_size.get();
        let factor = if ev.delta_y() < 0.0 { ZOOM_FACTOR } else { 1.0 / ZOOM_FACTOR };

        viewport.update(|vp| {
            let world = vp.screen_to_world(screen, (sw, sh));
            vp.zoom = (vp.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
            vp.offset_x = world.0 - (screen.0 - sw / 2.0) / vp.zoom;
            vp.offset_y = world.1 - (screen.1 - sh / 2.0) / vp.zoom;
        });
    };

    let view_box = move || {
        let (w, h) = screen_size.get();
        viewport.get().to_view_box(w, h)
    };

    let ps_down = push_snapshot.clone();
    let on_pointer_down = move |ev: ev::PointerEvent| {
        // If a text edit is active, let the blur handler commit and bail
        if editing_id.get().is_some() {
            return;
        }
        let mode = canvas_mode.get();
        let world = update_world(&ev);

        if eraser_active.get() {
            ps_down();
            erasing.set(true);
            let world = update_world(&ev);
            hit_and_erase(world, scene);
            return;
        }

        match mode {
            CanvasMode::Hand => {
                pan_anchor.set(Some((ev.client_x() as f64, ev.client_y() as f64)));
            }
            CanvasMode::Select => {
                // Double-click on existing text → edit
                if ev.detail() >= 2 {
                    let els = scene.get().elements;
                    if let Some(id) = hit_test_topmost(world, &els) {
                        if let Some(Element::Text(_, content)) = els.iter().find(|e| e.id() == id) {
                            editing_id.set(Some(id));
                            edit_text.set(content.clone());
                            return;
                        }
                    }
                }

                let ids = selected_ids.get();
                let els = scene.get().elements;

                if !ids.is_empty() {
                    if let Some(bounds @ (bx, by, bw, bh)) = combined_bounds(&ids, &els) {
                        let hpos = handle_positions(bx, by, bw, bh); // all 10 handle positions
                                                                     // resize handles (indices 0–7)
                        for (i, &(hx, hy)) in hpos[..8].iter().enumerate() {
                            if ((world.0 - hx).powi(2) + (world.1 - hy).powi(2)).sqrt() <= HANDLE_RESIZE_RADIUS {
                                ps_down();
                                drag_action.set(Some(Handle::Resize(i)));
                                moving_anchor.set(Some(world));
                                drag_bounds.set(Some(bounds));
                                last_world.set(Some(world));
                                drag_originals.set(
                                    els.iter()
                                        .filter(|el| ids.contains(&el.id()))
                                        .cloned()
                                        .collect(),
                                );
                                return;
                            }
                        }
                        // move handle (index 8, centre)
                        let (hx, hy) = hpos[8];
                        if ((world.0 - hx).powi(2) + (world.1 - hy).powi(2)).sqrt() <= HANDLE_MOVE_RADIUS {
                            ps_down();
                            drag_action.set(Some(Handle::Move));
                            moving_anchor.set(Some(world));
                            drag_bounds.set(Some(bounds));
                            last_world.set(Some(world));
                            return;
                        }
                        // rotate handle (index 9, above the box)
                        let (hx, hy) = hpos[9];
                        if ((world.0 - hx).powi(2) + (world.1 - hy).powi(2)).sqrt() <= HANDLE_ROTATE_RADIUS {
                            ps_down();
                            let cx = bx + bw / 2.0;
                            let cy = by + bh / 2.0;
                            drag_action.set(Some(Handle::Rotate));
                            drag_angle.set(Some((world.1 - cy).atan2(world.0 - cx)));
                            moving_anchor.set(Some(world));
                            drag_bounds.set(Some(bounds));
                            return;
                        }
                    }
                }

                if let Some(id) = hit_test_topmost(world, &els) {
                    if shift_pressed.get() {
                        let mut ids = selected_ids.get();
                        if let Some(pos) = ids.iter().position(|&x| x == id) {
                            ids.remove(pos);
                        } else {
                            ids.push(id);
                        }
                        selected_ids.set(ids);
                    } else {
                        selected_ids.set(vec![id]);
                    }
                    return;
                }

                if point_inside_any_element(world, &els) {
                    if !shift_pressed.get() {
                        selected_ids.set(Vec::new());
                    }
                    return;
                }

                selected_ids.set(Vec::new());
                select_anchor.set(Some(world));
            }
            CanvasMode::Draw => {
                let tool = selected_tool.get();
                let color = selected_color.get();

                if tool == Tool::Text {
                    ps_down();
                    let mut data = ElementData::new(0);
                    data.x = world.0;
                    data.y = world.1;
                    data.font_size = DEFAULT_FONT_SIZE;
                    data.width = 0.0;
                    data.height = 0.0;
                    data.stroke_color = color;
                    let id = scene.with(|s| s.next_id);
                    scene.update(|s| {
                        s.add_element(Element::Text(data, String::new()));
                    });
                    editing_id.set(Some(id));
                    edit_text.set(String::new());
                    return;
                }

                if tool == Tool::Freehand {
                    ps_down();
                    scene.update(|s| {
                        let mut data = ElementData::new(0);
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

                ps_down();
                drawing.set(Some(DrawingState {
                    anchor: world,
                    tool,
                    color,
                }));
            }
        }
    };

    let ps_up = push_snapshot.clone();
    let on_pointer_up = move |ev: ev::PointerEvent| {
        if eraser_active.get() {
            let world = update_world(&ev);
            hit_and_erase(world, scene);
        }
        erasing.set(false);
        match canvas_mode.get() {
            CanvasMode::Hand => {
                pan_anchor.set(None);
            }
            CanvasMode::Select => {
                if moving_anchor.get().is_some() {
                    if shift_pressed.get() {
                        let ids = selected_ids.get();
                        scene.update(|s| {
                            for el in s.elements.iter_mut() {
                                if ids.contains(&el.id()) {
                                    snap_element_to_grid(el, GRID_SIZE);
                                }
                            }
                        });
                    }
                    moving_anchor.set(None);
                    drag_action.set(None);
                    drag_bounds.set(None);
                    drag_angle.set(None);
                    last_world.set(None);
                    drag_originals.set(Vec::new());
                    select_anchor.set(None);
                    return;
                }

                if let Some(anchor) = select_anchor.get() {
                    let world = cursor_world.get();
                    let dx = world.0 - anchor.0;
                    let dy = world.1 - anchor.1;
                    if dx.hypot(dy) >= MIN_DRAG_DIST {
                        let rx = anchor.0.min(world.0);
                        let ry = anchor.1.min(world.1);
                        let rw = (world.0 - anchor.0).abs();
                        let rh = (world.1 - anchor.1).abs();
                        let els = scene.get().elements;
                        let contained: Vec<ElementId> = els
                            .iter()
                            .filter(|el| rect_fully_contains_element(rx, ry, rw, rh, el))
                            .map(|el| el.id())
                            .collect();
                        selected_ids.set(contained);
                    }
                    select_anchor.set(None);
                }
            }
            CanvasMode::Draw => {
                if let Some(state) = drawing.get() {
                    if state.tool == Tool::Freehand {
                        drawing.set(None);
                        return;
                    }

                    let world = update_world(&ev);
                    let dx = world.0 - state.anchor.0;
                    let dy = world.1 - state.anchor.1;
                    if dx.hypot(dy) < MIN_DRAG_DIST {
                        drawing.set(None);
                        return;
                    }
                    let el = build_element(
                        state.anchor,
                        world,
                        state.tool,
                        state.color,
                        shift_pressed.get(),
                    );
                    ps_up();
                    scene.update(|s| {
                        s.add_element(el);
                    });
                    drawing.set(None);
                }
            }
        }
    };

    let drawing_preview = move || {
        if canvas_mode.get() != CanvasMode::Draw {
            return None;
        }
        let state = drawing.get()?;
        if state.tool == Tool::Freehand {
            return None;
        }
        let world = cursor_world.get();
        let shift = shift_pressed.get();
        let dx = world.0 - state.anchor.0;
        let dy = world.1 - state.anchor.1;
        if dx.hypot(dy) < MIN_DRAG_DIST {
            return None;
        }
        let el = build_element(state.anchor, world, state.tool, state.color, shift);
        Some(view! { <g stroke-dasharray={DASH_PREVIEW}>{render_element(&el, viewport.get().zoom)}</g> }.into_view())
    };

    let selection_preview = move || {
        let anchor = select_anchor.get()?;
        let world = cursor_world.get();
        let x = anchor.0.min(world.0);
        let y = anchor.1.min(world.1);
        let w = (world.0 - anchor.0).abs();
        let h = (world.1 - anchor.1).abs();
        if w < 1.0 || h < 1.0 {
            return None;
        }
        let hex = ShapeColor::Blue.to_hex();
        Some(
            view! {
                <rect
                    x=x
                    y=y
                    width=w
                    height=h
                    fill=format!("{}33", hex)
                    stroke=hex
                    stroke-width="1"
                    stroke-dasharray={DASH_PREVIEW}
                    pointer-events="none"
                />
            }
            .into_view(),
        )
    };

    view! {
        <svg
            _ref=svg_ref
            width="100%"
            height="100%"
            style="display: block; user-select: none; -webkit-user-select: none;"
            viewBox=view_box
            on:pointerdown=on_pointer_down
            on:pointermove=on_pointer_move
            on:pointerup=on_pointer_up
            on:wheel=on_wheel
        >
            {move || {
                let gs = grid_style.get();
                let sz = grid_size.get().px();
                let half = sz / 2.0;

                let pattern = match gs {
                    GridStyle::Dot => {
                        view! {
                            <pattern
                                id="grid-dot"
                                width={sz.to_string()}
                                height={sz.to_string()}
                                patternUnits="userSpaceOnUse"
                                patternTransform={format!("translate({}, {})", -half, -half)}
                            >
                                <circle cx={half.to_string()} cy={half.to_string()} r="1.5" fill="#d1d5db" fill-opacity="0.25" />
                            </pattern>
                        }
                            .into_view()
                    }
                    GridStyle::Line => {
                        view! {
                            <pattern
                                id="grid-line"
                                width={sz.to_string()}
                                height={sz.to_string()}
                                patternUnits="userSpaceOnUse"
                            >
                                <path
                                    d={format!("M {} 0 L 0 0 0 {}", sz, sz)}
                                    fill="none"
                                    stroke="#d1d5db"
                                    stroke-opacity="0.25"
                                    stroke-width="1"
                                />
                            </pattern>
                        }
                            .into_view()
                    }
                    GridStyle::Off => view! {}.into_view(),
                };

                let rect = match gs {
                    GridStyle::Off => view! {}.into_view(),
                    _ => {
                        let fill_id = match gs {
                            GridStyle::Dot => "url(#grid-dot)",
                            GridStyle::Line => "url(#grid-line)",
                            _ => "",
                        };
                        view! {
                            <rect
                                x="-100000"
                                y="-100000"
                                width="200000"
                                height="200000"
                                fill=fill_id
                            />
                        }
                            .into_view()
                    }
                };

                let center = match center_style.get() {
                    CenterStyle::Crosshair => {
                        view! {
                            <path d="M-12,0 L12,0 M0,-12 L0,12" stroke="#7aa2f7" stroke-width="2" />
                        }
                            .into_view()
                    }
                    CenterStyle::Dot => {
                        view! {
                            <circle cx="0" cy="0" r="3" fill="#7aa2f7" />
                        }
                            .into_view()
                    }
                    CenterStyle::Off => view! {}.into_view(),
                };

                view! {
                    <defs>{pattern}</defs>
                    {rect}
                    {center}
                }
                    .into_view()
            }}

            {move || {
                scene
                    .get()
                    .elements
                    .iter()
                    .map(|el| {
                        let zoom = viewport.get().zoom;
                        let view = render_element(el, zoom);
                        view! { <g pointer-events="none">{view}</g> }.into_view()
                    })
                    .collect_view()
            }}

            {move || drawing_preview()}

            {move || selection_preview()}

            {move || {
                let ids = selected_ids.get();
                if ids.is_empty() {
                    return None;
                }
                let els = scene.get().elements;
                let (bx, by, bw, bh) = if ids.len() == 1 {
                    els.iter()
                        .find(|el| el.id() == ids[0])
                        .map(|el| match el {
                            Element::Rectangle(d) | Element::Ellipse(d) | Element::Text(d, _) => {
                                (d.x, d.y, d.width, d.height)
                            }
                            Element::Line(_, a, b) | Element::Arrow(_, a, b) => {
                                (a.x.min(b.x), a.y.min(b.y), (a.x - b.x).abs(), (a.y - b.y).abs())
                            }
                            Element::Freehand(..) => {
                                // For freehand, we might have to use bounds. For simplicity, just use combined_bounds
                                combined_bounds(&ids, &els).unwrap_or((0.0, 0.0, 0.0, 0.0))
                            }
                        })
                        .unwrap_or_else(|| combined_bounds(&ids, &els).unwrap_or((0.0, 0.0, 0.0, 0.0)))
                } else {
                    combined_bounds(&ids, &els).unwrap_or((0.0, 0.0, 0.0, 0.0))
                };
                let hex = ShapeColor::Blue.to_hex();
                let hr = 5.0;
                let cx = bx + bw / 2.0;
                let cy = by + bh / 2.0;

                // determine rotation angle for a single-element selection
                let rot: f64 = (ids.len() == 1)
                    .then(|| {
                        els.iter().find(|el| el.id() == ids[0]).and_then(|el| match el {
                            Element::Rectangle(d) | Element::Ellipse(d)
                            | Element::Text(d, _) if d.rotation != 0.0 => Some(d.rotation),
                            _ => None,
                        })
                    })
                    .flatten()
                    .unwrap_or(0.0);
                
                // Vector from center to handle: (0, - (bh / 2.0 + 25.0))
                let handle_vec_x = 0.0;
                let handle_vec_y = -(bh / 2.0 + ROTATE_HANDLE_OFFSET);
                
                // Rotate vector
                let rx = cx + handle_vec_x * rot.cos() - handle_vec_y * rot.sin();
                let ry = cy + handle_vec_x * rot.sin() + handle_vec_y * rot.cos();


                let inner = {
                    let corners = [
                        (bx, by),
                        (bx + bw / 2.0, by),
                        (bx + bw, by),
                        (bx, by + bh / 2.0),
                        (bx + bw, by + bh / 2.0),
                        (bx, by + bh),
                        (bx + bw / 2.0, by + bh),
                        (bx + bw, by + bh),
                    ];
                    view! {
                        <rect x=bx y=by width=bw height=bh fill="none"
                            stroke=hex stroke-width="1"
                            stroke-dasharray={DASH_BOUNDS} pointer-events="none" />
                        <line x1=cx y1=by x2=rx y2=ry
                            stroke=hex stroke-width="1" pointer-events="none" />
                        {corners.iter().map(|&(hx, hy)| {
                            view! {
                                <circle cx=hx cy=hy r=hr fill="white" stroke=hex
                                    stroke-width="1.5" pointer-events="none" />
                            }.into_view()
                        }).collect_view()}
                    }
                    .into_view()
                };

                let icons = view! {
                    <g stroke=hex stroke-width="1.5" fill="none"
                        transform=format!("translate({} {}) scale(0.75)", cx - 9.0, cy - 9.0)
                        pointer-events="none">
                        <circle cx="12" cy="12" r="9.25" fill="white" stroke=hex stroke-width="1.5" />
                        <path d="M12 3 L12 21 M3 12 L21 12" />
                        <path d="M9 6 L12 3 L15 6" />
                        <path d="M9 18 L12 21 L15 18" />
                        <path d="M6 9 L3 12 L6 15" />
                        <path d="M18 9 L21 12 L18 15" />
                    </g>
                    <g stroke=hex stroke-width="1.5" fill="none"
                        transform=format!("translate({} {}) scale(0.75)", rx - 9.0, ry - 9.0)
                        pointer-events="none">
                        <circle cx="12" cy="12" r="9.25" fill="white" stroke=hex stroke-width="1.5" />
                        <path d="M12 4 A8 8 0 1 1 4 12" />
                        <path d="M1.8 11.6 L4 9 L6.2 11.6" />
                    </g>
                };

                Some(
                    if rot != 0.0 {
                        let deg = rot.to_degrees();
                        view! {
                            <g transform={format!("rotate({} {} {})", deg, cx, cy)}>
                                {inner}
                                {icons}
                            </g>
                        }.into_view()
                    } else {
                        view! {
                            {inner}
                            {icons}
                        }.into_view()
                    }
                )
            }}
        </svg>

        {move || {
            let id = editing_id.get()?;
            let (data, _content) = scene.with(|s| {
                s.elements.iter().find(|e| e.id() == id).and_then(|e| {
                    if let Element::Text(d, c) = e {
                        Some((d.clone(), c.clone()))
                    } else {
                        None
                    }
                })
            })?;
            let zoom = viewport.get().zoom;
            let font_size = data.font_size.max(MIN_FONT_SIZE);
            let (sw, sh) = screen_size.get();
            let (sx, sy) = viewport.get().world_to_screen((data.x, data.y), (sw, sh));
            let fill = data
                .fill_color
                .map(|c| c.to_hex())
                .unwrap_or_else(|| data.stroke_color.to_hex());

            // initial textarea size: use stored bounds, or default (~30 chars)
            let default_ta_w = (30.0_f64 * font_size * 0.6).max(120.0);
            let ta_w = if data.width > 0.0 {
                (data.width * zoom).max(default_ta_w)
            } else {
                default_ta_w
            };
            let ta_h = if data.height > 0.0 {
                (data.height * zoom).max(font_size * zoom * 1.2)
            } else {
                (font_size * 1.2).max(50.0)
            };

            let ce_blur = commit_edit.clone();
            let ce_esc = commit_edit.clone();

            Some(
                view! {
                    <textarea
                        autofocus="true"
                        _ref=textarea_ref
                        style={format!(
                            "position:fixed;left:{}px;top:{}px;\
                             width:{}px;height:{}px;\
                             font-size:{}px;font-family:sans-serif;\
                             color:{};\
                             background:rgba(122,162,247,0.05);\
                             border:1px dashed {};\
                             outline:none;resize:none;\
                             overflow:hidden;\
                             white-space:pre-wrap;overflow-wrap:break-word;\
                             padding:0;margin:0;z-index:100;\
                             line-height:1.2;",
                            sx, sy, ta_w, ta_h, font_size * zoom, fill, fill
                        )}
                        prop:value=move || edit_text.get()
                        on:input=move |ev| {
                            edit_text.set(event_target_value(&ev));
                            let ta = event_target::<web_sys::HtmlTextAreaElement>(&ev);
                            ta.style().set_property("height", "auto").ok();
                            ta.style().set_property("height", &format!("{}px", ta.scroll_height())).ok();
                        }
                        on:blur=move |_| ce_blur()
                        on:keydown=move |ev: ev::KeyboardEvent| {
                            if ev.key() == "Escape" {
                                ce_esc();
                            }
                            if ev.key() == "Tab" {
                                ev.prevent_default();
                                let target = event_target::<web_sys::HtmlTextAreaElement>(&ev);
                                let cursor = target
                                    .selection_start()
                                    .ok()
                                    .flatten()
                                    .unwrap_or(0) as usize;
                                let mut val = edit_text.get();
                                val.insert(cursor, '\t');
                                edit_text.set(val);
                                let pos = (cursor + 1) as u32;
                                let _ = target.set_selection_start(Some(pos));
                                let _ = target.set_selection_end(Some(pos));
                            }
                        }
                    ></textarea>
                },
            )
        }}
    }
}
