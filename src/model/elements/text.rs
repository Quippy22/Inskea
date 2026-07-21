use crate::model::elements::utils::{rotate_bbox, snap_bbox_to_grid};
use crate::model::resize::{resize_bbox, resize_from_handle, resize_scale_element, ResizeContext};
use crate::model::*;
use leptos::IntoView;

pub(crate) const MIN_FONT_SIZE: f64 = 12.0;
pub(crate) const TEXT_ASCENT_RATIO: f64 = 0.85;

/// Estimated ratio of average character width to font size for sans-serif text.
/// Used to derive `max_chars` from world-space width and font size.
pub(crate) const CHAR_WIDTH_RATIO: f64 = 0.5;

/// A hand-built word-wrapping helper that gives the app full control over line breaks.
///
/// # Design
///
/// - `raw` holds the user-entered text with only **hard** `\n` (from Enter key presses).
/// - `lines` is derived from `raw` by inserting soft breaks every `max_chars` characters
///   within each hard-break segment. Hard `\n` are always preserved.
/// - Soft breaks are recalculated whenever the wrap width changes.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WrappedText {
    /// Raw text with only hard line breaks (from user Enter presses).
    pub raw: String,
    /// Wrapped lines, one per rendered `<tspan>`.
    pub lines: Vec<String>,
    /// Maximum number of characters per line at the current wrap width.
    /// 0 means wrapping is disabled (lines has a single entry).
    pub max_chars: usize,
}

impl WrappedText {
    /// Build a new `WrappedText` from raw content, computing soft breaks.
    pub fn new(raw: &str, width: f64, font_size: f64) -> Self {
        let max_chars = Self::compute_max_chars(width, font_size);
        let lines = Self::wrap(raw, max_chars);
        WrappedText {
            raw: raw.to_string(),
            lines,
            max_chars,
        }
    }

    /// Replace the raw content and recompute wrapping.
    pub fn set_raw(&mut self, raw: &str, width: f64, font_size: f64) {
        self.max_chars = Self::compute_max_chars(width, font_size);
        self.raw = raw.to_string();
        self.lines = Self::wrap(raw, self.max_chars);
    }

    /// Recompute wrapping for a new width (e.g. after resize).
    pub fn rewrap(&mut self, width: f64, font_size: f64) {
        self.max_chars = Self::compute_max_chars(width, font_size);
        self.lines = Self::wrap(&self.raw, self.max_chars);
    }

    fn compute_max_chars(width: f64, font_size: f64) -> usize {
        if width <= 0.0 || font_size <= 0.0 {
            return 0;
        }
        let char_width = font_size * CHAR_WIDTH_RATIO;
        (width / char_width).max(1.0) as usize
    }

    /// Wrap raw text into lines: split on hard `\n`, then split each segment
    /// into chunks of `max_chars` characters.
    fn wrap(raw: &str, max_chars: usize) -> Vec<String> {
        if max_chars == 0 {
            return vec![raw.to_string()];
        }
        let mut out = Vec::new();
        for segment in raw.split('\n') {
            let chars: Vec<char> = segment.chars().collect();
            if chars.len() <= max_chars {
                out.push(segment.to_string());
            } else {
                for chunk in chars.chunks(max_chars) {
                    out.push(chunk.iter().collect());
                }
            }
        }
        out
    }
}

// ── Text element ───────────────────────────────────────────────────────

/// A piece of text rendered as SVG, with automatic wrapping.
///
/// Wrapping is character-count based (see [`WrappedText`]).
/// The element is **self-sizing**: `data.width` and `data.height` are
/// recomputed from the actual wrapped content whenever the text changes.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Text {
    /// Position, font size, and fill colour.
    pub data: ElementData,
    /// Wrapped text with full control over hard and soft line breaks.
    pub wrapped: WrappedText,
}

impl Text {
    /// Replace the raw content, recompute wrapping at `wrap_width`,
    /// and update `data.width`/`data.height` to match the wrapped result.
    ///
    /// `wrap_width` determines `max_chars` for soft-break insertion and
    /// becomes `data.width` so the hitbox aligns with the wrap boundary.
    /// Height is auto-sized to fit all wrapped lines.
    fn recalc_height(&mut self) {
        let fs = self.data.style.font_size.max(MIN_FONT_SIZE);
        let line_h = fs * 1.2;
        let num_lines = self.wrapped.lines.len().max(1);
        self.data.height = num_lines as f64 * line_h;
    }

