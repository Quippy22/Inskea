use crate::model::elements::utils::{fill_paint, rect_from_drag, resize_box, rotate_bbox};
use crate::model::resize::ResizeContext;
use crate::model::*;
use leptos::IntoView;

/// Standard CSS / SVG resolution: 96 pixels per inch.
pub const PX_PER_INCH: f64 = 96.0;
/// Centimeters per inch.
pub const CM_PER_INCH: f64 = 2.54;
/// Millimeters per inch.
pub const MM_PER_INCH: f64 = 25.4;
/// Pixels per centimeter at 96 DPI (~37.79527559 px/cm).
pub const PX_PER_CM: f64 = PX_PER_INCH / CM_PER_INCH;
/// Pixels per millimeter at 96 DPI (~3.77952756 px/mm).
pub const PX_PER_MM: f64 = PX_PER_INCH / MM_PER_INCH;

/// Convert millimeters to rounded canvas pixels at 96 DPI.
pub fn mm_to_px(mm: f64) -> f64 {
    (mm * PX_PER_MM).round()
}

/// Convert centimeters to rounded canvas pixels at 96 DPI.
#[allow(dead_code)]
pub fn cm_to_px(cm: f64) -> f64 {
    (cm * PX_PER_CM).round()
}

/// Convert inches to rounded canvas pixels at 96 DPI.
#[allow(dead_code)]
pub fn in_to_px(inches: f64) -> f64 {
    (inches * PX_PER_INCH).round()
}

/// Convert canvas pixels to centimeters at 96 DPI.
pub fn px_to_cm(px: f64) -> f64 {
    px / PX_PER_CM
}

/// Convert canvas pixels to inches at 96 DPI.
pub fn px_to_in(px: f64) -> f64 {
    px / PX_PER_INCH
}

/// Predefined paper size presets (ISO A & B series) plus custom `Freehand`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum FramePreset {
    #[default]
    Freehand,
    A2,
    A3,
    A4,
    A5,
    B3,
    B4,
    B5,
}

impl FramePreset {
    pub const ALL: &'static [FramePreset] = &[
        FramePreset::Freehand,
        FramePreset::A4,
        FramePreset::A3,
        FramePreset::A2,
        FramePreset::A5,
        FramePreset::B4,
        FramePreset::B3,
        FramePreset::B5,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FramePreset::Freehand => "Freehand",
            FramePreset::A2 => "A2",
            FramePreset::A3 => "A3",
            FramePreset::A4 => "A4",
            FramePreset::A5 => "A5",
            FramePreset::B3 => "B3",
            FramePreset::B4 => "B4",
            FramePreset::B5 => "B5",
        }
    }

    /// Portrait dimensions in millimeters `(width_mm, height_mm)` for standard paper presets.
    /// Returns `None` for `Freehand`.
    pub fn size_mm(self) -> Option<(f64, f64)> {
        match self {
            FramePreset::Freehand => None,
            FramePreset::A2 => Some((420.0, 594.0)),
            FramePreset::A3 => Some((297.0, 420.0)),
            FramePreset::A4 => Some((210.0, 297.0)),
            FramePreset::A5 => Some((148.0, 210.0)),
            FramePreset::B3 => Some((353.0, 500.0)),
            FramePreset::B4 => Some((250.0, 353.0)),
            FramePreset::B5 => Some((176.0, 250.0)),
        }
    }

    /// Dimensions in canvas pixels `(width_px, height_px)` at 96 DPI for the given orientation.
    /// Returns `None` for `Freehand`.
    pub fn size_px(self, orientation: FrameOrientation) -> Option<(f64, f64)> {
        let (w_mm, h_mm) = self.size_mm()?;
        let w = mm_to_px(w_mm);
        let h = mm_to_px(h_mm);
        match orientation {
            FrameOrientation::Portrait => Some((w, h)),
            FrameOrientation::Landscape => Some((h, w)),
        }
    }
}

/// Portrait or landscape orientation for paper presets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum FrameOrientation {
    #[default]
    Portrait,
    Landscape,
}

impl FrameOrientation {
    pub fn label(self) -> &'static str {
        match self {
            FrameOrientation::Portrait => "Portrait",
            FrameOrientation::Landscape => "Landscape",
        }
    }
}

/// Default opacity for newly created frames (`0.6` = 60%).
pub const DEFAULT_FRAME_OPACITY: f64 = 0.6;

/// Default configuration for new frames created with the Frame tool.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameStyle {
    pub preset: FramePreset,
    pub orientation: FrameOrientation,
    pub opacity: f64,
}

