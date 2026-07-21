use crate::model::{
    Color, CurveMode, EdgeStyle, Element, ElementId, ElementStyle, LineStyle, Scene, StrokeStyle,
    StylingKind,
};
use crate::ui::components::ColorPickerButton;
use crate::ui::components::NumberSlider;
use crate::ui::dock::Tool;
use crate::ui::styles;
use leptos::*;
use leptos_color::components::color_picker::ColorPicker as LpcCp;
use leptos_color::theme::Theme;
use leptos_color::Color as LpcColor;
use std::rc::Rc;
use wasm_bindgen::JsCast;

fn read_panel_state(
    scene: &Scene,
    ids: &[ElementId],
    default_style: &ElementStyle,
    default_line_style: &LineStyle,
    _tool: Tool,
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
    (default_style.clone(), default_line_style.clone())
}

fn single_selected_element(scene: Scene, ids: Vec<ElementId>) -> Option<Element> {
    if ids.len() != 1 {
        return None;
    }
    let id = ids[0];
    scene.elements().iter().find(|e| e.id() == id).cloned()
}

#[component]
pub fn StylingPanel(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    selected_tool: RwSignal<Tool>,
    default_style: RwSignal<ElementStyle>,
    default_line_style: RwSignal<LineStyle>,
) -> impl IntoView {
    let styling_kind = Signal::derive(move || {
        let ids = selected_ids.get();
        if ids.len() == 1 {
            let el = leptos::untrack(|| single_selected_element(scene.get_untracked(), ids));
            if let Some(el) = el {
                return el.styling_kind();
            }
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
    let roundness = create_rw_signal(local_style.get_untracked().roundness);
    let opacity = create_rw_signal(local_style.get_untracked().opacity * 100.0);
    let is_rounded = Signal::derive(move || local_style.get().edge_style == EdgeStyle::Rounded);

    #[derive(Clone, Copy, PartialEq)]
    enum ColorSlot {
        Stroke,
        Fill,
    }
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

    // ── Sync: when selection/scene changes, pull back into local signals ─
    create_effect(move |_| {
        let _ids = selected_ids.get();
        let _s = scene.get();
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
        roundness.set(new_style.roundness);
        opacity.set(new_style.opacity * 100.0);
        local_style.set(new_style);
        local_line_style.set(new_line_style);
    });

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
        let default_line_style = default_line_style;
        move |f: &dyn Fn(&mut LineStyle)| {
            let ids = selected_ids.get_untracked();
            if ids.len() == 1 {
                scene.update(|s| {
                    if let Some(Element::Line(l)) = s.element_by_id_mut(ids[0]) {
                        f(&mut l.line_style);
                    }
                });
            } else {
                default_line_style.update(|ls| f(ls));
            }
        }
    };

    // ── Callbacks ────────────────────────────────────────────────────────
    let set_color: Rc<dyn Fn(Color)> = Rc::new({
        let local_style = local_style;
        let update_style = update_style;
        move |c| {
            local_style.update(|s| s.stroke_color = c.clone());
            update_style(&|s| s.stroke_color = c.clone());
        }
    });

    let set_fill: Rc<dyn Fn(Option<Color>)> = Rc::new({
        let local_style = local_style;
        let update_style = update_style;
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

    let set_start_arrowhead: Rc<dyn Fn()> = Rc::new({
        let local_line_style = local_line_style;
        let update_line_style = update_line_style;
        move || {
            let new = !local_line_style.get_untracked().has_start_arrowhead;
            local_line_style.update(|ls| ls.has_start_arrowhead = new);
            update_line_style(&|ls| ls.has_start_arrowhead = new);
        }
    });

    let set_end_arrowhead: Rc<dyn Fn()> = Rc::new({
        let local_line_style = local_line_style;
        let update_line_style = update_line_style;
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
        move |val| {
            let ids = selected_ids.get_untracked();
            if ids.len() == 1 {
                scene.update(|s| {
                    if let Some(el) = s.element_by_id_mut(ids[0]) {
                        el.data_mut().style.stroke_width = val;
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
        move |val| {
            let ids = selected_ids.get_untracked();
            if ids.len() == 1 {
                scene.update(|s| {
                    if let Some(el) = s.element_by_id_mut(ids[0]) {
                        el.data_mut().style.font_size = val;
                    }
                });
            } else {
                default_style.update(|s| s.font_size = val);
            }
        }
    });

    let roundness_cb: Rc<dyn Fn(f64)> = Rc::new({
        let local_style = local_style;
        let update_style = update_style;
        move |val| {
            local_style.update(|s| s.roundness = val);
            update_style(&|s| s.roundness = val);
        }
    });

    let opacity_cb: Rc<dyn Fn(f64)> = Rc::new({
        let scene = scene;
        let selected_ids = selected_ids;
        let default_style = default_style;
        move |val| {
            let v = (val / 100.0).clamp(0.0, 1.0);
            let ids = selected_ids.get_untracked();
            if ids.len() == 1 {
                scene.update(|s| {
                    if let Some(el) = s.element_by_id_mut(ids[0]) {
                        el.data_mut().style.opacity = v;
                    }
                });
            } else {
                default_style.update(|s| s.opacity = v);
            }
        }
    });

    // ── View ─────────────────────────────────────────────────────────────
    let stroke_opts: &[(StrokeStyle, &str)] = &[
        (StrokeStyle::Solid, "Solid"),
        (StrokeStyle::Dashed, "Dashed"),
        (StrokeStyle::Dotted, "Dotted"),
    ];

    let edge_opts: &[(EdgeStyle, &str)] =
        &[(EdgeStyle::Sharp, "Sharp"), (EdgeStyle::Rounded, "Rounded")];

    let curve_opts: &[(CurveMode, &str)] = &[
        (CurveMode::Straight, "Straight"),
        (CurveMode::Curved, "Curved"),
    ];

    let color_swatches: &[&str] = &[
        Color::WHITE,
        Color::YELLOW,
        Color::GREEN,
        Color::BLUE,
        Color::RED,
    ];

    let swatch_row =
        move |is_active: Rc<dyn Fn(&Color) -> bool>, on_click: Rc<dyn Fn(Color)>| -> Vec<_> {
            color_swatches
                .iter()
                .map(|&hex| {
                    let on_click = on_click.clone();
                    let is_active = is_active.clone();
                    let c = Color::new(hex);
                    let cc = c.clone();
                    view! {
                        <button
                            class=move || {
                                if is_active(&cc) {
                                    styles::BTN_SWATCH_SEL
                                } else {
                                    styles::BTN_SWATCH_OFF
                                }
                            }
                            style:background-color=hex
                            on:click=move |_| on_click(c.clone())
                        />
                    }
                })
                .collect()
        };

    view! {
        <div class="flex flex-col gap-4 select-none">
            <div class:hidden=move || is_text.get()>
                <NumberSlider
                    value=stroke_size
                    min=1.0
                    max=5.0
                    increment=1.0
                    label="Stroke size"
                    on_change=stroke_size_cb
                />
                <NumberSlider
                    value=opacity
                    min=5.0
                    max=100.0
                    increment=5.0
                    label="Opacity"
                    on_change=opacity_cb
                />
            </div>

            <div class:hidden=move || !is_text.get()>
                <NumberSlider
                    value=text_size
                    min=8.0
                    max=200.0
                    increment=1.0
                    label="Font size"
                    on_change=text_size_cb
                />
            </div>

            <div class:hidden=move || is_text.get()>
                <div>
                    <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                        "Stroke style"
                    </div>
                    {uniform_seg(
                        stroke_opts,
                        Signal::derive(move || local_style.get().stroke_style),
                        set_stroke_style,
                    )}
                </div>
            </div>

            <div>
                <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                    "Color"
                </div>
                <div class="flex gap-1 items-center flex-wrap">
                    {swatch_row(
                        Rc::new(move |c| local_style.get().stroke_color == *c),
                        Rc::new(move |c| set_color(c)),
                    )}
                    <ColorPickerButton on_click=Rc::new(move || {
                        active_slot.set(Some(ColorSlot::Stroke))
                    }) />
                </div>
            </div>

            <div class:hidden=move || !base_kind.get()>
                <div>
                    <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                        "Background"
                    </div>
                    <div class="flex gap-1 items-center flex-wrap">
                        <button
                            class=move || {
                                if local_style.get().fill_color.is_none() {
                                    styles::BTN_SWATCH_SEL
                                } else {
                                    styles::BTN_SWATCH_OFF
                                }
                            }
                            on:click={
                                let set_fill = set_fill.clone();
                                move |_| set_fill(None)
                            }
                            title="No fill"
                        ></button>
                        <div class="w-px h-5 bg-border mx-1"></div>
                        {swatch_row(
                            Rc::new(move |c| local_style.get().fill_color.as_ref() == Some(c)),
                            Rc::new(move |c| set_fill(Some(c))),
                        )}
                        <ColorPickerButton on_click=Rc::new(move || {
                            active_slot.set(Some(ColorSlot::Fill))
                        }) />
                    </div>
                </div>
            </div>

            <div class:hidden=move || is_text.get()>
                <div class:hidden=move || !is_line.get()>
                    <div>
                        <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                            "Line style"
                        </div>
                        {uniform_seg(
                            curve_opts,
                            Signal::derive(move || local_line_style.get().curve_mode),
                            set_curve_mode,
                        )}
                    </div>
                </div>
                <div class:hidden=move || !is_rect.get()>
                    <div>
                        <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                            "Edge style"
                        </div>
                        {uniform_seg(
                            edge_opts,
                            Signal::derive(move || local_style.get().edge_style),
                            set_edge_style,
                        )}
                    </div>
                    <div class:hidden=move || !(is_rounded.get() && is_rect.get())>
                        <NumberSlider
                            value=roundness
                            min=2.0
                            max=20.0
                            increment=2.0
                            label="Roundness"
                            on_change=roundness_cb
                        />
                    </div>
                </div>
            </div>

            <div class:hidden=move || !is_arrow.get()>
                <div>
                    <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                        "Arrow direction"
                    </div>
                    <div class="flex gap-1">
                        <button
                            class=move || {
                                if local_line_style.get().has_start_arrowhead {
                                    styles::BTN_SWATCH_SEL
                                } else {
                                    styles::BTN_SWATCH_OFF
                                }
                            }
                            on:click=move |_| set_start_arrowhead()
                            title="Arrowhead at start"
                        >
                            {move || arrowhead_icon(
                                local_line_style.get().has_start_arrowhead,
                                false,
                            )}
                        </button>
                        <button
                            class=move || {
                                if local_line_style.get().has_end_arrowhead {
                                    styles::BTN_SWATCH_SEL
                                } else {
                                    styles::BTN_SWATCH_OFF
                                }
                            }
                            on:click=move |_| set_end_arrowhead()
                            title="Arrowhead at end"
                        >
                            {move || arrowhead_icon(local_line_style.get().has_end_arrowhead, true)}
                        </button>
                    </div>
                </div>
            </div>
        </div>
        <Show when=move || active_slot.get().is_some()>
            <div class="absolute left-full top-1/2 -translate-y-1/2 ml-4" style="z-index: 9999;">
                <div node_ref=picker_ref>
                    {
                        let lpc_theme = Theme::custom(
                            LpcColor::from_html("#24283b").unwrap(),
                            LpcColor::from_html("#1e1f2e").unwrap(),
                            LpcColor::from_html("#a9b1d6").unwrap(),
                            LpcColor::from_html("#3b4261").unwrap(),
                            "8px".to_string(),
                            "0 4px 12px rgba(0,0,0,0.3)".to_string(),
                            "280px".to_string(),
                        );
                        view! {
                            <LpcCp
                                color=picker_color
                                on_change=picker_on_change
                                hide_alpha=true
                                theme=lpc_theme
                            />
                        }
                    }
                </div>
            </div>
        </Show>
    }
}

fn arrowhead_icon(on: bool, is_end: bool) -> impl IntoView {
    let (tx, ty, lx, ly, rx, ry) = if is_end {
        (20.0, 12.0, 13.0, 6.0, 13.0, 18.0)
    } else {
        (4.0, 12.0, 11.0, 6.0, 11.0, 18.0)
    };
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="w-6 h-6"
        >
            <line x1="4" y1="12" x2="20" y2="12" />
            {if on {
                Some(
                    leptos::view! {
                        <polyline points=format!(
                            "{:.0},{:.0} {:.0},{:.0} {:.0},{:.0}",
                            lx,
                            ly,
                            tx,
                            ty,
                            rx,
                            ry,
                        ) />
                    }
                        .into_view(),
                )
            } else {
                None
            }}
        </svg>
    }
}

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
                                    styles::SEG_BTN_ACTIVE
                                } else {
                                    styles::SEG_BTN_INACTIVE
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
