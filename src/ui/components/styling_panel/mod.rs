mod alignment;
mod arrow;
mod chart;
mod color;
mod connector;
mod edit;
mod fill;
mod float_picker;
mod frame;
mod group;
mod layers;
mod sliders;
mod style;
mod utils;

use self::alignment::alignment_section;
use self::arrow::arrow_section;
use self::chart::chart_section;
use self::color::{color_section, embedded_text_color_section, ColorSlot};
use self::connector::connector_section;
use self::edit::edit_section;
use self::fill::fill_section;
use self::float_picker::float_picker;
use self::frame::frame_section;
use self::group::group_section;
use self::layers::layers_section;
use self::sliders::sliders_section;
use self::style::style_section;
use self::utils::{
    read_panel_state, selected_chart_style, selected_connector_variant, selected_frame_style,
    selected_text_align, single_selected_element, uniform_icon_seg,
};
use crate::canvas::settings::CanvasSettings;
use crate::canvas::{CanvasMode, Viewport};
use crate::edit_ops;
use crate::edit_ops::Clipboard;
use crate::model::{
    Bounds, ChartKind, ChartPalette, ChartStyle, Color, Connector, ConnectorVariant, CurveMode,
    EdgeStyle, Element, ElementId, ElementStyle, FrameOrientation, FramePreset, FrameStyle,
    LineStyle, Point, Scene, StrokeStyle, StylingKind, TextAlign,
};
use crate::ui::dock::Tool;
use crate::ui::{icon, styles};
use crate::util::window_size;
use leptos::*;
use leptos_color::Color as LpcColor;
use std::rc::Rc;
use wasm_bindgen::JsCast;

const ALIGN_OPTS: &[(TextAlign, &str)] = &[
    (TextAlign::Left, "Align left"),
    (TextAlign::Center, "Align center"),
    (TextAlign::Right, "Align right"),
];

fn text_align_icon(align: TextAlign) -> View {
    match align {
        TextAlign::Left => icon::text_align_left().into_view(),
        TextAlign::Center => icon::text_align_center().into_view(),
        TextAlign::Right => icon::text_align_right().into_view(),
    }
}

/// Style mutation applied to the selected elements, or to the default style
/// when the selection is empty.
type UpdateStyle = Rc<dyn Fn(&dyn Fn(&mut ElementStyle))>;

/// The same for freehand line styles.
type UpdateLineStyle = Rc<dyn Fn(&dyn Fn(&mut LineStyle))>;

/// The same for chart styles.
type UpdateChartStyle = Rc<dyn Fn(&dyn Fn(&mut ChartStyle))>;

/// The same for frame styles.
type UpdateFrameStyle = Rc<dyn Fn(&dyn Fn(&mut FrameStyle))>;

#[derive(Clone)]
pub struct StylingPanelContext {
    pub scene: RwSignal<Scene>,
    pub selected_ids: RwSignal<Vec<ElementId>>,
    pub selected_tool: RwSignal<Tool>,
    pub default_style: RwSignal<ElementStyle>,
    pub default_line_style: RwSignal<LineStyle>,
    pub default_connector_variant: RwSignal<ConnectorVariant>,
    pub default_chart_style: RwSignal<ChartStyle>,
    pub default_frame_style: RwSignal<FrameStyle>,
    pub chart_editing_id: RwSignal<Option<ElementId>>,
    pub viewport: RwSignal<Viewport>,
    pub clipboard: RwSignal<Clipboard>,
    pub push_snapshot: Rc<dyn Fn()>,
    pub canvas_mode: RwSignal<CanvasMode>,
    pub cursor_world: RwSignal<Point>,
    pub settings: RwSignal<CanvasSettings>,
    pub on_save_snippet: Rc<dyn Fn()>,
}

fn mutate_selected_elements(
    scene: RwSignal<Scene>,
    ids: &[ElementId],
    push_snapshot: &Rc<dyn Fn()>,
    update: impl Fn(&mut Element),
) {
    edit_ops::mutate_scene(scene, push_snapshot, |scene| {
        for id in ids {
            if let Some(mut element) = scene.element_by_id_mut(*id) {
                update(&mut element);
            }
        }
    });
}