impl Default for FrameStyle {
    fn default() -> Self {
        Self {
            preset: FramePreset::Freehand,
            orientation: FrameOrientation::Portrait,
            opacity: DEFAULT_FRAME_OPACITY,
        }
    }
}

/// A named, ordered canvas frame / artboard.
///
/// Behaves like a rectangle on the canvas, with optional ISO A/B paper size presets,
/// physical dimension conversions (px / cm / in), an ordered page index, and zoom-to-fit focus.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Frame {
    /// Position, size, and appearance (stroke/fill/rotation).
    pub data: ElementData,
    /// Display name of the frame (e.g. `"Frame 1"`).
    #[serde(default)]
    pub name: String,
    /// Active size preset (`Freehand`, `A4`, `A3`, `A2`, `B4`, etc.).
    #[serde(default)]
    pub preset: FramePreset,
    /// Paper orientation when a preset size is used.
    #[serde(default)]
    pub orientation: FrameOrientation,
    /// 1-based page order index in the document's frame sequence.
    #[serde(default)]
    pub frame_order: u32,
}

impl Frame {
    /// Header label text shown at the top-left of the frame on canvas.
    pub fn header_label(&self) -> String {
        let title = if self.name.trim().is_empty() {
            if self.frame_order > 0 {
                format!("Frame {}", self.frame_order)
            } else {
                "Frame".to_string()
            }
        } else {
            self.name.clone()
        };
        match self.preset {
            FramePreset::Freehand => title,
            preset => format!("{} · {}", title, preset.label()),
        }
    }

    /// Format the frame's current dimensions in centimeters, inches, and pixels.
    pub fn dimensions_summary(&self) -> String {
        let w_px = self.data.width.abs();
        let h_px = self.data.height.abs();
        let w_cm = px_to_cm(w_px);
        let h_cm = px_to_cm(h_px);
        let w_in = px_to_in(w_px);
        let h_in = px_to_in(h_px);
        format!(
            "{:.1} × {:.1} cm  ({:.1} × {:.1} in · {:.0} × {:.0} px)",
            w_cm, h_cm, w_in, h_in, w_px, h_px
        )
    }

    /// Apply a preset and orientation to this frame, resizing around its **center**
    /// when the frame already exists on the canvas (`self.data.id != 0`), or setting
    /// its initial dimensions before placement when `self.data.id == 0`.
    pub fn apply_preset(&mut self, preset: FramePreset, orientation: FrameOrientation) {
        let prev_orientation = self.orientation;
        self.preset = preset;
        self.orientation = orientation;
        let can_pivot_center =
            self.data.id != 0 && self.data.width > 0.0 && self.data.height > 0.0;
        if let Some((w, h)) = preset.size_px(orientation) {
            if can_pivot_center {
                let cx = self.data.world_point.x + self.data.width / 2.0;
                let cy = self.data.world_point.y + self.data.height / 2.0;
                self.data.world_point.set(cx - w / 2.0, cy - h / 2.0);
            }
            self.data.width = w;
            self.data.height = h;
        } else if prev_orientation != orientation && can_pivot_center {
            let cx = self.data.world_point.x + self.data.width / 2.0;
            let cy = self.data.world_point.y + self.data.height / 2.0;
            let (new_w, new_h) = (self.data.height, self.data.width);
            self.data
                .world_point
                .set(cx - new_w / 2.0, cy - new_h / 2.0);
            self.data.width = new_w;
            self.data.height = new_h;
        }
    }

    /// Apply the styling panel's default frame style.
    pub fn apply_frame_style(&mut self, style: &FrameStyle) {
        self.apply_preset(style.preset, style.orientation);
        self.data.style.opacity = style.opacity;
        self.data.style.stroke_width = 1.5;
        self.data.style.stroke_style = super::StrokeStyle::Solid;
        self.data.style.edge_style = super::EdgeStyle::Sharp;
        self.data.style.roundness = 0.0;
    }

