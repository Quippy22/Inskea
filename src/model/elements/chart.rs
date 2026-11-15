use crate::model::elements::text::{FONT_STACK, TEXT_ASCENT_RATIO};
use crate::model::elements::utils::{rect_from_drag, resize_box, rotate_bbox};
use crate::model::resize::ResizeContext;
use crate::model::*;
use leptos::{view, IntoView};
use std::f64::consts::{FRAC_PI_2, TAU};

/// Minimum width or height of a chart element.
pub const MIN_CHART_SIZE: f64 = 40.0;

/// Default dimensions when inserting a chart from the menu.
pub const DEFAULT_CHART_WIDTH: f64 = 320.0;
pub const DEFAULT_CHART_HEIGHT: f64 = 220.0;

/// Supported visual chart archetypes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum ChartKind {
    #[default]
    Bar,
    Line,
    Area,
    Donut,
    Sparkline,
}

impl ChartKind {
    pub const ALL: [ChartKind; 5] = [
        ChartKind::Bar,
        ChartKind::Line,
        ChartKind::Area,
        ChartKind::Donut,
        ChartKind::Sparkline,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ChartKind::Bar => "Bar",
            ChartKind::Line => "Line",
            ChartKind::Area => "Area",
            ChartKind::Donut => "Donut",
            ChartKind::Sparkline => "Sparkline",
        }
    }

    pub fn default_dimensions(&self) -> (f64, f64) {
        match self {
            ChartKind::Bar | ChartKind::Line | ChartKind::Area => {
                (DEFAULT_CHART_WIDTH, DEFAULT_CHART_HEIGHT)
            }
            ChartKind::Donut => (260.0, 240.0),
            ChartKind::Sparkline => (220.0, 90.0),
        }
    }
}

/// Curated multi-series color palettes that harmonize with Inskea themes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum ChartPalette {
    #[default]
    Ocean,
    Sunset,
    Forest,
    Pastel,
    Neon,
    Monochrome,
}

impl ChartPalette {
    pub const ALL: [ChartPalette; 6] = [
        ChartPalette::Ocean,
        ChartPalette::Sunset,
        ChartPalette::Forest,
        ChartPalette::Pastel,
        ChartPalette::Neon,
        ChartPalette::Monochrome,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ChartPalette::Ocean => "Ocean",
            ChartPalette::Sunset => "Sunset",
            ChartPalette::Forest => "Forest",
            ChartPalette::Pastel => "Pastel",
            ChartPalette::Neon => "Neon",
            ChartPalette::Monochrome => "Mono",
        }
    }

    pub fn swatches(&self) -> [&'static str; 6] {
        match self {
            ChartPalette::Ocean => [
                "#38bdf8", "#818cf8", "#34d399", "#f472b6", "#fbbf24", "#a78bfa",
            ],
            ChartPalette::Sunset => [
                "#f87171", "#fb923c", "#facc15", "#4ade80", "#38bdf8", "#c084fc",
            ],
            ChartPalette::Forest => [
                "#34d399", "#10b981", "#a3e635", "#2dd4bf", "#60a5fa", "#fbbf24",
            ],
            ChartPalette::Pastel => [
                "#93c5fd", "#fdba74", "#86efac", "#f9a8d4", "#fde047", "#c4b5fd",
            ],
            ChartPalette::Neon => [
                "#22d3ee", "#a855f7", "#ec4899", "#eab308", "#10b981", "#3b82f6",
            ],
            ChartPalette::Monochrome => [
                "#ffffff", "#cbd5e1", "#94a3b8", "#64748b", "#475569", "#334155",
            ],
        }
    }

    pub fn color_at(&self, index: usize, stroke_color: &Color) -> String {
        if *self == ChartPalette::Monochrome {
            return stroke_color.to_hex();
        }
        let palette = self.swatches();
        palette[index % palette.len()].to_string()
    }
}

/// One labeled data entry in a chart.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChartItem {
    pub label: String,
    pub value: f64,
}

impl ChartItem {
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value: if value.is_finite() { value } else { 0.0 },
        }
    }
}

pub fn default_chart_items() -> Vec<ChartItem> {
    vec![
        ChartItem::new("Q1", 42.0),
        ChartItem::new("Q2", 78.0),
        ChartItem::new("Q3", 56.0),
        ChartItem::new("Q4", 94.0),
    ]
}

fn default_true() -> bool {
    true
}

/// Shared chart configuration used for draw-mode defaults and panel state.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ChartStyle {
    pub kind: ChartKind,
    pub palette: ChartPalette,
    pub show_grid: bool,
    pub show_labels: bool,
    pub show_values: bool,
    pub show_legend: bool,
}

impl Default for ChartStyle {
    fn default() -> Self {
        Self {
            kind: ChartKind::Bar,
            palette: ChartPalette::Ocean,
            show_grid: true,
            show_labels: true,
            show_values: true,
            show_legend: false,
        }
    }
}

/// A first-class vector chart element on the canvas.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Chart {
    pub data: ElementData,
    #[serde(default)]
    pub kind: ChartKind,
    #[serde(default)]
    pub palette: ChartPalette,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(default = "default_chart_items")]
    pub items: Vec<ChartItem>,
    #[serde(default = "default_true")]
    pub show_grid: bool,
    #[serde(default = "default_true")]
    pub show_labels: bool,
    #[serde(default = "default_true")]
    pub show_values: bool,
    #[serde(default)]
    pub show_legend: bool,
}

