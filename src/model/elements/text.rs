use crate::model::elements::utils::rotate_bbox;
use crate::model::resize::{
    resize_bbox, resize_scale_element, rotate_point_around, ResizeContext, ResizeHandle,
    MIN_ELEMENT_SIZE,
};
use crate::model::*;
use leptos::IntoView;

pub(crate) const MIN_FONT_SIZE: f64 = 12.0;
pub(crate) const TEXT_ASCENT_RATIO: f64 = 0.85;
/// Font family used for all text rendering and editing.
pub(crate) const FONT_FAMILY: &str = "Cabin";

/// Font stack for rendered and exported text: the bundled font when it is
/// available, and the generic sans-serif fallback otherwise. `@font-face`
/// registration uses `FONT_FAMILY` alone.
pub(crate) const FONT_STACK: &str = "Cabin, sans-serif";

/// Horizontal alignment of the text within its box. Left is the default;
/// Center and Right only show a difference once the box is wider than a line
/// (i.e. after a side-resize wraps written text into a column).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl TextAlign {
    /// SVG `text-anchor` value for this alignment.
    pub fn anchor(&self) -> &'static str {
        match self {
            TextAlign::Left => "start",
            TextAlign::Center => "middle",
            TextAlign::Right => "end",
        }
    }

    /// CSS `text-align` value for this alignment.
    pub fn css(&self) -> &'static str {
        match self {
            TextAlign::Left => "left",
            TextAlign::Center => "center",
            TextAlign::Right => "right",
        }
    }
}

/// Text content plus the lines it is displayed as.
///
/// `raw` holds the actual content (including any hard `\n` breaks the user
/// typed). `lines` holds what is rendered as one `<tspan>` per entry: the hard
/// breaks, plus any soft breaks introduced by side-resize word wrapping.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WrappedText {
    /// Raw text with hard line breaks (from user Enter presses).
    pub raw: String,
    /// Display lines: hard breaks plus soft word-wrap breaks, one per `<tspan>`.
    pub lines: Vec<String>,
}

impl WrappedText {
    /// Build a new `WrappedText` from raw content.
    pub fn new(raw: &str) -> Self {
        WrappedText {
            raw: raw.to_string(),
            lines: Self::split_lines(raw),
        }
    }

    /// Replace the raw content and re-split the lines.
    pub fn set_raw(&mut self, raw: &str) {
        self.raw = raw.to_string();
        self.lines = Self::split_lines(raw);
    }

    fn split_lines(raw: &str) -> Vec<String> {
        raw.split('\n').map(|s| s.to_string()).collect()
    }

    /// Recompute the display lines from `raw`. With a positive `max_width`
    /// (world units) whole words are pushed to a new soft line when they do
    /// not fit; hard `\n` breaks are always respected. A `max_width <= 0`
    /// disables wrapping and leaves one line per hard break.
    pub fn rebuild(&mut self, max_width: f64, font_size: f64, font_family: &str) {
        if max_width <= 0.0 {
            self.lines = Self::split_lines(&self.raw);
        } else {
            self.lines = Self::split_lines(&self.raw)
                .into_iter()
                .flat_map(|line| soft_wrap_line(&line, max_width, font_size, font_family))
                .collect();
        }
    }
}

/// Width (world units) of the widest single word in `raw` at `font_size`.
pub fn longest_word_width(raw: &str, font_size: f64, font_family: &str) -> f64 {
    raw.split_whitespace()
        .map(|w| measure_text_width(w, font_size, font_family))
        .fold(0.0, f64::max)
}

/// Width (world units) of the widest hard line in `raw` at `font_size`,
/// i.e. the natural (unwrapped) width of the text.
pub(crate) fn natural_line_width(raw: &str, font_size: f64, font_family: &str) -> f64 {
    raw.split('\n')
        .map(|l| measure_text_width(l, font_size, font_family))
        .fold(0.0, f64::max)
}