    fn render_inner(&self, show_label: bool, zoom: f64) -> leptos::View {
        let x = self.data.world_point.x;
        let y = self.data.world_point.y;
        let w = self.data.width;
        let h = self.data.height;
        let sw = 1.5_f64;
        let fill = fill_paint(&self.data.style.fill_color);
        let stroke = self.data.style.stroke_color.to_hex();
        let opacity = self.data.style.opacity;

        // Keep the frame title readable across zoom levels without growing huge when zoomed in.
        let eff_zoom = zoom.clamp(0.15, 4.0);
        let badge_font_size = (12.0 / eff_zoom).clamp(8.0, 28.0);
        let badge_offset_y = (6.0 / eff_zoom).clamp(4.0, 16.0);
        let header_text = self.header_label();

        // Corner ticks subtly distinguish a Frame / Artboard from an ordinary rectangle.
        let tick = (10.0 / eff_zoom).clamp(4.0, 24.0).min(w / 4.0).min(h / 4.0);
        let tick_sw = (1.25 / eff_zoom).clamp(0.75, 3.0);
        let ticks_d = format!(
            "M {} {} L {} {} L {} {} \
             M {} {} L {} {} L {} {} \
             M {} {} L {} {} L {} {} \
             M {} {} L {} {} L {} {}",
            x,
            y + tick,
            x,
            y,
            x + tick,
            y,
            x + w - tick,
            y,
            x + w,
            y,
            x + w,
            y + tick,
            x,
            y + h - tick,
            x,
            y + h,
            x + tick,
            y + h,
            x + w - tick,
            y + h,
            x + w,
            y + h,
            x + w,
            y + h - tick,
        );

        let frame_body = leptos::view! {
            <g opacity=opacity>
                <rect
                    x=x
                    y=y
                    width=w
                    height=h
                    fill=fill
                    stroke=stroke.clone()
                    stroke-width=sw
                    stroke-linejoin="miter"
                    pointer-events="stroke"
                />
                <path
                    d=ticks_d
                    fill="none"
                    stroke=stroke.clone()
                    stroke-width=tick_sw
                    stroke-linecap="square"
                    opacity="0.65"
                />
                <text
                    x=x
                    y=y - badge_offset_y
                    fill=stroke
                    font-size=badge_font_size
                    font-family="sans-serif"
                    font-weight="600"
                    style="user-select: none; pointer-events: none;"
                    opacity="0.85"
                >
                    {header_text}
                </text>
            </g>
        }
        .into_view();

        let shape = if self.data.rotation == 0.0 {
            frame_body
        } else {
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            let deg = self.data.rotation.to_degrees();
            leptos::view! {
                <g transform=format!("rotate({} {} {})", deg, cx, cy)>
                    {frame_body}
                </g>
            }
            .into_view()
        };

        if !show_label {
            return shape;
        }
        let Some(label) = self.data.embedded_text.as_ref() else {
            return shape;
        };
        let Some(layout) = super::embedded_text::layout_rectangle(&self.data) else {
            return shape;
        };
        let text = super::embedded_text::render_label(label, &layout, opacity);
        leptos::view! { <g>{shape}{text}</g> }.into_view()
    }
}

impl FromDrag for Frame {
    fn from_drag(anchor: Point, current: Point, color: Color, shift: bool) -> Self {
        let (pt, w, h) = rect_from_drag(anchor, current, shift);
        Self {
            data: ElementData {
                world_point: pt,
                width: w,
                height: h,
                style: super::ElementStyle {
                    stroke_color: color,
                    stroke_width: 1.5,
                    roundness: 0.0,
                    opacity: DEFAULT_FRAME_OPACITY,
                    ..Default::default()
                },
                ..ElementData::new(0)
            },
            name: String::new(),
            preset: FramePreset::Freehand,
            orientation: FrameOrientation::Portrait,
            frame_order: 0,
        }
    }
}

impl UpdateDrag for Frame {
    fn update_drag(&mut self, current: Point, anchor: Point, shift: bool, _zoom: f64) {
        if let Some((preset_w, preset_h)) = self.preset.size_px(self.orientation) {
            let x = if current.x >= anchor.x {
                anchor.x
            } else {
                anchor.x - preset_w
            };
            let y = if current.y >= anchor.y {
                anchor.y
            } else {
                anchor.y - preset_h
            };
            self.data.world_point.set(x, y);
            self.data.width = preset_w;
            self.data.height = preset_h;
        } else {
            let (pt, w, h) = rect_from_drag(anchor, current, shift);
            self.data.world_point.set(pt.x, pt.y);
            self.data.width = w;
            self.data.height = h;
        }
    }
}

impl Render for Frame {
    fn render(&self, zoom: f64) -> leptos::View {
        self.render_inner(true, zoom)
    }

    fn render_label_mask(&self) -> leptos::View {
        self.render_inner(false, 1.0)
    }
}