// ── Layout & Scene Graph Primitives ────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub struct ChartRectPrimitive {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub rx: f64,
    pub fill: String,
    pub fill_opacity: f64,
    pub stroke: String,
    pub stroke_width: f64,
    pub stroke_dasharray: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartLinePrimitive {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub stroke: String,
    pub stroke_width: f64,
    pub stroke_dasharray: String,
    pub opacity: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartPathPrimitive {
    pub d: String,
    pub fill: String,
    pub fill_opacity: f64,
    pub stroke: String,
    pub stroke_width: f64,
    pub stroke_dasharray: String,
    pub linecap: &'static str,
    pub linejoin: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartCirclePrimitive {
    pub cx: f64,
    pub cy: f64,
    pub r: f64,
    pub fill: String,
    pub stroke: String,
    pub stroke_width: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartTextPrimitive {
    pub x: f64,
    pub y: f64,
    pub text: String,
    pub font_size: f64,
    pub anchor: &'static str,
    pub fill: String,
    pub opacity: f64,
    pub bold: bool,
}

/// Complete vector geometry of a chart in local (unrotated) world coordinates,
/// shared by both the live Leptos canvas renderer and the SVG/PNG exporter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChartSceneGraph {
    pub card: Option<ChartRectPrimitive>,
    pub lines: Vec<ChartLinePrimitive>,
    pub rects: Vec<ChartRectPrimitive>,
    pub paths: Vec<ChartPathPrimitive>,
    pub circles: Vec<ChartCirclePrimitive>,
    pub texts: Vec<ChartTextPrimitive>,
}

/// Computed plot-area metrics in unrotated coordinates.
#[derive(Clone, Copy, Debug)]
pub struct PlotMetrics {
    pub plot: Rect,
    pub max_val: f64,
}

impl Chart {
    /// Create a new chart centered at `centre` with default sample data.
    pub fn new_at(centre: Point, kind: ChartKind, color: Color) -> Self {
        let (w, h) = kind.default_dimensions();
        let mut data = ElementData::new(0);
        data.world_point = Point::new(centre.x - w / 2.0, centre.y - h / 2.0);
        data.width = w;
        data.height = h;
        data.style.stroke_color = color;
        data.style.edge_style = EdgeStyle::Rounded;
        data.style.roundness = 6.0;
        Self {
            data,
            kind,
            palette: ChartPalette::Ocean,
            title: String::new(),
            items: default_chart_items(),
            show_grid: true,
            show_labels: true,
            show_values: true,
            show_legend: false,
        }
    }

    pub fn chart_style(&self) -> ChartStyle {
        ChartStyle {
            kind: self.kind,
            palette: self.palette,
            show_grid: self.show_grid,
            show_labels: self.show_labels,
            show_values: self.show_values,
            show_legend: self.show_legend,
        }
    }

    pub fn apply_chart_style(&mut self, style: &ChartStyle) {
        self.kind = style.kind;
        self.palette = style.palette;
        self.show_grid = style.show_grid;
        self.show_labels = style.show_labels;
        self.show_values = style.show_values;
        self.show_legend = style.show_legend;
    }

    /// Maximum positive value across all items, clamped to at least 1.0 so
    /// division never produces NaN/Inf.
    pub fn scale_max(&self) -> f64 {
        let max = self
            .items
            .iter()
            .map(|item| item.value.max(0.0))
            .fold(0.0_f64, f64::max);
        if max <= 1e-6 {
            100.0
        } else {
            max
        }
    }

    /// Format a numeric value concisely (e.g. `42`, `12.5`, `1.2k`).
    pub fn format_value(val: f64) -> String {
        let abs = val.abs();
        if abs >= 1_000_000.0 {
            format!("{:.1}M", val / 1_000_000.0)
        } else if abs >= 10_000.0 {
            format!("{:.1}k", val / 1_000.0)
        } else if (val - val.round()).abs() < 0.05 {
            format!("{:.0}", val.round())
        } else {
            format!("{:.1}", val)
        }
    }

    /// Compute the inner plot rectangle and scale maximum in unrotated space.
    pub fn plot_metrics(&self) -> PlotMetrics {
        let x = self.data.world_point.x;
        let y = self.data.world_point.y;
        let w = self.data.width.max(MIN_CHART_SIZE);
        let h = self.data.height.max(MIN_CHART_SIZE);
        let max_val = self.scale_max();

        if self.kind == ChartKind::Sparkline {
            let pad_x = (w * 0.06).clamp(6.0, 16.0);
            let pad_y = (h * 0.16).clamp(6.0, 20.0);
            return PlotMetrics {
                plot: Rect::new(
                    x + pad_x,
                    y + pad_y,
                    (w - pad_x * 2.0).max(8.0),
                    (h - pad_y * 2.0).max(8.0),
                ),
                max_val,
            };
        }

        let base_fs = (self.data.style.font_size * 0.65).clamp(9.0, 14.0);
        let has_title = !self.title.trim().is_empty();
        let title_space = if has_title { base_fs * 2.0 + 6.0 } else { 0.0 };
        let value_headroom = if self.show_values && self.kind != ChartKind::Donut {
            base_fs * 1.4
        } else {
            6.0
        };
        let label_space = if self.show_labels && self.kind != ChartKind::Donut {
            base_fs * 1.7 + 4.0
        } else {
            8.0
        };
        let legend_space = if self.show_legend && !self.items.is_empty() {
            base_fs * 1.8 + 6.0
        } else {
            0.0
        };
        let left_space = if self.show_grid && self.kind != ChartKind::Donut && w >= 140.0 {
            (base_fs * 2.6).clamp(24.0, 42.0)
        } else {
            12.0
        };
        let right_space: f64 = 12.0;

        let top = (10.0 + title_space + value_headroom).min(h * 0.45);
        let bottom = (8.0 + label_space + legend_space).min(h * 0.45);
        let left = left_space.min(w * 0.35);
        let right = right_space.min(w * 0.2);

        let pw = (w - left - right).max(16.0);
        let ph = (h - top - bottom).max(16.0);

        PlotMetrics {
            plot: Rect::new(x + left, y + top, pw, ph),
            max_val,
        }
    }

    /// Unrotated positions of each data item's interactive value handle / anchor.
    pub fn unrotated_value_handles(&self) -> Vec<(usize, Point)> {
        let n = self.items.len();
        if n == 0 {
            return Vec::new();
        }
        let metrics = self.plot_metrics();
        let plot = metrics.plot;
        let max_val = metrics.max_val;

        match self.kind {
            ChartKind::Bar => {
                let slot_w = plot.w / n as f64;
                self.items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let ratio = (item.value.max(0.0) / max_val).clamp(0.0, 1.0);
                        let bh = (ratio * plot.h).max(2.0);
                        let cx = plot.x + (i as f64 + 0.5) * slot_w;
                        let cy = plot.y + plot.h - bh;
                        (i, Point::new(cx, cy))
                    })
                    .collect()
            }
            ChartKind::Line | ChartKind::Area | ChartKind::Sparkline => {
                self.line_vertices(&plot, max_val)
                    .into_iter()
                    .enumerate()
                    .collect()
            }
            ChartKind::Donut => {
                let (cx, cy, r_out, _) = self.donut_geometry(&plot);
                let total: f64 = self.items.iter().map(|i| i.value.max(0.0)).sum();
                let mut angle = -FRAC_PI_2;
                self.items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let sweep = if total <= 1e-6 {
                            TAU / n as f64
                        } else {
                            (item.value.max(0.0) / total) * TAU
                        };
                        let mid = angle + sweep / 2.0;
                        angle += sweep;
                        (
                            i,
                            Point::new(cx + r_out * mid.cos(), cy + r_out * mid.sin()),
                        )
                    })
                    .collect()
            }
        }
    }

    /// World-space positions of each data item's interactive handle and
    /// connector snap anchor, accounting for element rotation.
    pub fn value_handles(&self) -> Vec<(usize, Point)> {
        let mut handles = self.unrotated_value_handles();
        let rot = self.data.rotation;
        if rot != 0.0 {
            let pivot = self.bounds().center();
            for (_, pt) in &mut handles {
                pt.rotate_around(pivot, rot);
            }
        }
        handles
    }

    /// Update the value of `items[index]` from a pointer drag at `world`.
    /// `base_max` is the `scale_max()` captured when the drag began so that
    /// dragging the tallest bar does not fight a shifting scale mid-drag.
    pub fn set_value_from_world(
        &mut self,
        index: usize,
        world: Point,
        base_max: f64,
        base_item_value: f64,
        anchor_world: Point,
    ) {
        if index >= self.items.len() {
            return;
        }
        let pivot = self.bounds().center();
        let local = Point::unrotate(world, pivot.x, pivot.y, self.data.rotation);
        let local_anchor = Point::unrotate(anchor_world, pivot.x, pivot.y, self.data.rotation);
        let metrics = self.plot_metrics();
        let plot = metrics.plot;
        let max_val = base_max.max(1.0);

        let raw_val = match self.kind {
            ChartKind::Bar | ChartKind::Line | ChartKind::Area | ChartKind::Sparkline => {
                let bottom = plot.y + plot.h;
                let ratio = ((bottom - local.y) / plot.h.max(1.0)).clamp(0.0, 2.5);
                ratio * max_val
            }
            ChartKind::Donut => {
                // Vertical drag upward increases slice value; downward decreases it.
                let dy = local_anchor.y - local.y;
                let delta = (dy / plot.h.max(40.0)) * max_val;
                (base_item_value + delta).max(0.0)
            }
        };

        let rounded = if max_val >= 10.0 {
            raw_val.round()
        } else {
            (raw_val * 10.0).round() / 10.0
        };
        self.items[index].value = rounded.max(0.0);
    }

    fn line_vertices(&self, plot: &Rect, max_val: f64) -> Vec<Point> {
        let n = self.items.len();
        if n == 0 {
            return Vec::new();
        }
        self.items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let fx = if n == 1 {
                    0.5
                } else {
                    i as f64 / (n - 1) as f64
                };
                let ratio = (item.value.max(0.0) / max_val).clamp(0.0, 1.0);
                let px = plot.x + fx * plot.w;
                let py = plot.y + plot.h * (1.0 - ratio);
                Point::new(px, py)
            })
            .collect()
    }

    fn donut_geometry(&self, plot: &Rect) -> (f64, f64, f64, f64) {
        let cx = plot.x + plot.w / 2.0;
        let cy = plot.y + plot.h / 2.0;
        let r_out = (plot.w.min(plot.h) / 2.0 * 0.86).max(8.0);
        let r_in = r_out * 0.56;
        (cx, cy, r_out, r_in)
    }

    /// Build the complete vector scene graph for this chart in unrotated world coordinates.
    pub fn scene_graph(&self) -> ChartSceneGraph {
        let mut graph = ChartSceneGraph::default();
        let x = self.data.world_point.x;
        let y = self.data.world_point.y;
        let w = self.data.width.max(MIN_CHART_SIZE);
        let h = self.data.height.max(MIN_CHART_SIZE);
        let sw = self.data.style.stroke_width;
        let stroke_hex = self.data.style.stroke_color.to_hex();
        let dash = self.data.style.stroke_style.stroke_dasharray().to_string();
        let rounded = self.data.style.edge_style == EdgeStyle::Rounded;
        let rx = if rounded {
            self.data.style.roundness
        } else {
            0.0
        };
        let base_fs = (self.data.style.font_size * 0.65).clamp(9.0, 14.0);

        // Optional background card if fill_color is set
        if let Some(fill) = &self.data.style.fill_color {
            graph.card = Some(ChartRectPrimitive {
                x,
                y,
                w,
                h,
                rx,
                fill: fill.to_hex(),
                fill_opacity: 0.18,
                stroke: stroke_hex.clone(),
                stroke_width: (sw * 0.6).max(1.0),
                stroke_dasharray: dash.clone(),
            });
        }

        // Optional chart title
        if !self.title.trim().is_empty() && self.kind != ChartKind::Sparkline {
            let title_fs = (base_fs * 1.25).clamp(11.0, 18.0);
            graph.texts.push(ChartTextPrimitive {
                x: x + w / 2.0,
                y: y + 10.0 + title_fs * TEXT_ASCENT_RATIO,
                text: self.title.clone(),
                font_size: title_fs,
                anchor: "middle",
                fill: stroke_hex.clone(),
                opacity: 0.95,
                bold: true,
            });
        }

        let metrics = self.plot_metrics();
        let plot = metrics.plot;
        let max_val = metrics.max_val;

        // Gridlines & Y-axis labels (for Bar, Line, Area)
        if self.show_grid && matches!(self.kind, ChartKind::Bar | ChartKind::Line | ChartKind::Area)
        {
            let steps = if plot.h >= 110.0 { 4 } else { 2 };
            for step in 0..=steps {
                let frac = step as f64 / steps as f64;
                let gy = plot.y + plot.h * (1.0 - frac);
                let is_baseline = step == 0;
                graph.lines.push(ChartLinePrimitive {
                    x1: plot.x,
                    y1: gy,
                    x2: plot.x + plot.w,
                    y2: gy,
                    stroke: stroke_hex.clone(),
                    stroke_width: if is_baseline {
                        (sw * 0.65).max(1.0)
                    } else {
                        1.0
                    },
                    stroke_dasharray: if is_baseline {
                        "none".to_string()
                    } else {
                        "3 3".to_string()
                    },
                    opacity: if is_baseline { 0.45 } else { 0.2 },
                });

                if w >= 140.0 {
                    let tick_val = max_val * frac;
                    let tick_fs = (base_fs * 0.85).clamp(8.0, 12.0);
                    graph.texts.push(ChartTextPrimitive {
                        x: plot.x - 6.0,
                        y: gy + tick_fs * 0.35,
                        text: Self::format_value(tick_val),
                        font_size: tick_fs,
                        anchor: "end",
                        fill: stroke_hex.clone(),
                        opacity: 0.55,
                        bold: false,
                    });
                }
            }
        }

        let n = self.items.len();
        if n == 0 {
            return graph;
        }

        match self.kind {
            ChartKind::Bar => {
                let slot_w = plot.w / n as f64;
                let bar_w = (slot_w * 0.64).clamp(4.0, 64.0);
                let bar_rx = if rounded {
                    self.data.style.roundness.min(bar_w / 2.0)
                } else {
                    0.0
                };
                for (i, item) in self.items.iter().enumerate() {
                    let ratio = (item.value.max(0.0) / max_val).clamp(0.0, 1.0);
                    let bh = (ratio * plot.h).max(2.0);
                    let cx = plot.x + (i as f64 + 0.5) * slot_w;
                    let bx = cx - bar_w / 2.0;
                    let by = plot.y + plot.h - bh;
                    let color = self.palette.color_at(i, &self.data.style.stroke_color);

                    let fill_opacity = if self.palette == ChartPalette::Monochrome {
                        let shade = 0.85 - (i % 4) as f64 * 0.15;
                        shade.clamp(0.35, 0.9)
                    } else {
                        0.85
                    };

                    graph.rects.push(ChartRectPrimitive {
                        x: bx,
                        y: by,
                        w: bar_w,
                        h: bh,
                        rx: bar_rx.min(bh / 2.0),
                        fill: color.clone(),
                        fill_opacity,
                        stroke: color,
                        stroke_width: (sw * 0.75).max(1.0),
                        stroke_dasharray: dash.clone(),
                    });

                    if self.show_values {
                        let val_fs = (base_fs * 0.9).clamp(8.5, 12.5);
                        graph.texts.push(ChartTextPrimitive {
                            x: cx,
                            y: (by - 5.0).max(y + val_fs),
                            text: Self::format_value(item.value),
                            font_size: val_fs,
                            anchor: "middle",
                            fill: stroke_hex.clone(),
                            opacity: 0.85,
                            bold: false,
                        });
                    }

                    if self.show_labels {
                        graph.texts.push(ChartTextPrimitive {
                            x: cx,
                            y: plot.y + plot.h + base_fs * 1.15 + 2.0,
                            text: item.label.clone(),
                            font_size: base_fs,
                            anchor: "middle",
                            fill: stroke_hex.clone(),
                            opacity: 0.75,
                            bold: false,
                        });
                    }
                }
            }
            ChartKind::Line | ChartKind::Area | ChartKind::Sparkline => {
                let pts = self.line_vertices(&plot, max_val);
                let primary_color = self.palette.color_at(0, &self.data.style.stroke_color);
                let curve_mode = if rounded {
                    CurveMode::Curved
                } else {
                    CurveMode::Straight
                };
                let line_d = crate::model::elements::path::path_d(&pts, curve_mode);

                // Area fill under the curve for Area and Sparkline
                if matches!(self.kind, ChartKind::Area | ChartKind::Sparkline) && pts.len() >= 2 {
                    let first = pts[0];
                    let last = pts[pts.len() - 1];
                    let baseline_y = plot.y + plot.h;
                    let area_d = format!(
                        "{} L{} {} L{} {} Z",
                        line_d, last.x, baseline_y, first.x, baseline_y
                    );
                    graph.paths.push(ChartPathPrimitive {
                        d: area_d,
                        fill: primary_color.clone(),
                        fill_opacity: if self.kind == ChartKind::Sparkline {
                            0.16
                        } else {
                            0.24
                        },
                        stroke: "none".to_string(),
                        stroke_width: 0.0,
                        stroke_dasharray: "none".to_string(),
                        linecap: "round",
                        linejoin: "round",
                    });
                }

                if !line_d.is_empty() {
                    graph.paths.push(ChartPathPrimitive {
                        d: line_d,
                        fill: "none".to_string(),
                        fill_opacity: 0.0,
                        stroke: primary_color.clone(),
                        stroke_width: sw.max(1.5),
                        stroke_dasharray: dash.clone(),
                        linecap: "round",
                        linejoin: "round",
                    });
                }

                let dot_r = if self.kind == ChartKind::Sparkline {
                    (sw * 1.1).clamp(2.0, 3.5)
                } else {
                    (sw * 1.5).clamp(3.0, 5.5)
                };

                for (i, (pt, item)) in pts.iter().zip(self.items.iter()).enumerate() {
                    let node_color = if self.palette == ChartPalette::Monochrome {
                        primary_color.clone()
                    } else {
                        self.palette.color_at(i, &self.data.style.stroke_color)
                    };
                    graph.circles.push(ChartCirclePrimitive {
                        cx: pt.x,
                        cy: pt.y,
                        r: dot_r,
                        fill: node_color,
                        stroke: stroke_hex.clone(),
                        stroke_width: 1.2,
                    });

                    if self.kind != ChartKind::Sparkline {
                        if self.show_values {
                            let val_fs = (base_fs * 0.9).clamp(8.5, 12.5);
                            graph.texts.push(ChartTextPrimitive {
                                x: pt.x,
                                y: (pt.y - dot_r - 5.0).max(y + val_fs),
                                text: Self::format_value(item.value),
                                font_size: val_fs,
                                anchor: "middle",
                                fill: stroke_hex.clone(),
                                opacity: 0.85,
                                bold: false,
                            });
                        }
                        if self.show_labels {
                            graph.texts.push(ChartTextPrimitive {
                                x: pt.x,
                                y: plot.y + plot.h + base_fs * 1.15 + 2.0,
                                text: item.label.clone(),
                                font_size: base_fs,
                                anchor: "middle",
                                fill: stroke_hex.clone(),
                                opacity: 0.75,
                                bold: false,
                            });
                        }
                    }
                }
            }
            ChartKind::Donut => {
                let (cx, cy, r_out, r_in) = self.donut_geometry(&plot);
                let total: f64 = self.items.iter().map(|i| i.value.max(0.0)).sum();
                let mut start_angle = -FRAC_PI_2;

                for (i, item) in self.items.iter().enumerate() {
                    let sweep = if total <= 1e-6 {
                        TAU / n as f64
                    } else {
                        (item.value.max(0.0) / total) * TAU
                    };
                    if sweep <= 1e-5 {
                        continue;
                    }
                    let end_angle = start_angle + sweep;
                    let d = donut_slice_path(cx, cy, r_out, r_in, start_angle, end_angle);
                    let color = self.palette.color_at(i, &self.data.style.stroke_color);
                    let fill_opacity = if self.palette == ChartPalette::Monochrome {
                        (0.9 - (i % 5) as f64 * 0.14).clamp(0.3, 0.9)
                    } else {
                        0.88
                    };

                    graph.paths.push(ChartPathPrimitive {
                        d,
                        fill: color,
                        fill_opacity,
                        stroke: stroke_hex.clone(),
                        stroke_width: (sw * 0.6).max(1.0),
                        stroke_dasharray: dash.clone(),
                        linecap: "round",
                        linejoin: "round",
                    });

                    // Label / value on or near the slice
                    if self.show_labels || self.show_values {
                        let mid = start_angle + sweep / 2.0;
                        let label_r = (r_out + r_in) / 2.0;
                        let lx = cx + label_r * mid.cos();
                        let ly = cy + label_r * mid.sin();
                        let text = match (self.show_labels, self.show_values) {
                            (true, true) => {
                                format!("{} ({})", item.label, Self::format_value(item.value))
                            }
                            (true, false) => item.label.clone(),
                            (false, true) => Self::format_value(item.value),
                            (false, false) => String::new(),
                        };
                        if !text.is_empty() && sweep >= 0.22 {
                            let slice_fs = (base_fs * 0.85).clamp(8.0, 11.5);
                            graph.texts.push(ChartTextPrimitive {
                                x: lx,
                                y: ly + slice_fs * 0.35,
                                text,
                                font_size: slice_fs,
                                anchor: "middle",
                                fill: stroke_hex.clone(),
                                opacity: 0.95,
                                bold: true,
                            });
                        }
                    }

                    start_angle = end_angle;
                }

                // Center total label when no embedded text is set
                if self.data.embedded_text.is_none() && self.show_values && r_in >= 18.0 {
                    let center_fs = (base_fs * 1.1).clamp(9.0, 15.0);
                    graph.texts.push(ChartTextPrimitive {
                        x: cx,
                        y: cy + center_fs * 0.35,
                        text: Self::format_value(total),
                        font_size: center_fs,
                        anchor: "middle",
                        fill: stroke_hex.clone(),
                        opacity: 0.8,
                        bold: true,
                    });
                }
            }
        }

        // Optional Legend row at the bottom
        if self.show_legend && self.kind != ChartKind::Sparkline && !self.items.is_empty() {
            let legend_fs = (base_fs * 0.85).clamp(8.0, 11.5);
            let ly = y + h - 8.0;
            let item_spacing = (w - 20.0) / self.items.len() as f64;
            for (i, item) in self.items.iter().enumerate() {
                let lx = x + 12.0 + (i as f64 + 0.5) * item_spacing;
                let color = self.palette.color_at(i, &self.data.style.stroke_color);
                graph.circles.push(ChartCirclePrimitive {
                    cx: lx - legend_fs * 1.2,
                    cy: ly - legend_fs * 0.32,
                    r: 3.5,
                    fill: color,
                    stroke: "none".to_string(),
                    stroke_width: 0.0,
                });
                graph.texts.push(ChartTextPrimitive {
                    x: lx - legend_fs * 0.5,
                    y: ly,
                    text: item.label.clone(),
                    font_size: legend_fs,
                    anchor: "start",
                    fill: stroke_hex.clone(),
                    opacity: 0.8,
                    bold: false,
                });
            }
        }

        graph
    }

    /// Parse tabular text (CSV, TSV, semicolon-separated, Markdown table, or `Label: Value` lines)
    /// into an optional chart title and a list of `ChartItem`s.
    pub fn parse_tabular_data(input: &str) -> Option<(Option<String>, Vec<ChartItem>)> {
        let lines: Vec<&str> = input
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        if lines.is_empty() {
            return None;
        }

        let mut title: Option<String> = None;
        let mut items = Vec::new();

        for (line_idx, raw_line) in lines.iter().enumerate() {
            // Skip Markdown table separator rows like `|---|---|` or `| :--- | ---: |`
            let trimmed_pipes = raw_line.trim_matches('|').trim();
            if !trimmed_pipes.is_empty()
                && trimmed_pipes
                    .chars()
                    .all(|c| matches!(c, '-' | ':' | '|' | ' ' | '\t'))
            {
                continue;
            }

            let cells = split_tabular_row(raw_line);
            if cells.len() < 2 {
                continue;
            }

            let label_cell = cells[0].trim().trim_matches('"').trim_matches('\'').trim();
            let val_cell = cells[1].trim().trim_matches('"').trim_matches('\'').trim();

            if let Some(val) = parse_numeric_cell(val_cell) {
                let label = if label_cell.is_empty() {
                    format!("Item {}", items.len() + 1)
                } else {
                    label_cell.to_string()
                };
                items.push(ChartItem::new(label, val));
            } else if line_idx == 0 && items.is_empty() {
                // First row is a header row (e.g. "Quarter, Revenue (k)")
                if !val_cell.is_empty()
                    && !val_cell.eq_ignore_ascii_case("value")
                    && !val_cell.eq_ignore_ascii_case("values")
                {
                    title = Some(val_cell.to_string());
                }
            }
        }

        if items.is_empty() {
            None
        } else {
            Some((title, items))
        }
    }

    /// Serialize the current items into clean CSV text for editing or copying.
    pub fn to_csv_string(&self) -> String {
        let mut out = String::new();
        if !self.title.trim().is_empty() {
            out.push_str(&format!("Label,{}\n", self.title.trim()));
        }
        for item in &self.items {
            out.push_str(&format!("{},{}\n", item.label, item.value));
        }
        out
    }

    fn render_inner(&self, show_label: bool) -> leptos::View {
        let graph = self.scene_graph();
        let opacity = self.data.style.opacity;

        let card_view = graph.card.map(|c| {
            view! {
                <rect
                    x=c.x
                    y=c.y
                    width=c.w
                    height=c.h
                    rx=c.rx
                    ry=c.rx
                    fill=c.fill
                    fill-opacity=c.fill_opacity
                    stroke=c.stroke
                    stroke-width=c.stroke_width
                    stroke-dasharray=c.stroke_dasharray
                />
            }
        });

        let lines_view: Vec<_> = graph
            .lines
            .into_iter()
            .map(|l| {
                view! {
                    <line
                        x1=l.x1
                        y1=l.y1
                        x2=l.x2
                        y2=l.y2
                        stroke=l.stroke
                        stroke-width=l.stroke_width
                        stroke-dasharray=l.stroke_dasharray
                        opacity=l.opacity
                    />
                }
            })
            .collect();

        let rects_view: Vec<_> = graph
            .rects
            .into_iter()
            .map(|r| {
                view! {
                    <rect
                        x=r.x
                        y=r.y
                        width=r.w
                        height=r.h
                        rx=r.rx
                        ry=r.rx
                        fill=r.fill
                        fill-opacity=r.fill_opacity
                        stroke=r.stroke
                        stroke-width=r.stroke_width
                        stroke-dasharray=r.stroke_dasharray
                    />
                }
            })
            .collect();

        let paths_view: Vec<_> = graph
            .paths
            .into_iter()
            .map(|p| {
                view! {
                    <path
                        d=p.d
                        fill=p.fill
                        fill-opacity=p.fill_opacity
                        stroke=p.stroke
                        stroke-width=p.stroke_width
                        stroke-dasharray=p.stroke_dasharray
                        stroke-linecap=p.linecap
                        stroke-linejoin=p.linejoin
                    />
                }
            })
            .collect();

        let circles_view: Vec<_> = graph
            .circles
            .into_iter()
            .map(|c| {
                view! {
                    <circle
                        cx=c.cx
                        cy=c.cy
                        r=c.r
                        fill=c.fill
                        stroke=c.stroke
                        stroke-width=c.stroke_width
                    />
                }
            })
            .collect();

        let texts_view: Vec<_> = graph
            .texts
            .into_iter()
            .map(|t| {
                let weight = if t.bold { "600" } else { "400" };
                view! {
                    <text
                        x=t.x
                        y=t.y
                        fill=t.fill
                        font-size=t.font_size
                        font-family=FONT_STACK
                        font-weight=weight
                        text-anchor=t.anchor
                        opacity=t.opacity
                        pointer-events="none"
                        style="user-select: none; -webkit-user-select: none;"
                    >
                        {t.text}
                    </text>
                }
            })
            .collect();

        let body = view! {
            <g opacity=opacity>
                {card_view}
                {lines_view}
                {rects_view}
                {paths_view}
                {circles_view}
                {texts_view}
            </g>
        }
        .into_view();

        let rotated = if self.data.rotation == 0.0 {
            body
        } else {
            let cx = self.data.world_point.x + self.data.width / 2.0;
            let cy = self.data.world_point.y + self.data.height / 2.0;
            let deg = self.data.rotation.to_degrees();
            view! {
                <g transform=format!("rotate({} {} {})", deg, cx, cy)>
                    {body}
                </g>
            }
            .into_view()
        };

        if !show_label {
            return rotated;
        }
        let Some(label) = self.data.embedded_text.as_ref() else {
            return rotated;
        };
        let Some(layout) = super::embedded_text::layout_rectangle(&self.data) else {
            return rotated;
        };
        let text = super::embedded_text::render_label(label, &layout, opacity);
        view! { <g>{rotated}{text}</g> }.into_view()
    }
}