#[component]
pub fn StylingPanel(context: StylingPanelContext) -> impl IntoView {
    let StylingPanelContext {
        scene,
        selected_ids,
        selected_tool,
        default_style,
        default_line_style,
        default_connector_variant,
        default_chart_style,
        default_frame_style,
        chart_editing_id,
        viewport,
        clipboard,
        push_snapshot,
        canvas_mode,
        cursor_world,
        settings,
        on_save_snippet,
    } = context;
    let styling_kind = Signal::derive(move || {
        let ids = selected_ids.get();
        let el =
            leptos::untrack(|| scene.with_untracked(|scene| single_selected_element(scene, &ids)));
        if let Some(el) = el {
            return el.styling_kind();
        }
        selected_tool.get().styling_kind()
    });

    let is_text = Signal::derive(move || styling_kind.get() == StylingKind::Text);
    let is_line_or_arrow = Signal::derive(move || {
        matches!(styling_kind.get(), StylingKind::Line | StylingKind::Arrow)
    });
    let is_connector = Signal::derive(move || styling_kind.get() == StylingKind::Connector);
    let is_arrow = Signal::derive(move || {
        matches!(
            styling_kind.get(),
            StylingKind::Arrow | StylingKind::Connector
        )
    });
    let is_chart = Signal::derive(move || styling_kind.get() == StylingKind::Chart);
    let is_frame = Signal::derive(move || styling_kind.get() == StylingKind::Frame);
    let is_rect = Signal::derive(move || {
        matches!(
            styling_kind.get(),
            StylingKind::Rectangle | StylingKind::Chart | StylingKind::Frame
        )
    });
    let is_freehand = Signal::derive(move || styling_kind.get() == StylingKind::Freehand);
    let is_image = Signal::derive(move || styling_kind.get() == StylingKind::Image);
    // Images keep only the opacity slider; stroke and fill do nothing to them.
    let hide_stroke = Signal::derive(move || is_text.get() || is_image.get());
    let base_kind = Signal::derive(move || {
        !is_text.get()
            && !is_line_or_arrow.get()
            && !is_connector.get()
            && !is_freehand.get()
            && !is_image.get()
    });
    let has_single_selected_chart = Signal::derive(move || {
        let ids = selected_ids.get();
        ids.len() == 1
            && scene.with(|s| matches!(s.element_by_id(ids[0]), Some(Element::Chart(_))))
    });
    let has_single_selected_frame = Signal::derive(move || {
        let ids = selected_ids.get();
        ids.len() == 1
            && scene.with(|s| matches!(s.element_by_id(ids[0]), Some(Element::Frame(_))))
    });

    let (init_style, init_line_style) = scene.with_untracked(|scene| {
        read_panel_state(
            scene,
            &selected_ids.get_untracked(),
            &default_style.get_untracked(),
            &default_line_style.get_untracked(),
            selected_tool.get_untracked(),
        )
    });

    let local_style = create_rw_signal(init_style);
    let local_line_style = create_rw_signal(init_line_style);
    let local_embedded_text_color = create_rw_signal(Color::default());
    let has_embedded_text = Signal::derive(move || {
        let ids = selected_ids.get();
        if ids.len() != 1 {
            return false;
        }
        scene.with(|scene| {
            scene
                .element_by_id(ids[0])
                .is_some_and(|element| element.data().embedded_text.is_some())
        })
    });
    let local_connector_variant = create_rw_signal(scene.with_untracked(|scene| {
        selected_connector_variant(
            scene,
            &selected_ids.get_untracked(),
            default_connector_variant.get_untracked(),
        )
    }));
    let local_chart_style = create_rw_signal(scene.with_untracked(|scene| {
        selected_chart_style(
            scene,
            &selected_ids.get_untracked(),
            default_chart_style.get_untracked(),
        )
    }));
    let local_frame_style = create_rw_signal(scene.with_untracked(|scene| {
        selected_frame_style(
            scene,
            &selected_ids.get_untracked(),
            default_frame_style.get_untracked(),
        )
    }));

    let stroke_size = create_rw_signal(local_style.get_untracked().stroke_width);
    let text_size = create_rw_signal(local_style.get_untracked().font_size);
    let text_align = create_rw_signal(
        scene.with_untracked(|scene| selected_text_align(scene, &selected_ids.get_untracked())),
    );
    let roundness = create_rw_signal(local_style.get_untracked().roundness);
    let opacity = create_rw_signal(local_style.get_untracked().opacity * 100.0);
    let is_rounded = Signal::derive(move || local_style.get().edge_style == EdgeStyle::Rounded);

    let active_slot = create_rw_signal(None::<ColorSlot>);
    let picker_ref: NodeRef<html::Div> = create_node_ref();

    create_effect(move |_| {
        if active_slot.get().is_some() {
            let handle =
                window_event_listener(leptos::ev::click, move |ev: web_sys::MouseEvent| {
                    if let Some(el) = picker_ref.get() {
                        if let Some(target) = ev.target() {
                            let target: web_sys::Node = target.unchecked_into();
                            if !el.contains(Some(&target)) {
                                active_slot.set(None);
                            }
                        }
                    }
                });
            on_cleanup(move || handle.remove());
        }
    });

    create_effect(move |_| {
        let _ids = selected_ids.get();
        // Track scene changes without cloning the document (this effect
        // re-runs on every drag step).
        scene.with(|_| ());
        let _dls = default_line_style.get();
        let _dcv = default_connector_variant.get();
        let _dcs = default_chart_style.get();
        let _dfs = default_frame_style.get();
        let (new_style, new_line_style, align, embedded_color, variant, chart_style, frame_style) =
            leptos::untrack(|| {
                scene.with_untracked(|scene| {
                    let ids = selected_ids.get_untracked();
                    let (style, line_style) = read_panel_state(
                        scene,
                        &ids,
                        &default_style.get_untracked(),
                        &default_line_style.get_untracked(),
                        selected_tool.get_untracked(),
                    );
                    let align = selected_text_align(scene, &ids);
                    let embedded_color = ids
                        .first()
                        .and_then(|id| scene.element_by_id(*id))
                        .and_then(|element| {
                            element.data().embedded_text.as_ref().map(|text| text.color)
                        });
                    let variant = selected_connector_variant(
                        scene,
                        &ids,
                        default_connector_variant.get_untracked(),
                    );
                    let chart_style = selected_chart_style(
                        scene,
                        &ids,
                        default_chart_style.get_untracked(),
                    );
                    let frame_style = selected_frame_style(
                        scene,
                        &ids,
                        default_frame_style.get_untracked(),
                    );
                    (
                        style,
                        line_style,
                        align,
                        embedded_color,
                        variant,
                        chart_style,
                        frame_style,
                    )
                })
            });
        stroke_size.set(new_style.stroke_width);
        text_size.set(new_style.font_size);
        text_align.set(align);
        roundness.set(new_style.roundness);
        opacity.set(new_style.opacity * 100.0);
        local_style.set(new_style);
        local_line_style.set(new_line_style);
        local_embedded_text_color.set(embedded_color.unwrap_or_default());
        local_connector_variant.set(variant);
        local_chart_style.set(chart_style);
        local_frame_style.set(frame_style);
    });

    let update_style: UpdateStyle = Rc::new({
        let scene = scene;
        let selected_ids = selected_ids;
        let default_style = default_style;
        let push_snapshot = push_snapshot.clone();
        move |f: &dyn Fn(&mut ElementStyle)| {
            let ids = selected_ids.get_untracked();
            if !ids.is_empty() {
                mutate_selected_elements(scene, &ids, &push_snapshot, |element| {
                    f(&mut element.data_mut().style);
                });
            } else {
                default_style.update(|s| f(s));
            }
        }
    });

    let update_line_style: UpdateLineStyle = Rc::new({
        let scene = scene;
        let selected_ids = selected_ids;
        let default_line_style = default_line_style;
        let push_snapshot = push_snapshot.clone();
        move |f: &dyn Fn(&mut LineStyle)| {
            let ids = selected_ids.get_untracked();
            if !ids.is_empty() {
                mutate_selected_elements(scene, &ids, &push_snapshot, |element| match element {
                    Element::Line(line) => f(&mut line.line_style),
                    Element::Connector(connector) => f(&mut connector.line_style),
                    _ => {}
                });
            } else {
                default_line_style.update(|ls| f(ls));
            }
        }
    });

    let set_color: Rc<dyn Fn(Color)> = Rc::new({
        let local_style = local_style;
        let update_style = update_style.clone();
        move |c| {
            local_style.update(|s| s.stroke_color = c);
            update_style(&|s| s.stroke_color = c);
        }
    });

    let set_fill: Rc<dyn Fn(Option<Color>)> = Rc::new({
        let local_style = local_style;
        let update_style = update_style.clone();
        move |c| {
            local_style.update(|s| s.fill_color = c);
            update_style(&|s| s.fill_color = c);
        }
    });

    let set_embedded_text_color: Rc<dyn Fn(Color)> = Rc::new({
        let local_color = local_embedded_text_color;
        let scene = scene;
        let selected_ids = selected_ids;
        let push_snapshot = push_snapshot.clone();
        move |color| {
            local_color.set(color);
            let ids = selected_ids.get_untracked();
            if !ids.is_empty() {
                mutate_selected_elements(scene, &ids, &push_snapshot, |element| {
                    if let Some(label) = &mut element.data_mut().embedded_text {
                        label.color = color;
                    }
                });
            }
        }
    });

    let picker_set_color = set_color.clone();
    let picker_set_fill = set_fill.clone();
    let picker_set_embedded_text_color = set_embedded_text_color.clone();

    let picker_color = Signal::derive(move || {
        let hex_str = match active_slot.get() {
            Some(ColorSlot::Stroke) => local_style.get().stroke_color.to_hex(),
            Some(ColorSlot::Fill) => local_style
                .get()
                .fill_color
                .as_ref()
                .map_or_else(|| Color::new(Color::WHITE).to_hex(), |c| c.to_hex()),
            Some(ColorSlot::EmbeddedText) => local_embedded_text_color.get().to_hex(),
            _ => String::new(),
        };
        LpcColor::from_html(&hex_str).unwrap_or_default()
    });

    let picker_on_change = Callback::new(move |lc: LpcColor| {
        let hex = lc.to_css_hex();
        let c = if hex.len() == 9 {
            Color::new(&hex[..7])
        } else {
            Color::new(&hex)
        };
        match active_slot.get() {
            Some(ColorSlot::Stroke) => picker_set_color(c),
            Some(ColorSlot::Fill) => picker_set_fill(Some(c)),
            Some(ColorSlot::EmbeddedText) => picker_set_embedded_text_color(c),
            None => {}
        }
    });

    let set_stroke_style: Rc<dyn Fn(StrokeStyle)> = Rc::new({
        let local_style = local_style;
        let update_style = update_style.clone();
        move |v| {
            local_style.update(|s| s.stroke_style = v);
            update_style(&|s| s.stroke_style = v);
        }
    });

    let set_edge_style: Rc<dyn Fn(EdgeStyle)> = Rc::new({
        let local_style = local_style;
        let update_style = update_style.clone();
        move |v| {
            local_style.update(|s| s.edge_style = v);
            update_style(&|s| s.edge_style = v);
        }
    });

    let set_curve_mode: Rc<dyn Fn(CurveMode)> = Rc::new({
        let local_line_style = local_line_style;
        let update_line_style = update_line_style.clone();
        move |v| {
            local_line_style.update(|ls| ls.curve_mode = v);
            update_line_style(&|ls| ls.curve_mode = v);
        }
    });

    let set_connector_variant: Rc<dyn Fn(ConnectorVariant)> = Rc::new({
        let local_connector_variant = local_connector_variant;
        let default_connector_variant = default_connector_variant;
        let scene = scene;
        let selected_ids = selected_ids;
        let push_snapshot = push_snapshot.clone();
        move |variant| {
            local_connector_variant.set(variant);
            let ids = selected_ids.get_untracked();
            if !ids.is_empty() {
                mutate_selected_elements(scene, &ids, &push_snapshot, |element| match element {
                    Element::Connector(connector) => connector.variant = variant,
                    // Older arrows are plain lines. Routing is meaningless for
                    // a line, so choosing a route promotes it to a connector.
                    Element::Line(line) => {
                        *element = Element::Connector(Connector::from_line(
                            line.clone(),
                            variant,
                            None,
                            None,
                        ));
                    }
                    _ => {}
                });
            } else {
                default_connector_variant.set(variant);
            }
        }
    });

    let set_start_arrowhead: Rc<dyn Fn()> = Rc::new({
        let local_line_style = local_line_style;
        let update_line_style = update_line_style.clone();
        move || {
            let new = !local_line_style.get_untracked().has_start_arrowhead;
            local_line_style.update(|ls| ls.has_start_arrowhead = new);
            update_line_style(&|ls| ls.has_start_arrowhead = new);
        }
    });

    let set_end_arrowhead: Rc<dyn Fn()> = Rc::new({
        let local_line_style = local_line_style;
        let update_line_style = update_line_style.clone();
        move || {
            let new = !local_line_style.get_untracked().has_end_arrowhead;
            local_line_style.update(|ls| ls.has_end_arrowhead = new);
            update_line_style(&|ls| ls.has_end_arrowhead = new);
        }
    });

    let stroke_size_cb: Rc<dyn Fn(f64)> = Rc::new({
        let update_style = update_style.clone();
        move |val| update_style(&|style| style.stroke_width = val)
    });

    let text_size_cb: Rc<dyn Fn(f64)> = Rc::new({
        let scene = scene;
        let selected_ids = selected_ids;
        let default_style = default_style;
        let push_snapshot = push_snapshot.clone();
        move |val| {
            let ids = selected_ids.get_untracked();
            if !ids.is_empty() {
                mutate_selected_elements(scene, &ids, &push_snapshot, |element| {
                    element.data_mut().style.font_size = val;
                    if let Element::Text(text) = element {
                        text.font_changed();
                    }
                });
            } else {
                default_style.update(|s| s.font_size = val);
            }
        }
    });

    let set_text_align: Rc<dyn Fn(TextAlign)> = Rc::new({
        let scene = scene;
        let selected_ids = selected_ids;
        let push_snapshot = push_snapshot.clone();
        move |val| {
            let ids = selected_ids.get_untracked();
            if ids.is_empty() {
                return;
            }
            let changed = scene.with_untracked(|s| {
                ids.iter().any(
                    |id| matches!(s.element_by_id(*id), Some(Element::Text(t)) if t.align != val),
                )
            });
            if !changed {
                return;
            }
            mutate_selected_elements(scene, &ids, &push_snapshot, |element| {
                if let Element::Text(text) = element {
                    text.set_alignment(val);
                }
            });
        }
    });

    let roundness_cb: Rc<dyn Fn(f64)> = Rc::new({
        let local_style = local_style;
        let update_style = update_style.clone();
        move |val| {
            local_style.update(|s| s.roundness = val);
            update_style(&|s| s.roundness = val);
        }
    });

    let opacity_cb: Rc<dyn Fn(f64)> = Rc::new({
        let update_style = update_style.clone();
        move |val| {
            let v = (val / 100.0).clamp(0.0, 1.0);
            update_style(&|style| style.opacity = v);
        }
    });

    let update_chart_style: UpdateChartStyle = Rc::new({
        let local_chart_style = local_chart_style;
        let default_chart_style = default_chart_style;
        let scene = scene;
        let selected_ids = selected_ids;
        let push_snapshot = push_snapshot.clone();
        move |f: &dyn Fn(&mut ChartStyle)| {
            local_chart_style.update(|cs| f(cs));
            let ids = selected_ids.get_untracked();
            if !ids.is_empty() {
                edit_ops::mutate_scene(scene, &push_snapshot, |scene| {
                    for id in &ids {
                        if let Some(mut element) = scene.element_by_id_mut(*id) {
                            if let Element::Chart(chart) = &mut *element {
                                let mut cs = chart.chart_style();
                                f(&mut cs);
                                chart.apply_chart_style(&cs);
                            }
                        }
                        scene.refresh_connectors_for(*id);
                    }
                });
            } else {
                default_chart_style.update(|cs| f(cs));
            }
        }
    });

    let set_chart_kind: Rc<dyn Fn(ChartKind)> = Rc::new({
        let update_chart_style = update_chart_style.clone();
        move |kind| update_chart_style(&|cs| cs.kind = kind)
    });

    let set_chart_palette: Rc<dyn Fn(ChartPalette)> = Rc::new({
        let update_chart_style = update_chart_style.clone();
        move |pal| update_chart_style(&|cs| cs.palette = pal)
    });

    let toggle_chart_grid: Rc<dyn Fn()> = Rc::new({
        let update_chart_style = update_chart_style.clone();
        move || update_chart_style(&|cs| cs.show_grid = !cs.show_grid)
    });

    let toggle_chart_labels: Rc<dyn Fn()> = Rc::new({
        let update_chart_style = update_chart_style.clone();
        move || update_chart_style(&|cs| cs.show_labels = !cs.show_labels)
    });

    let toggle_chart_values: Rc<dyn Fn()> = Rc::new({
        let update_chart_style = update_chart_style.clone();
        move || update_chart_style(&|cs| cs.show_values = !cs.show_values)
    });

    let toggle_chart_legend: Rc<dyn Fn()> = Rc::new({
        let update_chart_style = update_chart_style.clone();
        move || update_chart_style(&|cs| cs.show_legend = !cs.show_legend)
    });

    let on_edit_chart_data: Rc<dyn Fn()> = Rc::new({
        let selected_ids = selected_ids;
        let chart_editing_id = chart_editing_id;
        move || {
            let ids = selected_ids.get_untracked();
            if ids.len() == 1 {
                let target = ids[0];
                if chart_editing_id.get_untracked() == Some(target) {
                    chart_editing_id.set(None);
                } else {
                    chart_editing_id.set(Some(target));
                }
            }
        }
    });

    let update_frame_style: UpdateFrameStyle = Rc::new({
        let local_frame_style = local_frame_style;
        let default_frame_style = default_frame_style;
        let scene = scene;
        let selected_ids = selected_ids;
        let push_snapshot = push_snapshot.clone();
        move |f: &dyn Fn(&mut FrameStyle)| {
            local_frame_style.update(|fs| f(fs));
            let ids = selected_ids.get_untracked();
            if !ids.is_empty() {
                edit_ops::mutate_scene(scene, &push_snapshot, |scene| {
                    for id in &ids {
                        if let Some(mut element) = scene.element_by_id_mut(*id) {
                            if let Element::Frame(frame) = &mut *element {
                                let mut fs = FrameStyle {
                                    preset: frame.preset,
                                    orientation: frame.orientation,
                                };
                                f(&mut fs);
                                frame.apply_frame_style(&fs);
                            }
                        }
                        scene.refresh_connectors_for(*id);
                    }
                });
            } else {
                default_frame_style.update(|fs| f(fs));
            }
        }
    });

    let set_frame_preset: Rc<dyn Fn(FramePreset)> = Rc::new({
        let update_frame_style = update_frame_style.clone();
        move |preset| update_frame_style(&|fs| fs.preset = preset)
    });

    let set_frame_orientation: Rc<dyn Fn(FrameOrientation)> = Rc::new({
        let update_frame_style = update_frame_style.clone();
        move |orient| update_frame_style(&|fs| fs.orientation = orient)
    });

    let frame_dimensions_label = Signal::derive(move || {
        let ids = selected_ids.get();
        if ids.len() == 1 {
            if let Some(summary) = scene.with(|s| match s.element_by_id(ids[0]) {
                Some(Element::Frame(f)) => Some(f.dimensions_summary()),
                _ => None,
            }) {
                return summary;
            }
        }
        let fs = local_frame_style.get();
        if let Some((w_px, h_px)) = fs.preset.size_px(fs.orientation) {
            let w_cm = crate::model::elements::frame::px_to_cm(w_px);
            let h_cm = crate::model::elements::frame::px_to_cm(h_px);
            let w_in = crate::model::elements::frame::px_to_in(w_px);
            let h_in = crate::model::elements::frame::px_to_in(h_px);
            format!(
                "{:.1} × {:.1} cm  ({:.1} × {:.1} in · {:.0} × {:.0} px)",
                w_cm, h_cm, w_in, h_in, w_px, h_px
            )
        } else {
            "Drag on canvas to create a custom-sized frame".to_string()
        }
    });

    let on_focus_frame: Rc<dyn Fn()> = Rc::new({
        let scene = scene;
        let selected_ids = selected_ids;
        let viewport = viewport;
        move || {
            let ids = selected_ids.get_untracked();
            if let Some(&id) = ids.first() {
                if let Some(bounds) = scene.with_untracked(|s| match s.element_by_id(id) {
                    Some(Element::Frame(f)) => Some(f.bounds()),
                    _ => None,
                }) {
                    let screen = window_size();
                    viewport.update(|vp| vp.fit_rect(bounds, screen, 72.0, 0.1, 20.0));
                }
            }
        }
    });

    let color_swatches: &[&str] = &[
        Color::WHITE,
        Color::YELLOW,
        Color::GREEN,
        Color::BLUE,
        Color::RED,
    ];

    view! {
        <div class="flex flex-col gap-4 select-none">
            {frame_section(
                is_frame,
                has_single_selected_frame,
                local_frame_style,
                frame_dimensions_label,
                set_frame_preset,
                set_frame_orientation,
                on_focus_frame,
            )}
            {chart_section(
                is_chart,
                has_single_selected_chart,
                local_chart_style,
                set_chart_kind,
                set_chart_palette,
                toggle_chart_grid,
                toggle_chart_labels,
                toggle_chart_values,
                toggle_chart_legend,
                on_edit_chart_data,
            )}
            {sliders_section(
                stroke_size,
                stroke_size_cb,
                opacity,
                opacity_cb,
                text_size,
                text_size_cb,
                styling_kind,
            )} <div class="flex flex-col gap-1.5 mt-1" class:hidden=move || !is_text.get()>
                <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                    Alignment
                </div>
                {uniform_icon_seg(
                    ALIGN_OPTS,
                    Signal::derive(move || text_align.get()),
                    set_text_align,
                    text_align_icon,
                )}
            </div>
            {style_section(
                local_style,
                local_line_style,
                set_stroke_style,
                set_curve_mode,
                set_edge_style,
                roundness,
                roundness_cb,
                hide_stroke,
                is_line_or_arrow,
                is_rect,
                is_rounded,
            )} {connector_section(is_connector, local_connector_variant, set_connector_variant)}
            {color_section(local_style, set_color, active_slot, color_swatches, hide_stroke)}
            {embedded_text_color_section(
                local_embedded_text_color,
                set_embedded_text_color,
                active_slot,
                color_swatches,
                has_embedded_text,
            )} {fill_section(local_style, set_fill, active_slot, base_kind, color_swatches)}
            {arrow_section(is_arrow, local_line_style, set_start_arrowhead, set_end_arrowhead)}
            {edit_section(
                scene,
                selected_ids,
                clipboard,
                push_snapshot.clone(),
                canvas_mode,
                cursor_world,
            )} {group_section(scene, selected_ids, canvas_mode, push_snapshot.clone())}
            {alignment_section(scene, selected_ids, canvas_mode, push_snapshot.clone())}
            {layers_section(scene, selected_ids, push_snapshot.clone())}
            <Show when=move || !selected_ids.get().is_empty()>
                <button
                    class=format!("{} w-full", styles::BTN_TEXT)
                    on:click={
                        let on_save_snippet = on_save_snippet.clone();
                        move |_| on_save_snippet()
                    }
                    title="Save the selection as a reusable snippet"
                >
                    <span class="mr-1.5">{icon::save_snippet()}</span>
                    "Save as snippet"
                </button>
            </Show>
        </div>
        {float_picker(active_slot, picker_color, picker_on_change, picker_ref, settings)}
    }
}