impl HitTest for Frame {
    fn hit_test(&self, point: Point, margin: f64) -> bool {
        let cx = self.data.world_point.x + self.data.width / 2.0;
        let cy = self.data.world_point.y + self.data.height / 2.0;
        let pt = Point::unrotate(point, cx, cy, self.data.rotation);
        let px = pt.x;
        let py = pt.y;
        let x = self.data.world_point.x;
        let y = self.data.world_point.y;
        let w = self.data.width;
        let h = self.data.height;

        // Allow clicking the top-left header label badge region above the frame.
        let header_h = 24.0;
        let header_w = (w * 0.75).clamp(60.0, 300.0);
        if px >= x - margin
            && px <= x + header_w + margin
            && py >= y - header_h - margin
            && py <= y + margin
        {
            return true;
        }

        // Frames are visual artboards with no clickable fill.
        // Clicking inside the frame has normal canvas click behavior.
        // Only clicking the frame's edges/borders selects the frame.
        let dl = (px - x).abs();
        let dr = (px - (x + w)).abs();
        let dt = (py - y).abs();
        let db = (py - (y + h)).abs();
        let near_edge = dl.min(dr).min(dt).min(db);
        let stroke_w = self.data.style.stroke_width.max(1.5);
        let threshold = (margin + stroke_w).max(8.0);

        near_edge <= threshold
            && px >= x - threshold
            && px <= x + w + threshold
            && py >= y - threshold
            && py <= y + h + threshold
    }
}

impl Bounds for Frame {
    fn bounds(&self) -> Rect {
        Rect::new(
            self.data.world_point.x,
            self.data.world_point.y,
            self.data.width,
            self.data.height,
        )
    }
}

impl Offset for Frame {
    fn offset(&mut self, dx: f64, dy: f64) {
        self.data.world_point.offset(dx, dy);
    }
}

impl Rotate for Frame {
    fn rotate_around(&mut self, point: Point, delta: f64) {
        rotate_bbox(&mut self.data, point, delta);
    }
}

impl Resize for Frame {
    fn resize(&mut self, ctx: &ResizeContext) {
        resize_box(&mut self.data, ctx);
        if let Some((pw, ph)) = self.preset.size_px(self.orientation) {
            if (self.data.width - pw).abs() > 0.5 || (self.data.height - ph).abs() > 0.5 {
                self.preset = FramePreset::Freehand;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_preset_pixel_conversions_follow_96_dpi() {
        // A4: 210mm × 297mm -> 794 × 1123 px at 96 DPI
        assert_eq!(
            FramePreset::A4.size_px(FrameOrientation::Portrait),
            Some((794.0, 1123.0))
        );
        assert_eq!(
            FramePreset::A4.size_px(FrameOrientation::Landscape),
            Some((1123.0, 794.0))
        );
        // A3: 297mm × 420mm -> 1123 × 1587 px
        assert_eq!(
            FramePreset::A3.size_px(FrameOrientation::Portrait),
            Some((1123.0, 1587.0))
        );
        // B4: 250mm × 353mm -> 945 × 1334 px
        assert_eq!(
            FramePreset::B4.size_px(FrameOrientation::Portrait),
            Some((945.0, 1334.0))
        );
        assert_eq!(FramePreset::Freehand.size_px(FrameOrientation::Portrait), None);
    }

    #[test]
    fn unit_conversions_round_trip_cleanly() {
        let px = cm_to_px(21.0);
        assert_eq!(px, 794.0);
        let cm = px_to_cm(px);
        assert!((cm - 21.0).abs() < 0.05);

        let inch_px = in_to_px(8.5);
        assert_eq!(inch_px, 816.0);
        assert!((px_to_in(inch_px) - 8.5).abs() < 1e-6);
    }

    #[test]
    fn switching_orientation_pivots_around_frame_center() {
        let mut frame = Frame::from_drag(
            Point::new(100.0, 200.0),
            Point::new(894.0, 1323.0),
            Color::default(),
            false,
        );
        frame.data.id = 1;
        frame.apply_preset(FramePreset::A4, FrameOrientation::Portrait);
        let center_before = frame.bounds().center();

        frame.apply_preset(FramePreset::A4, FrameOrientation::Landscape);
        let center_after = frame.bounds().center();

        assert_eq!((frame.data.width, frame.data.height), (1123.0, 794.0));
        assert!((center_before.x - center_after.x).abs() < 1e-6);
        assert!((center_before.y - center_after.y).abs() < 1e-6);
    }

    #[test]
    fn new_frames_default_to_sixty_percent_opacity() {
        let frame = Frame::from_drag(
            Point::new(0.0, 0.0),
            Point::new(200.0, 100.0),
            Color::default(),
            false,
        );
        assert!((frame.data.style.opacity - 0.6).abs() < 1e-9);
        assert!((FrameStyle::default().opacity - 0.6).abs() < 1e-9);
    }
}