/// Greedy word wrapping. Words are moved whole onto the next soft line when
/// they do not fit, so a word is never split. Empty input yields one empty line.
fn soft_wrap_line(line: &str, max_width: f64, font_size: f64, font_family: &str) -> Vec<String> {
    let space_w = measure_text_width(" ", font_size, font_family);
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut cur_w = 0.0;
    for word in line.split_whitespace() {
        let word_w = measure_text_width(word, font_size, font_family);
        let gap = if cur.is_empty() { 0.0 } else { space_w };
        if cur_w + gap + word_w <= max_width || cur.is_empty() {
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(word);
            cur_w += gap + word_w;
        } else {
            out.push(std::mem::take(&mut cur));
            cur.push_str(word);
            cur_w = word_w;
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

/// Measure rendered text width (world units) at `font_size`.
///
/// Under wasm this measures against `canvas 2d` with the real font, which
/// matches the SVG rendering. Natively (unit tests) there is no DOM, so a
/// deterministic per-char heuristic is used instead.
fn measure_text_width(text: &str, font_size: f64, font_family: &str) -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(ctx) = canvas_context() {
            ctx.set_font(&format!("{font_size}px {font_family}, sans-serif"));
            if let Ok(metrics) = ctx.measure_text(text) {
                return metrics.width();
            }
        }
    }
    let _ = font_family;
    text.chars().count() as f64 * font_size * 0.55
}

/// Tight glyph metrics (world units) of `text` at `font_size`: rendered width,
/// glyph ascent above the baseline, and glyph descent below it.
///
/// Under wasm the actual bounding box reported by `canvas 2d` matches the SVG
/// rendering exactly, so the selection box hugs the true glyphs instead of a
/// hard-coded fraction of the font size (which drifts off for fonts whose
/// ascender/descender differ from the guess). Natively a heuristic is used.
fn measure_text_extent(text: &str, font_size: f64, font_family: &str) -> (f64, f64, f64) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(ctx) = canvas_context() {
            ctx.set_font(&format!("{font_size}px {font_family}, sans-serif"));
            if let Ok(metrics) = ctx.measure_text(text) {
                let asc = metrics.actual_bounding_box_ascent();
                let desc = metrics.actual_bounding_box_descent();
                return (metrics.width(), asc, desc);
            }
        }
    }
    let _ = font_family;
    (
        text.chars().count() as f64 * font_size * 0.55,
        font_size * 0.85,
        font_size * 0.25,
    )
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// Scratch 2D context reused for every glyph measurement. Creating a
    /// fresh `<canvas>` per measurement allocated a DOM element on every
    /// text render and hit test.
    static MEASURE_CTX: std::cell::OnceCell<web_sys::CanvasRenderingContext2d> =
        const { std::cell::OnceCell::new() };
}

#[cfg(target_arch = "wasm32")]
fn canvas_context() -> Option<web_sys::CanvasRenderingContext2d> {
    use wasm_bindgen::JsCast;
    MEASURE_CTX.with(|cell| {
        cell.get()
            .or_else(|| {
                let document = web_sys::window()?.document()?;
                let canvas = document.create_element("canvas").ok()?;
                let ctx = canvas
                    .dyn_into::<web_sys::HtmlCanvasElement>()
                    .ok()?
                    .get_context("2d")
                    .ok()??
                    .dyn_into::<web_sys::CanvasRenderingContext2d>()
                    .ok()?;
                cell.set(ctx).ok()?;
                cell.get()
            })
            .cloned()
    })
}

// ── Text element ───────────────────────────────────────────────────────

/// A piece of text rendered as SVG.
///
/// `data.width` is the wrap width: side (E/W) resizing soft-wraps whole words
/// into it (never narrower than the longest word), while corner/down resizing
/// scales the whole box by changing the font size. `data.height` follows the
/// wrapped line count.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Text {
    /// Position, font size, and fill colour.
    pub data: ElementData,
    /// Text content split on hard line breaks.
    pub wrapped: WrappedText,
    /// Horizontal alignment of the lines within the box.
    #[serde(default)]
    pub align: TextAlign,
}

impl Text {
    /// The wrap column the text sits in: `data.width`, or the text's own
    /// width when it is a fresh, unwrapped (yet unsized) box. Alignment
    /// positions each line inside this column.
    pub fn column_width(&self) -> f64 {
        self.data.width.max(0.0)
    }
}

