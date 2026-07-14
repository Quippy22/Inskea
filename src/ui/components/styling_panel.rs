use std::rc::Rc;
use crate::model::{CurveMode, EdgeStyle, Element, ElementId, ElementStyle, LineStyle, Scene, ShapeColor, StrokeStyle, StylingKind};
use crate::ui::dock::Tool;
use crate::ui::classes;
use crate::ui::components::NumberSlider;
use leptos::*;

/// Build initial style/line_style from the current scene selection, or defaults.
fn read_panel_state(
    scene: &Scene,
    ids: &[ElementId],
    default_style: &ElementStyle,
    tool: Tool,
) -> (ElementStyle, LineStyle) {
    if ids.len() == 1 {
        if let Some(el) = scene.elements().iter().find(|e| e.id() == ids[0]) {
            let d = el.data();
            let ls = if let Element::Line(l) = el {
                l.line_style.clone()
            } else {
                LineStyle::default()
            };
            return (d.style.clone(), ls);
        }
    }
    (
        default_style.clone(),
        LineStyle {
            has_arrowhead: matches!(tool, Tool::Arrow),
            ..Default::default()
        },
    )
}

#[component]
pub fn StylingPanel(
    kind: StylingKind,
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    selected_tool: RwSignal<Tool>,
    default_style: RwSignal<ElementStyle>,
) -> impl IntoView {
    let is_text = kind == StylingKind::Text;
    let is_line = kind == StylingKind::Line || kind == StylingKind::Arrow;
    let is_arrow = kind == StylingKind::Arrow;
    let base_kind = !is_text && !is_line;

    let (init_style, init_line_style) = read_panel_state(
        &scene.get_untracked(),
        &selected_ids.get_untracked(),
        &default_style.get_untracked(),
        selected_tool.get_untracked(),
    );

    let local_style = create_rw_signal(init_style);
    let local_line_style = create_rw_signal(init_line_style);

    let size_value = create_rw_signal(
        if is_text { local_style.get().font_size } else { local_style.get().stroke_width },
    );

    let (size_label, size_min, size_max) = if is_text {
        ("Font size", 8.0, 200.0)
    } else {
        ("Stroke size", 1.0, 5.0)
    };

    // ── Mutator: write to scene element when selected, else default_style ─
    let update_style = {
        let scene = scene;
        let selected_ids = selected_ids;
        let default_style = default_style;
        move |f: &dyn Fn(&mut ElementStyle)| {
            let ids = selected_ids.get_untracked();
            if ids.len() == 1 {
                scene.update(|s| {
                    if let Some(el) = s.element_by_id_mut(ids[0]) {
                        f(&mut el.data_mut().style);
                    }
                });
            } else {
                default_style.update(|s| f(s));
            }
        }
    };

    let update_line_style = {
        let scene = scene;
        let selected_ids = selected_ids;
        move |f: &dyn Fn(&mut LineStyle)| {
            let ids = selected_ids.get_untracked();
            if ids.len() == 1 {
                scene.update(|s| {
                    if let Some(el) = s.element_by_id_mut(ids[0]) {
                        if let Element::Line(l) = el {
                            f(&mut l.line_style);
                        }
                    }
                });
            }
            // line-style makes no sense in draw-mode; only persist when selected.
        }
    };

    // ── Callbacks ────────────────────────────────────────────────────────
    let set_color: Rc<dyn Fn(ShapeColor)> = Rc::new({
        let local_style = local_style;
        let update_style = update_style;
        move |c| {
            local_style.update(|s| s.stroke_color = c);
            update_style(&|s| s.stroke_color = c);
        }
    });

    let set_fill: Rc<dyn Fn(Option<ShapeColor>)> = Rc::new({
        let local_style = local_style;
        let update_style = update_style;
        move |c| {
            local_style.update(|s| s.fill_color = c);
            update_style(&|s| s.fill_color = c);
        }
    });

    let set_stroke_style: Rc<dyn Fn(StrokeStyle)> = Rc::new({
        let local_style = local_style;
        let update_style = update_style;
        move |v| {
            local_style.update(|s| s.stroke_style = v);
            update_style(&|s| s.stroke_style = v);
        }
    });

    let set_edge_style: Rc<dyn Fn(EdgeStyle)> = Rc::new({
        let local_style = local_style;
        let update_style = update_style;
        move |v| {
            local_style.update(|s| s.edge_style = v);
            update_style(&|s| s.edge_style = v);
        }
    });

    let set_curve_mode: Rc<dyn Fn(CurveMode)> = Rc::new({
        let local_line_style = local_line_style;
        let update_line_style = update_line_style;
        move |v| {
            local_line_style.update(|ls| ls.curve_mode = v);
            update_line_style(&|ls| ls.curve_mode = v);
        }
    });

    let set_has_arrowhead: Rc<dyn Fn(bool)> = Rc::new({
        let local_line_style = local_line_style;
        let update_line_style = update_line_style;
        move |v| {
            local_line_style.update(|ls| ls.has_arrowhead = v);
            update_line_style(&|ls| ls.has_arrowhead = v);
        }
    });

    let size_cb: Rc<dyn Fn(f64)> = Rc::new({
        let scene = scene;
        let selected_ids = selected_ids;
        let default_style = default_style;
        move |val| {
            let ids = selected_ids.get_untracked();
            if ids.len() == 1 {
                scene.update(|s| {
                    if let Some(el) = s.element_by_id_mut(ids[0]) {
                        let d = el.data_mut();
                        d.style.stroke_width = val;
                        d.style.font_size = val;
                    }
                });
            } else {
                default_style.update(|s| {
                    s.stroke_width = val;
                    s.font_size = val;
                });
            }
        }
    });

    // ── View ─────────────────────────────────────────────────────────────
    let stroke_opts: &[(StrokeStyle, &str)] = &[
        (StrokeStyle::Solid, "Solid"),
        (StrokeStyle::Dashed, "Dashed"),
        (StrokeStyle::Dotted, "Dotted"),
    ];

    let edge_opts: &[(EdgeStyle, &str)] = &[
        (EdgeStyle::Sharp, "Sharp"),
        (EdgeStyle::Rounded, "Rounded"),
    ];

    let curve_opts: &[(CurveMode, &str)] = &[
        (CurveMode::Straight, "Straight"),
        (CurveMode::Curved, "Curved"),
    ];

    let arrowhead_opts: &[(bool, &str)] = &[(false, "None"), (true, "Triangle")];

    let color_swatches: &[ShapeColor] = &[
        ShapeColor::White,
        ShapeColor::Yellow,
        ShapeColor::Green,
        ShapeColor::Blue,
        ShapeColor::Red,
    ];

    view! {
        <div class="flex flex-col gap-4">
            <NumberSlider
                value=size_value
                min=size_min
                max=size_max
                increment=1.0
                label=size_label
                on_change=size_cb
            />

            {if !is_text {
                view! {
                    <div>
                        <div class=classes::SETTINGS_LABEL class:mb-1=move || true>"Stroke style"</div>
                        {uniform_seg(stroke_opts, Signal::derive(move || local_style.get().stroke_style), set_stroke_style)}
                    </div>
                }.into_view()
            } else {
                view! {}.into_view()
            }}

            <div>
                <div class=classes::SETTINGS_LABEL class:mb-1=move || true>"Color"</div>
                <div class="flex gap-1">
                    {color_swatches
                        .iter()
                        .map(|c| {
                            let c = *c;
                            let set_color = set_color.clone();
                            view! {
                                <button
                                    class=move || {
                                        if local_style.get().stroke_color == c {
                                            classes::BTN_SWATCH_SEL
                                        } else {
                                            classes::BTN_SWATCH_OFF
                                        }
                                    }
                                    style:background-color=c.to_hex()
                                    on:click=move |_| set_color(c)
                                />
                            }
                        })
                        .collect::<Vec<_>>()}
                </div>
            </div>

            {if base_kind {
                view! {
                    <div>
                        <div class=classes::SETTINGS_LABEL class:mb-1=move || true>"Background"</div>
                        <div class="flex gap-1 items-center">
                            <button
                                class=move || {
                                    if local_style.get().fill_color.is_none() {
                                        classes::BTN_SWATCH_SEL
                                    } else {
                                        classes::BTN_SWATCH_OFF
                                    }
                                }
                                on:click={
                                    let set_fill = set_fill.clone();
                                    move |_| set_fill(None)
                                }
                                title="No fill"
                            />
                            <div class="w-px h-5 bg-border mx-1"></div>
                            {color_swatches
                                .iter()
                                .map(|c| {
                                    let c = *c;
                                    let set_fill = set_fill.clone();
                                    view! {
                                        <button
                                            class=move || {
                                                match local_style.get().fill_color {
                                                    Some(fc) if fc == c => classes::BTN_SWATCH_SEL,
                                                    _ => classes::BTN_SWATCH_OFF,
                                                }
                                            }
                                            style:background-color=c.to_hex()
                                            on:click=move |_| set_fill(Some(c))
                                        />
                                    }
                                })
                                .collect::<Vec<_>>()}
                        </div>
                    </div>
                }.into_view()
            } else {
                view! {}.into_view()
            }}

            {if is_text {
                view! {}.into_view()
            } else if is_line {
                view! {
                    <div>
                        <div class=classes::SETTINGS_LABEL class:mb-1=move || true>"Line style"</div>
                        {uniform_seg(curve_opts, Signal::derive(move || local_line_style.get().curve_mode), set_curve_mode)}
                    </div>
                }.into_view()
            } else {
                view! {
                    <div>
                        <div class=classes::SETTINGS_LABEL class:mb-1=move || true>"Edge style"</div>
                        {uniform_seg(edge_opts, Signal::derive(move || local_style.get().edge_style), set_edge_style)}
                    </div>
                }.into_view()
            }}

            {if is_arrow {
                view! {
                    <div>
                        <div class=classes::SETTINGS_LABEL class:mb-1=move || true>"Arrowhead style"</div>
                        {uniform_seg(arrowhead_opts, Signal::derive(move || local_line_style.get().has_arrowhead), set_has_arrowhead)}
                    </div>
                }.into_view()
            } else {
                view! {}.into_view()
            }}
        </div>
    }
}

/// Segmented control with equal-width buttons.
fn uniform_seg<T: PartialEq + Copy + 'static>(
    options: &'static [(T, &'static str)],
    active: Signal<T>,
    on_change: Rc<dyn Fn(T)>,
) -> impl IntoView {
    let last = options.len() - 1;
    view! {
        <div class="flex rounded-md border border-border overflow-hidden">
            {options
                .iter()
                .enumerate()
                .map(|(i, (val, label))| {
                    let is_last = i == last;
                    let val = *val;
                    let on_change = on_change.clone();
                    view! {
                        <button
                            class=move || {
                                let base = if active.get() == val {
                                    classes::SEG_BTN_ACTIVE
                                } else {
                                    classes::SEG_BTN_INACTIVE
                                };
                                format!("{} flex-1 justify-center", base)
                            }
                            class:border-r=move || !is_last
                            class:border-border=move || !is_last
                            on:click=move |_| on_change(val)
                        >
                            {*label}
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}