fn split_tabular_row(line: &str) -> Vec<&str> {
    if line.contains('|') {
        let trimmed = line.trim_matches('|');
        return trimmed.split('|').collect();
    }
    if line.contains('\t') {
        return line.split('\t').collect();
    }
    if line.contains(';') {
        return line.split(';').collect();
    }
    if line.contains(',') {
        return line.split(',').collect();
    }
    if line.contains(':') {
        return line.splitn(2, ':').collect();
    }
    Vec::new()
}

fn parse_numeric_cell(raw: &str) -> Option<f64> {
    let cleaned: String = raw
        .chars()
        .filter(|c| !matches!(c, '$' | '€' | '£' | '%' | ',' | ' ' | '_'))
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    cleaned.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Build an SVG path for a donut ring sector from `start_angle` to `end_angle` (in radians).
fn donut_slice_path(
    cx: f64,
    cy: f64,
    r_out: f64,
    r_in: f64,
    start_angle: f64,
    end_angle: f64,
) -> String {
    let sweep = (end_angle - start_angle).clamp(0.0, TAU);
    // Full circle (single slice or 100% share): split into two half-annuli so
    // the SVG arc command does not degenerate to a zero-length path.
    if sweep >= TAU - 1e-4 {
        let mid = start_angle + std::f64::consts::PI;
        return format!(
            "{} {}",
            donut_arc_segment(cx, cy, r_out, r_in, start_angle, mid),
            donut_arc_segment(cx, cy, r_out, r_in, mid, start_angle + TAU)
        );
    }
    donut_arc_segment(cx, cy, r_out, r_in, start_angle, end_angle)
}

fn donut_arc_segment(
    cx: f64,
    cy: f64,
    r_out: f64,
    r_in: f64,
    start_angle: f64,
    end_angle: f64,
) -> String {
    let (s_sin, s_cos) = start_angle.sin_cos();
    let (e_sin, e_cos) = end_angle.sin_cos();
    let x1 = cx + r_out * s_cos;
    let y1 = cy + r_out * s_sin;
    let x2 = cx + r_out * e_cos;
    let y2 = cy + r_out * e_sin;
    let x3 = cx + r_in * e_cos;
    let y3 = cy + r_in * e_sin;
    let x4 = cx + r_in * s_cos;
    let y4 = cy + r_in * s_sin;
    let large_arc = if end_angle - start_angle > std::f64::consts::PI {
        1
    } else {
        0
    };
    format!(
        "M{:.2} {:.2} A{:.2} {:.2} 0 {} 1 {:.2} {:.2} L{:.2} {:.2} A{:.2} {:.2} 0 {} 0 {:.2} {:.2} Z",
        x1, y1, r_out, r_out, large_arc, x2, y2, x3, y3, r_in, r_in, large_arc, x4, y4
    )
}

impl FromDrag for Chart {
    fn from_drag(anchor: Point, current: Point, color: Color, shift: bool) -> Self {
        let (pt, w, h) = rect_from_drag(anchor, current, shift);
        Self {
            data: ElementData {
                world_point: pt,
                width: w.max(MIN_CHART_SIZE),
                height: h.max(MIN_CHART_SIZE),
                style: super::ElementStyle {
                    stroke_color: color,
                    edge_style: EdgeStyle::Rounded,
                    roundness: 6.0,
                    ..Default::default()
                },
                ..ElementData::new(0)
            },
            kind: ChartKind::Bar,
            palette: ChartPalette::Ocean,
            title: String::new(),
            items: default_chart_items(),
            show_grid: true,
            show_labels: true,
            show_values: true,
            show_legend: false,
        }
    }
}

impl UpdateDrag for Chart {
    fn update_drag(&mut self, current: Point, anchor: Point, shift: bool, _zoom: f64) {
        let (pt, w, h) = rect_from_drag(anchor, current, shift);
        self.data.world_point.set(pt.x, pt.y);
        self.data.width = w.max(MIN_CHART_SIZE);
        self.data.height = h.max(MIN_CHART_SIZE);
    }
}

impl Render for Chart {
    fn render(&self, _zoom: f64) -> leptos::View {
        self.render_inner(true)
    }

    fn render_label_mask(&self) -> leptos::View {
        self.render_inner(false)
    }
}

impl HitTest for Chart {
    fn hit_test(&self, point: Point, margin: f64) -> bool {
        let cx = self.data.world_point.x + self.data.width / 2.0;
        let cy = self.data.world_point.y + self.data.height / 2.0;
        let pt = Point::unrotate(point, cx, cy, self.data.rotation);
        pt.x >= self.data.world_point.x - margin
            && pt.x <= self.data.world_point.x + self.data.width + margin
            && pt.y >= self.data.world_point.y - margin
            && pt.y <= self.data.world_point.y + self.data.height + margin
    }
}

impl Bounds for Chart {
    fn bounds(&self) -> Rect {
        Rect::new(
            self.data.world_point.x,
            self.data.world_point.y,
            self.data.width,
            self.data.height,
        )
    }
}

impl Offset for Chart {
    fn offset(&mut self, dx: f64, dy: f64) {
        self.data.world_point.offset(dx, dy);
    }
}

impl Rotate for Chart {
    fn rotate_around(&mut self, point: Point, delta: f64) {
        rotate_bbox(&mut self.data, point, delta);
    }
}

impl Resize for Chart {
    fn resize(&mut self, ctx: &ResizeContext) {
        resize_box(&mut self.data, ctx);
        self.data.width = self.data.width.max(MIN_CHART_SIZE);
        self.data.height = self.data.height.max(MIN_CHART_SIZE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_from_drag_has_default_items_and_minimum_dimensions() {
        let chart = Chart::from_drag(
            Point::new(10.0, 20.0),
            Point::new(15.0, 25.0),
            Color::new(Color::WHITE),
            false,
        );
        assert_eq!(chart.items.len(), 4);
        assert!(chart.data.width >= MIN_CHART_SIZE);
        assert!(chart.data.height >= MIN_CHART_SIZE);
        assert_eq!(chart.kind, ChartKind::Bar);
    }

    #[test]
    fn parse_tabular_csv_with_header() {
        let csv = "Quarter,Revenue\nQ1,120\nQ2,240.5\nQ3,$310";
        let (title, items) = Chart::parse_tabular_data(csv).expect("should parse CSV");
        assert_eq!(title.as_deref(), Some("Revenue"));
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], ChartItem::new("Q1", 120.0));
        assert_eq!(items[1], ChartItem::new("Q2", 240.5));
        assert_eq!(items[2], ChartItem::new("Q3", 310.0));
    }

    #[test]
    fn parse_tabular_markdown_table_and_tsv() {
        let md = "| Metric | Score |\n| --- | --- |\n| Speed | 88 |\n| Memory | 94 |";
        let (title, items) = Chart::parse_tabular_data(md).expect("should parse markdown table");
        assert_eq!(title.as_deref(), Some("Score"));
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], ChartItem::new("Speed", 88.0));
        assert_eq!(items[1], ChartItem::new("Memory", 94.0));

        let tsv = "Alpha\t45\nBeta\t65";
        let (title_tsv, items_tsv) = Chart::parse_tabular_data(tsv).expect("should parse TSV");
        assert_eq!(title_tsv, None);
        assert_eq!(items_tsv.len(), 2);
    }

    #[test]
    fn value_handles_match_item_count_and_rotate_with_chart() {
        let mut chart = Chart::new_at(
            Point::new(200.0, 200.0),
            ChartKind::Bar,
            Color::new(Color::WHITE),
        );
        let unrotated = chart.value_handles();
        assert_eq!(unrotated.len(), 4);

        chart.data.rotation = FRAC_PI_2;
        let rotated = chart.value_handles();
        assert_eq!(rotated.len(), 4);
        assert!((unrotated[0].1.x - rotated[0].1.x).abs() > 1.0);
    }

    #[test]
    fn dragging_value_handle_updates_item_value() {
        let mut chart = Chart::new_at(
            Point::new(200.0, 200.0),
            ChartKind::Bar,
            Color::new(Color::WHITE),
        );
        let base_max = chart.scale_max();
        let metrics = chart.plot_metrics();
        // Dragging to the top of the plot area sets value to base_max
        let top_pt = Point::new(metrics.plot.x + 20.0, metrics.plot.y);
        chart.set_value_from_world(0, top_pt, base_max, chart.items[0].value, top_pt);
        assert!((chart.items[0].value - base_max.round()).abs() < 1e-6);

        // Dragging to the bottom of the plot area sets value to 0
        let bottom_pt = Point::new(metrics.plot.x + 20.0, metrics.plot.y + metrics.plot.h);
        chart.set_value_from_world(0, bottom_pt, base_max, chart.items[0].value, top_pt);
        assert_eq!(chart.items[0].value, 0.0);
    }

    #[test]
    fn scene_graph_builds_primitives_for_every_chart_kind() {
        for kind in ChartKind::ALL {
            let mut chart =
                Chart::new_at(Point::new(150.0, 150.0), kind, Color::new(Color::WHITE));
            chart.title = "Quarterly Overview".to_string();
            chart.show_legend = true;
            let graph = chart.scene_graph();
            match kind {
                ChartKind::Bar => assert_eq!(graph.rects.len(), chart.items.len()),
                ChartKind::Line => {
                    assert_eq!(graph.paths.len(), 1);
                    assert!(graph.circles.len() >= chart.items.len());
                }
                ChartKind::Area | ChartKind::Sparkline => {
                    assert_eq!(graph.paths.len(), 2);
                }
                ChartKind::Donut => {
                    assert_eq!(graph.paths.len(), chart.items.len());
                }
            }
        }
    }
}