impl Text {
    /// Recompute the display lines (soft-wrapping whole words to fit
    /// `data.width`) and derive `data.height` from the line count.
    pub(crate) fn recalc(&mut self) {
        let fs = self.data.style.font_size.max(MIN_FONT_SIZE);
        let ff = FONT_FAMILY;
        self.wrapped.rebuild(self.data.width, fs, ff);
        self.recalc_height();
    }

    /// Derive `data.height` from the existing display lines. Used by the
    /// uniform-scale path, where width and font scale together so the line
    /// breaks must not be recomputed (re-measuring words each frame on the
    /// wrap threshold is what makes the text flicker).
    fn recalc_height(&mut self) {
        let fs = self.data.style.font_size.max(MIN_FONT_SIZE);
        let num_lines = self.wrapped.lines.len().max(1);
        self.data.height = num_lines as f64 * fs * 1.2;
    }

    pub fn set_content(&mut self, raw: &str) {
        self.wrapped.set_raw(raw);
        self.recalc();
    }

    /// Change horizontal alignment without moving the visible text block.
    ///
    /// Alignment changes the position of shorter wrapped lines inside the
    /// column, but the text element itself should remain anchored where the
    /// user placed it. Since `visual_bounds` is intentionally tight around
    /// the glyphs, changing alignment would otherwise move its left edge and
    /// make the selected element jump.
    pub fn set_alignment(&mut self, align: TextAlign) {
        if self.align == align {
            return;
        }

        let before_x = self.visual_bounds().x;
        self.align = align;
        let after_x = self.visual_bounds().x;
        let local_dx = before_x - after_x;

        // Keep the element's horizontal anchor stable without changing its
        // existing rotation transform.
        self.data.world_point.offset(local_dx, 0.0);
    }

    /// Recompute the box after the font size changed externally (style panel).
    /// A box that was soft-wrapped keeps its wrap width and re-wraps against
    /// it; a box at its natural width grows with the new font so it never
    /// accidentally truncates.
    pub(crate) fn font_changed(&mut self) {
        let fs = self.data.style.font_size.max(MIN_FONT_SIZE);
        let ff = FONT_FAMILY;
        let hard_lines = self.wrapped.raw.split('\n').count();
        let wrapped = self.wrapped.lines.len() > hard_lines;
        if !wrapped {
            let natural = natural_line_width(&self.wrapped.raw, fs, ff);
            if self.data.width < natural {
                self.data.width = natural;
            }
        }
        self.recalc();
    }
}

impl FromDrag for Text {
    fn from_drag(anchor: Point, _current: Point, color: Color, _shift: bool) -> Self {
        let mut data = ElementData::new(0);
        data.world_point.set(anchor.x, anchor.y);
        data.width = 0.0;
        data.height = 0.0;
        data.style.stroke_color = color;
        Self {
            data,
            wrapped: WrappedText::new(""),
            align: TextAlign::Left,
        }
    }
}

impl UpdateDrag for Text {
    fn update_drag(&mut self, _current: Point, _anchor: Point, _shift: bool, _zoom: f64) {
        // Text elements are placed on click, not dragged to size.
    }
}

