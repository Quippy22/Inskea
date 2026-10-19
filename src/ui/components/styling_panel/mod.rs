mod alignment;
mod arrow;
mod color;
mod edit;
mod fill;
mod float_picker;
mod group;
mod layers;
mod sliders;
mod style;
mod utils;

use self::alignment::alignment_section;
use self::arrow::arrow_section;
use self::color::{color_section, ColorSlot};
use self::edit::edit_section;
use self::fill::fill_section;
use self::float_picker::float_picker;
use self::group::group_section;
use self::layers::layers_section;
use self::sliders::sliders_section;
use self::style::style_section;
use self::utils::{read_panel_state, selected_text_align, single_selected_element, uniform_seg};
use crate::canvas::CanvasMode;
use crate::edit_ops;
use crate::edit_ops::Clipboard;
use crate::model::{
    Color, CurveMode, EdgeStyle, Element, ElementId, ElementStyle, LineStyle, Point, Scene,
    StrokeStyle, StylingKind, TextAlign,
};
use crate::ui::dock::Tool;
use crate::ui::styles;
use leptos::*;
use leptos_color::Color as LpcColor;
use std::rc::Rc;
use wasm_bindgen::JsCast;

const ALIGN_OPTS: &[(TextAlign, &str)] = &[
    (TextAlign::Left, "Left"),
    (TextAlign::Center, "Center"),
    (TextAlign::Right, "Right"),
];

/// Style mutation applied to the selected elements, or to the default style
/// when the selection is empty.
type UpdateStyle = Rc<dyn Fn(&dyn Fn(&mut ElementStyle))>;

/// The same for freehand line styles.
type UpdateLineStyle = Rc<dyn Fn(&dyn Fn(&mut LineStyle))>;