    pub fn set_content(&mut self, raw: &str, wrap_width: f64) {
        self.wrapped
            .set_raw(raw, wrap_width, self.data.style.font_size);
        self.data.width = wrap_width;
        self.recalc_height();
    }

    /// Resize the element to a new world-space width, re-wrap the text,
    /// and auto-adjust `data.height` to fit all lines.
    ///
    /// `data.width` stays at `new_width`; only the height is recomputed.
    pub fn resize_text(&mut self, new_width: f64) {
        self.data.width = new_width;
        self.wrapped.rewrap(new_width, self.data.style.font_size);
        self.recalc_height();
    }
}

impl FromDrag for Text {
    fn from_drag(anchor: Point, _current: Point, color: Color, _shift: bool) -> Self {
        let mut data = ElementData::new(0);
        data.world_point.set(anchor.x, anchor.y);
        data.width = 0.0;
        data.height = 0.0;
        data.style.stroke_color = color;
        let w = data.width;
        let fs = data.style.font_size;
        Self {
            data,
            wrapped: WrappedText::new("", w, fs),
        }
    }
}

impl UpdateDrag for Text {
    fn update_drag(&mut self, _current: Point, _anchor: Point, _shift: bool) {
        // Text elements are placed on click, not dragged to size.
    }
}

impl Render for Text {
    fn render(&self, _zoom: f64) -> leptos::View {
        let font_size = self.data.style.font_size.max(MIN_FONT_SIZE);
        let lines = &self.wrapped.lines;
        let x = self.data.world_point.x;
        let baseline = self.data.world_point.y + font_size * TEXT_ASCENT_RATIO;
        let opacity = self.data.style.opacity;
        let fill = self
            .data
            .style
            .fill_color
            .as_ref()
            .map(|c| c.to_hex())
            .unwrap_or_else(|| self.data.style.stroke_color.to_hex());

        let inner = if lines.len() <= 1 {
            leptos::view! {
                <text
                    x=x.to_string()
                    y=baseline.to_string()
                    fill=fill
                    font-size=font_size.to_string()
                    font-family="sans-serif"
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
                    x=x.to_string()
                    y=baseline.to_string()
                    fill=fill
                    font-size=font_size.to_string()
                    font-family="sans-serif"
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
                                <tspan x=x.to_string() dy=dy>
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
        let (px, py) = (point.x, point.y);
        px >= self.data.world_point.x - margin
            && px <= self.data.world_point.x + self.data.width + margin
            && py >= self.data.world_point.y - margin
            && py <= self.data.world_point.y + self.data.height + margin
    }
}

impl Bounds for Text {
    fn bounds(&self) -> (f64, f64, f64, f64) {
        (
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

impl SnapToGrid for Text {
    fn snap_to_grid(&mut self, grid: f64) {
        snap_bbox_to_grid(
            &mut self.data.world_point,
            self.data.width,
            self.data.height,
            grid,
        );
    }
}

impl Rotate for Text {
    fn rotate_around(&mut self, point: Point, delta: f64) {
        rotate_bbox(&mut self.data, point, delta);
    }
}

impl Resize for Text {
    fn resize(&mut self, ctx: &ResizeContext) {
        if ctx.multi {
            let (pos, (nw, nh)) = match resize_bbox(
                Point {
                    x: ctx.bx,
                    y: ctx.by,
                },
                (ctx.bw, ctx.bh),
                ctx.pointer_world,
                ctx.handle,
                ctx.shift,
                ctx.alt,
            ) {
                Some(v) => v,
                None => return,
            };
            resize_scale_element(
                &mut self.data,
                ctx.orig.data(),
                pos,
                nw,
                nh,
                ctx.bx,
                ctx.by,
                ctx.bw,
                ctx.bh,
                false,
            );
        } else {
            let result = resize_from_handle(
                &self.data,
                ctx.handle,
                ctx.pointer_world,
                ctx.shift,
                ctx.alt,
            );
            self.data.world_point = result.world_point;
            self.data.width = result.width;
            self.data.height = result.height;
        }
        self.resize_text(self.data.width);
    }
}