impl Render for Text {
    fn render(&self, _zoom: f64) -> leptos::View {
        let font_size = self.data.style.font_size.max(MIN_FONT_SIZE);
        let font_family = FONT_STACK;
        let lines = &self.wrapped.lines;
        let x = self.data.world_point.x;
        let baseline = self.data.world_point.y + font_size * TEXT_ASCENT_RATIO;
        let opacity = self.data.style.opacity;
        // Text is always painted with the stroke colour: that is the colour
        // the panel's swatches edit. (A fill colour can be inherited from the
        // default style, but it is meaningless for glyphs and used to make
        // the swatches look dead.)
        let fill = self.data.style.stroke_color.to_hex();

        // Alignment anchors each line inside the box column: lines of
        // different widths drift against `anchor_x` per `text-anchor`.
        let anchor = self.align.anchor();
        let anchor_x = match self.align {
            TextAlign::Left => x,
            TextAlign::Center => x + self.column_width() / 2.0,
            TextAlign::Right => x + self.column_width(),
        };

        let inner = if lines.len() <= 1 {
            leptos::view! {
                <text
                    x=anchor_x.to_string()
                    y=baseline.to_string()
                    fill=fill
                    font-size=font_size.to_string()
                    font-family=font_family
                    text-anchor=anchor
                    pointer-events="none"
                    style="user-select: none; -webkit-user-select: none;"
                    opacity=opacity
                >
                    {lines.first().cloned().unwrap_or_default()}
                </text>
            }
            .into_view()
        } else {
            leptos::view! {
                <text
                    x=anchor_x.to_string()
                    y=baseline.to_string()
                    fill=fill
                    font-size=font_size.to_string()
                    font-family=font_family
                    text-anchor=anchor
                    pointer-events="none"
                    style="user-select: none; -webkit-user-select: none;"
                    opacity=opacity
                >
                    {lines
                        .iter()
                        .enumerate()
                        .map(|(i, line)| {
                            let dy = if i == 0 { "0" } else { "1.2em" };
                            leptos::view! {
                                <tspan x=anchor_x.to_string() dy=dy>
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
            let cy = self.data.world_point.y + self.data.height / 2.0;
            let deg = self.data.rotation.to_degrees();
            leptos::view! { <g transform=format!("rotate({} {} {})", deg, cx, cy)>{inner}</g> }
                .into_view()
        }
    }
}

impl HitTest for Text {
    fn hit_test(&self, point: Point, margin: f64) -> bool {
        // Rotate the query into the text's own frame so a rotated box is
        // clicked where it is drawn, not at its unrotated envelope.
        let center = Point::new(
            self.data.world_point.x + self.data.width / 2.0,
            self.data.world_point.y + self.data.height / 2.0,
        );
        let pt = Point::unrotate(point, center.x, center.y, self.data.rotation);
        let (px, py) = (pt.x, pt.y);
        px >= self.data.world_point.x - margin
            && px <= self.data.world_point.x + self.data.width + margin
            && py >= self.data.world_point.y - margin
            && py <= self.data.world_point.y + self.data.height + margin
    }
}

impl Bounds for Text {
    fn bounds(&self) -> Rect {
        Rect::new(
            self.data.world_point.x,
            self.data.world_point.y,
            self.data.width,
            self.data.height,
        )
    }
}

impl Offset for Text {
    fn offset(&mut self, dx: f64, dy: f64) {
        self.data.world_point.offset(dx, dy);
    }
}

impl Text {
    /// Tight glyph box for the selection outline. The layout box
    /// (`data.width` × `data.height`) is a 1.2em line-height box, so it sits
    /// a lot taller and emptier than the visible glyphs; the outline should
    /// hug the text instead.
    pub fn visual_bounds(&self) -> Rect {
        let fs = self.data.style.font_size.max(MIN_FONT_SIZE);
        let ff = FONT_FAMILY;
        // The box hugs the actual glyphs, measured from the same canvas the
        // lines render against: the right edge matches the widest rendered
        // line (the wrap width `data.width` would leave a gap on every line
        // shorter than the column), and the vertical extent uses the glyphs'
        // own ascent/descent above and below the baseline instead of a fixed
        // fraction of the font size.
        let metrics: Vec<_> = self
            .wrapped
            .lines
            .iter()
            .map(|l| measure_text_extent(l, fs, ff))
            .collect();
        let baseline = self.data.world_point.y + fs * TEXT_ASCENT_RATIO;
        let width = metrics.iter().map(|m| m.0).fold(0.0, f64::max);
        let ascent = metrics.iter().map(|m| m.1).fold(0.0, f64::max);
        let descent = metrics.iter().map(|m| m.2).fold(0.0, f64::max);
        let num_lines = metrics.len().max(1);
        let height = ascent + descent + (num_lines - 1) as f64 * fs * 1.2;
        // Alignment slides the widest line around inside the box column, so
        // the outline follows where the glyphs actually sit.
        let column = self.column_width();
        let overflow = column - width;
        let x = match self.align {
            TextAlign::Left => self.data.world_point.x,
            TextAlign::Center => self.data.world_point.x + (overflow / 2.0).max(0.0),
            TextAlign::Right => self.data.world_point.x + overflow.max(0.0),
        };
        Rect::new(x, baseline - ascent, width, height)
    }
}

impl Rotate for Text {
    fn rotate_around(&mut self, point: Point, delta: f64) {
        rotate_bbox(&mut self.data, point, delta);
    }
}

impl Resize for Text {
    fn resize(&mut self, ctx: &ResizeContext) {
        let orig = ctx.orig.data();
        if ctx.multi {
            self.resize_multi(ctx, orig);
            return;
        }
        match ctx.handle {
            // Side handles: only width is free, so wrap words to the new width.
            ResizeHandle::E | ResizeHandle::W => self.resize_side(ctx, orig),
            // Corner and top/bottom handles scale the whole box as one thing —
            // the font size changes and the text keeps its shape.
            _ => self.resize_scale(ctx, orig),
        }
    }
}

impl Text {
    /// E/W drag: recompute `data.width`, soft-wrap the words into it, clamp
    /// the width at the longest word, and derive the height from the lines.
    fn resize_side(&mut self, ctx: &ResizeContext, orig: &ElementData) {
        let rotation = orig.rotation;
        let center = Point::new(
            orig.world_point.x + orig.width / 2.0,
            orig.world_point.y + orig.height / 2.0,
        );
        let local_pointer = rotate_point_around(ctx.pointer_world, center, -rotation);
        let anchor = ctx.handle.opposite_anchor(orig);
        let fs = orig.style.font_size.max(MIN_FONT_SIZE);
        let min_w = longest_word_width(&self.wrapped.raw, fs, FONT_FAMILY).max(MIN_ELEMENT_SIZE);

        let mut new_w = if ctx.alt {
            (2.0 * (local_pointer.x - center.x).abs()).max(MIN_ELEMENT_SIZE)
        } else {
            (local_pointer.x - anchor.x).abs().max(MIN_ELEMENT_SIZE)
        };
        // The box never narrows below the widest single word.
        new_w = new_w.max(min_w);

        let new_local_x = if ctx.alt {
            center.x - new_w / 2.0
        } else {
            // E keeps the left edge at the anchor; W keeps the right edge there.
            match ctx.handle {
                ResizeHandle::E => anchor.x,
                ResizeHandle::W => anchor.x - new_w,
                _ => unreachable!(),
            }
        };

        let new_center = Point::new(new_local_x + new_w / 2.0, center.y);
        let c = rotate_point_around(new_center, center, rotation);
        self.data.world_point = Point::new(c.x - new_w / 2.0, c.y - orig.height / 2.0);
        self.data.width = new_w;
        self.recalc();
    }

    /// Corner/down drag: scale width, height, and font size uniformly around
    /// the anchor (or, with Alt, around the center). The width-to-font ratio
    /// is preserved, so re-wrapping yields the same line breaks and the text
    /// keeps its shape.
    fn resize_scale(&mut self, ctx: &ResizeContext, orig: &ElementData) {
        let rotation = orig.rotation;
        let center = Point::new(
            orig.world_point.x + orig.width / 2.0,
            orig.world_point.y + orig.height / 2.0,
        );
        let (free_x, free_y) = ctx.handle.free_axes();
        if orig.width <= 0.0 || orig.height <= 0.0 {
            return;
        }
        let anchor = ctx.handle.opposite_anchor(orig);
        let local_pointer = rotate_point_around(ctx.pointer_world, center, -rotation);
        let scale = if ctx.alt {
            let distance = if free_x {
                2.0 * (local_pointer.x - center.x).abs()
            } else {
                2.0 * (local_pointer.y - center.y).abs()
            };
            distance / if free_x { orig.width } else { orig.height }
        } else if free_x && free_y {
            // Corner: measure both axes against the grab point. The scale
            // starts at exactly 1 (the handle sits outside the layout box, so
            // an absolute measure would jump on pointer-down) and a drag that
            // leans vertical still scales, instead of only hearing the
            // horizontal travel and appearing to lock.
            let start = rotate_point_around(ctx.start_world, center, -rotation);
            let sx = 1.0
                + ((local_pointer.x - anchor.x).abs() - (start.x - anchor.x).abs()) / orig.width;
            let sy = 1.0
                + ((local_pointer.y - anchor.y).abs() - (start.y - anchor.y).abs()) / orig.height;
            // Follow whichever axis the pointer actually moved along; taking
            // the larger factor would block shrinking drags on the other axis.
            if (sx - 1.0).abs() >= (sy - 1.0).abs() {
                sx
            } else {
                sy
            }
        } else {
            (local_pointer.y - anchor.y).abs() / orig.height
        };
        // Never shrink below the minimum font size.
        let min_scale = MIN_FONT_SIZE / orig.style.font_size.max(MIN_FONT_SIZE);
        let scale = scale.max(min_scale);

        let new_w = orig.width * scale;
        let new_h = orig.height * scale;
        let new_fs = orig.style.font_size * scale;

        let new_local_x = if ctx.alt {
            center.x - new_w / 2.0
        } else if (anchor.x - orig.world_point.x).abs() < 0.001 {
            orig.world_point.x
        } else {
            anchor.x - new_w
        };
        let new_local_y = if ctx.alt {
            center.y - new_h / 2.0
        } else if (anchor.y - orig.world_point.y).abs() < 0.001 {
            orig.world_point.y
        } else {
            anchor.y - new_h
        };

        let new_center = Point::new(new_local_x + new_w / 2.0, new_local_y + new_h / 2.0);
        let c = rotate_point_around(new_center, center, rotation);
        self.data.world_point = Point::new(c.x - new_w / 2.0, c.y - new_h / 2.0);
        self.data.width = new_w;
        self.data.style.font_size = new_fs.max(MIN_FONT_SIZE);
        // Keep the existing line breaks: width and font scaled by the same
        // factor, so the same words still fit each line.
        self.recalc_height();
    }

    /// Multi-select resize: scale the box and the font together.
    fn resize_multi(&mut self, ctx: &ResizeContext, orig: &ElementData) {
        if let Some((pos, size)) = resize_bbox(
            Point::new(ctx.bx, ctx.by),
            Point::new(ctx.bw, ctx.bh),
            ctx.pointer_world,
            ctx.handle,
            ctx.shift,
            ctx.alt,
        ) {
            let obw = ctx.bw.max(MIN_ELEMENT_SIZE);
            let sx = size.x / obw;
            resize_scale_element(
                &mut self.data,
                orig,
                pos,
                size.x,
                size.y,
                ctx.bx,
                ctx.by,
                ctx.bw,
                ctx.bh,
                false,
            );
            self.data.style.font_size = (orig.style.font_size * sx).max(MIN_FONT_SIZE);
            // Same line breaks, scaled: group scaling keeps the text's shape.
            self.recalc_height();
        }
    }
}

// Under wasm the app measures with the real font; natively (tests) it falls
// back to a deterministic heuristic of `0.55 * chars * font_size`.

#[cfg(test)]
mod tests {
    use super::*;

    fn text_elem(raw: &str, width: f64, font_size: f64) -> Text {
        let mut data = ElementData::new(1);
        data.world_point = Point::new(0.0, 0.0);
        data.width = width;
        data.height = font_size * 1.2;
        data.style.font_size = font_size;
        Text {
            data,
            wrapped: WrappedText::new(raw),
            align: TextAlign::Left,
        }
    }

    fn ctx<'a>(
        orig: &'a Element,
        handle: ResizeHandle,
        start: Point,
        pointer: Point,
        alt: bool,
    ) -> ResizeContext<'a> {
        ResizeContext {
            orig,
            handle,
            pointer_world: pointer,
            start_world: start,
            shift: false,
            alt,
            multi: false,
            bx: 0.0,
            by: 0.0,
            bw: 100.0,
            bh: 100.0,
        }
    }

    #[test]
    fn soft_wrap_moves_whole_words_to_new_lines() {
        let mut t = text_elem("a b c d", 0.0, 20.0);
        t.wrapped.rebuild(1.8 * 20.0, 20.0, FONT_FAMILY);
        assert_eq!(t.wrapped.lines, vec!["a b", "c d"]);
    }

    #[test]
    fn soft_wrap_respects_hard_newlines() {
        let mut t = text_elem("aa\nbb cc", 0.0, 20.0);
        t.wrapped.rebuild(1.5 * 20.0, 20.0, FONT_FAMILY);
        assert_eq!(t.wrapped.lines, vec!["aa", "bb", "cc"]);
    }

    #[test]
    fn longest_word_width_ignores_soft_wrap() {
        let w = longest_word_width("x supercalifragilistic y", 20.0, FONT_FAMILY);
        // "supercalifragilistic" = 20 chars → 20 * 20 * 0.55
        assert!((w - 220.0).abs() < 0.001);
    }

    #[test]
    fn side_resize_wraps_and_clamps_to_longest_word() {
        // "aa bb cc" at 20px: "aa"/"bb"/"cc" are 22 wide each, "aa bb" is 55,
        // "aa bb cc" is 88. A 56-wide box wraps as ["aa bb", "cc"].
        let mut t = text_elem("aa bb cc", 56.0, 20.0);
        t.recalc();
        assert_eq!(t.wrapped.lines, vec!["aa bb", "cc"]);
        let orig = Element::Text(t.clone());
        // W drag: pointer 6px from the right edge → 22-wide box clamped to the
        // longest word ("aa"/"bb"/"cc"). The right edge stays fixed at x=56.
        t.resize(&ctx(
            &orig,
            ResizeHandle::W,
            Point::new(56.0, 10.0),
            Point::new(50.0, 10.0),
            false,
        ));
        assert!((t.data.width - 22.0).abs() < 0.001);
        assert!((t.data.world_point.x - 34.0).abs() < 0.001);
        assert_eq!(t.wrapped.lines, vec!["aa", "bb", "cc"]);
        assert!((t.data.height - 20.0 * 1.2 * 3.0).abs() < 0.001);
    }

    #[test]
    fn corner_resize_scales_font_and_preserves_breaks() {
        let mut t = text_elem("aa bb cc", 56.0, 20.0);
        t.recalc();
        assert_eq!(t.wrapped.lines, vec!["aa bb", "cc"]);
        let orig = Element::Text(t.clone());
        // SE drag from the box corner to 1.5× the size → font, width and
        // height all scale 1.5×.
        let fs_before = t.data.style.font_size;
        t.resize(&ctx(
            &orig,
            ResizeHandle::Se,
            Point::new(56.0, 48.0),
            Point::new(84.0, 72.0),
            false,
        ));
        assert_eq!(t.data.style.font_size, fs_before * 1.5);
        assert!((t.data.width - 84.0).abs() < 0.001);
        assert!((t.data.height - 72.0).abs() < 0.001);
        assert_eq!(t.wrapped.lines, vec!["aa bb", "cc"]);
    }

    #[test]
    fn corner_resize_starts_at_one_and_follows_vertical_drags() {
        let mut t = text_elem("aa bb cc", 56.0, 20.0);
        t.recalc();
        let orig = Element::Text(t.clone());
        // The handle is drawn outside the layout box (tight glyph box + padding).
        let grab = Point::new(58.5, 29.0);
        // Grabbing the handle must not resize anything.
        t.resize(&ctx(&orig, ResizeHandle::Se, grab, grab, false));
        assert_eq!(t.data.style.font_size, 20.0);
        assert!((t.data.width - 56.0).abs() < 0.001);
        // Pulling straight down scales from the vertical travel alone.
        t.resize(&ctx(
            &orig,
            ResizeHandle::Se,
            grab,
            Point::new(58.5, 77.0),
            false,
        ));
        assert_eq!(t.data.style.font_size, 40.0);
        assert!((t.data.width - 112.0).abs() < 0.001);
        assert!((t.data.height - 96.0).abs() < 0.001);
    }

    #[test]
    fn text_height_follows_wrapped_lines() {
        let mut t = text_elem("one two three four", 1000.0, 20.0);
        t.recalc();
        // Everything fits on one line at 1000 width.
        assert_eq!(t.wrapped.lines.len(), 1);
        assert!((t.data.height - 24.0).abs() < 0.001);
        t.data.width = 20.0;
        t.recalc();
        // At 20 width every word is its own line.
        assert_eq!(t.wrapped.lines, vec!["one", "two", "three", "four"]);
        assert!((t.data.height - 24.0 * 4.0).abs() < 0.001);
    }

    #[test]
    fn font_change_grows_natural_box() {
        // Natural width of "aa bb cc" at 20px = 88 world units.
        let mut t = text_elem("aa bb cc", 88.0, 20.0);
        t.recalc();
        assert_eq!(t.wrapped.lines, vec!["aa bb cc"]);
        t.data.style.font_size = 40.0;
        t.font_changed();
        // Natural width grows with the font (176 world units at 40px).
        assert!((t.data.width - 176.0).abs() < 0.001);
        assert_eq!(t.wrapped.lines, vec!["aa bb cc"]);
        assert!((t.data.height - 48.0).abs() < 0.001);
    }

    #[test]
    fn font_change_rewraps_wrapped_box() {
        let mut t = text_elem("aa bb cc", 56.0, 20.0);
        t.recalc();
        assert_eq!(t.wrapped.lines, vec!["aa bb", "cc"]);
        t.data.style.font_size = 40.0;
        t.font_changed();
        // Wrap width is kept, so the larger font breaks every word.
        assert!((t.data.width - 56.0).abs() < 0.001);
        assert_eq!(t.wrapped.lines, vec!["aa", "bb", "cc"]);
        assert!((t.data.height - 48.0 * 3.0).abs() < 0.001);
    }

    #[test]
    fn alignment_preserves_visual_box_position() {
        // "aa bb cc" at 20px: "aa bb" = 55 wide, "aa bb cc" = 88 wide. A 70-wide
        // box wraps as ["aa bb", "cc"], so the widest line is 55 inside a
        // column wider than it — where alignment can move the outline.
        let mut t = text_elem("aa bb cc", 70.0, 20.0);
        t.recalc();
        assert_eq!(t.wrapped.lines, vec!["aa bb", "cc"]);
        let width = t.visual_bounds().w;
        assert!((width - 55.0).abs() < 0.001);

        let base = t.visual_bounds().x;
        t.set_alignment(TextAlign::Center);
        assert!((t.visual_bounds().x - base).abs() < 0.001);
        t.set_alignment(TextAlign::Right);
        assert!((t.visual_bounds().x - base).abs() < 0.001);
        t.set_alignment(TextAlign::Left);
        assert!((t.visual_bounds().x - base).abs() < 0.001);
    }

    #[test]
    fn alignment_does_notmove_unsized_fresh_text() {
        // Fresh text sits at its natural width, so the column equals the line:
        // every alignment collapses to the same outline.
        let mut t = text_elem("hello", 55.0, 20.0);
        t.recalc();
        let left = t.visual_bounds().x;
        t.set_alignment(TextAlign::Center);
        assert!((t.visual_bounds().x - left).abs() < 0.001);
    }

    #[test]
    fn hit_test_follows_the_rotation() {
        // A 100 × 24 box at the origin, rotated a quarter turn about its
        // centre (50, 12). (5, 2) lies inside the unrotated box but outside
        // the rotated one; (50, -30) is the other way round.
        let mut t = text_elem("hello", 100.0, 20.0);
        t.data.height = 24.0;
        assert!(t.hit_test(Point::new(5.0, 2.0), 0.0));
        t.data.rotation = std::f64::consts::FRAC_PI_2;
        assert!(!t.hit_test(Point::new(5.0, 2.0), 0.0));
        assert!(t.hit_test(Point::new(50.0, -30.0), 0.0));
    }
}
