use crate::model::elements::utils::{rotate_bbox, snap_bbox_to_grid};
use crate::model::resize::{resize_bbox, resize_from_handle, resize_scale_element, ResizeContext};
use crate::model::*;
use leptos::IntoView;

pub(crate) const MIN_FONT_SIZE: f64 = 12.0;
pub(crate) const TEXT_ASCENT_RATIO: f64 = 0.85;
/// Font family used for all text rendering and editing.
pub(crate) const FONT_FAMILY: &str = "Cabin";

/// Raw text split into lines on hard `\n` breaks only. No soft wrapping is
/// applied: a line is exactly what the user typed between two Enter presses.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WrappedText {
    /// Raw text with hard line breaks (from user Enter presses).
    pub raw: String,
    /// Lines split on hard `\n` breaks, one per rendered `<tspan>`.
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
}

// ── Text element ───────────────────────────────────────────────────────

/// A piece of text rendered as SVG.
///
/// No soft wrapping: lines come from the user's hard `\n` breaks only.
/// `data.width`/`data.height` are set from the editing overlay's measured
/// size on commit.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Text {
    /// Position, font size, and fill colour.
    pub data: ElementData,
    /// Text content split on hard line breaks.
    pub wrapped: WrappedText,
}

impl Text {
    /// Replace the raw content and update `data.height` to match the lines.
    /// Width is left untouched; without wrapping the text's width is not
    /// derived from the content at model level.
    fn recalc_height(&mut self) {
        let fs = self.data.style.font_size.max(MIN_FONT_SIZE);
        let line_h = fs * 1.2;
        let num_lines = self.wrapped.lines.len().max(1);
        self.data.height = num_lines as f64 * line_h;
    }

    pub fn set_content(&mut self, raw: &str) {
        self.wrapped.set_raw(raw);
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
        Self {
            data,
            wrapped: WrappedText::new(""),
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
        let font_family = FONT_FAMILY;
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
                    font-family=font_family
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
                    font-family=font_family
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
            let (pos, size) = match resize_bbox(
                Point::new(ctx.bx, ctx.by),
                Point::new(ctx.bw, ctx.bh),
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
                size.x,
                size.y,
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
        self.recalc_height();
    }
}