#[component]
pub fn StylingPanel(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    selected_tool: RwSignal<Tool>,
    default_style: RwSignal<ElementStyle>,
    default_line_style: RwSignal<LineStyle>,
    clipboard: RwSignal<Clipboard>,
    push_snapshot: Rc<dyn Fn()>,
    canvas_mode: RwSignal<CanvasMode>,
    cursor_world: RwSignal<Point>,
) -> impl IntoView {
    let styling_kind = Signal::derive(move || {
        let ids = selected_ids.get();
        let el = leptos::untrack(|| single_selected_element(scene.get_untracked(), ids));
        if let Some(el) = el {
            return el.styling_kind();
        }
        selected_tool.get().styling_kind()
    });

    let is_text = Signal::derive(move || styling_kind.get() == StylingKind::Text);
    let is_line = Signal::derive(move || {
        styling_kind.get() == StylingKind::Line || styling_kind.get() == StylingKind::Arrow
    });
    let is_arrow = Signal::derive(move || styling_kind.get() == StylingKind::Arrow);
    let is_rect = Signal::derive(move || styling_kind.get() == StylingKind::Rectangle);
    let is_freehand = Signal::derive(move || styling_kind.get() == StylingKind::Freehand);
    let base_kind = Signal::derive(move || !is_text.get() && !is_line.get() && !is_freehand.get());

    let (init_style, init_line_style) = read_panel_state(
        &scene.get_untracked(),
        &selected_ids.get_untracked(),
        &default_style.get_untracked(),
        &default_line_style.get_untracked(),
        selected_tool.get_untracked(),
    );

    let local_style = create_rw_signal(init_style);
    let local_line_style = create_rw_signal(init_line_style);

    let stroke_size = create_rw_signal(local_style.get_untracked().stroke_width);
    let text_size = create_rw_signal(local_style.get_untracked().font_size);
    let text_align = create_rw_signal(selected_text_align(
        &scene.get_untracked(),
        &selected_ids.get_untracked(),
    ));
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
        let _ = scene.with(|_| ());
        let _dls = default_line_style.get();
        let (new_style, new_line_style) = leptos::untrack(|| {
            read_panel_state(
                &scene.get_untracked(),
                &selected_ids.get_untracked(),
                &default_style.get_untracked(),
                &default_line_style.get_untracked(),
                selected_tool.get_untracked(),
            )
        });
        stroke_size.set(new_style.stroke_width);
        text_size.set(new_style.font_size);
        text_align.set(selected_text_align(
            &scene.get_untracked(),
            &selected_ids.get_untracked(),
        ));
        roundness.set(new_style.roundness);
        opacity.set(new_style.opacity * 100.0);
        local_style.set(new_style);
        local_line_style.set(new_line_style);
    });

    let update_style: UpdateStyle = Rc::new({
        let scene = scene;
        let selected_ids = selected_ids;
        let default_style = default_style;
        let push_snapshot = push_snapshot.clone();
        move |f: &dyn Fn(&mut ElementStyle)| {
            let ids = selected_ids.get_untracked();
            if !ids.is_empty() {
                edit_ops::mutate_scene(scene, &push_snapshot, |s| {
                    for id in &ids {
                        if let Some(el) = s.element_by_id_mut(*id) {
                            f(&mut el.data_mut().style);
                        }
                    }
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
                edit_ops::mutate_scene(scene, &push_snapshot, |s| {
                    for id in &ids {
                        if let Some(Element::Line(l)) = s.element_by_id_mut(*id) {
                            f(&mut l.line_style);
                        }
                    }
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
            local_style.update(|s| s.stroke_color = c.clone());
            update_style(&|s| s.stroke_color = c.clone());
        }
    });

    let set_fill: Rc<dyn Fn(Option<Color>)> = Rc::new({
        let local_style = local_style;
        let update_style = update_style.clone();
        move |c| {
            local_style.update(|s| s.fill_color = c.clone());
            update_style(&|s| s.fill_color = c.clone());
        }
    });

    let picker_set_color = set_color.clone();
    let picker_set_fill = set_fill.clone();

    let picker_color = Signal::derive(move || {
        let hex_str = match active_slot.get() {
            Some(ColorSlot::Stroke) => local_style.get().stroke_color.to_hex(),
            Some(ColorSlot::Fill) => local_style
                .get()
                .fill_color
                .as_ref()
                .map_or_else(|| Color::new(Color::WHITE).to_hex(), |c| c.to_hex()),
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
        let scene = scene;
        let selected_ids = selected_ids;
        let default_style = default_style;
        let push_snapshot = push_snapshot.clone();
        move |val| {
            let ids = selected_ids.get_untracked();
            if !ids.is_empty() {
                edit_ops::mutate_scene(scene, &push_snapshot, |s| {
                    for id in &ids {
                        if let Some(el) = s.element_by_id_mut(*id) {
                            el.data_mut().style.stroke_width = val;
                        }
                    }
                });
            } else {
                default_style.update(|s| s.stroke_width = val);
            }
        }
    });

    let text_size_cb: Rc<dyn Fn(f64)> = Rc::new({
        let scene = scene;
        let selected_ids = selected_ids;
        let default_style = default_style;
        let push_snapshot = push_snapshot.clone();
        move |val| {
            let ids = selected_ids.get_untracked();
            if !ids.is_empty() {
                edit_ops::mutate_scene(scene, &push_snapshot, |s| {
                    for id in &ids {
                        if let Some(el) = s.element_by_id_mut(*id) {
                            el.data_mut().style.font_size = val;
                            if let Element::Text(t) = el {
                                t.font_changed();
                            }
                        }
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
            edit_ops::mutate_scene(scene, &push_snapshot, |s| {
                for id in &ids {
                    if let Some(Element::Text(t)) = s.element_by_id_mut(*id) {
                        t.set_alignment(val);
                    }
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
        let scene = scene;
        let selected_ids = selected_ids;
        let default_style = default_style;
        let push_snapshot = push_snapshot.clone();
        move |val| {
            let v = (val / 100.0).clamp(0.0, 1.0);
            let ids = selected_ids.get_untracked();
            if !ids.is_empty() {
                edit_ops::mutate_scene(scene, &push_snapshot, |s| {
                    for id in &ids {
                        if let Some(el) = s.element_by_id_mut(*id) {
                            el.data_mut().style.opacity = v;
                        }
                    }
                });
            } else {
                default_style.update(|s| s.opacity = v);
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
            {sliders_section(
                stroke_size,
                stroke_size_cb,
                opacity,
                opacity_cb,
                text_size,
                text_size_cb,
                is_text,
            )} <div class="flex flex-col gap-1.5 mt-1" class:hidden=move || !is_text.get()>
                <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                    Alignment
                </div>
                {uniform_seg(ALIGN_OPTS, Signal::derive(move || text_align.get()), set_text_align)}
            </div>
            {style_section(
                local_style,
                local_line_style,
                set_stroke_style,
                set_curve_mode,
                set_edge_style,
                roundness,
                roundness_cb,
                is_text,
                is_line,
                is_rect,
                is_rounded,
            )} {color_section(local_style, set_color, active_slot, color_swatches)}
            {fill_section(local_style, set_fill, active_slot, base_kind, color_swatches)}
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
        </div>
        {float_picker(active_slot, picker_color, picker_on_change, picker_ref)}
    }
}
