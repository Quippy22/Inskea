use crate::model::{CurveMode, EdgeStyle, ElementStyle, LineStyle, StrokeStyle};
use crate::ui::components::styling_panel::utils::{uniform_icon_seg, uniform_seg};
use crate::ui::components::NumberSlider;
use crate::ui::{icon, styles};
use leptos::*;
use std::rc::Rc;

fn stroke_style_icon(style: StrokeStyle) -> View {
    match style {
        StrokeStyle::Solid => icon::stroke_solid().into_view(),
        StrokeStyle::Dashed => icon::stroke_dashed().into_view(),
        StrokeStyle::Dotted => icon::stroke_dotted().into_view(),
    }
}

fn edge_style_icon(style: EdgeStyle) -> View {
    match style {
        EdgeStyle::Sharp => icon::edge_sharp().into_view(),
        EdgeStyle::Rounded => icon::edge_rounded().into_view(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn style_section(
    local_style: RwSignal<ElementStyle>,
    local_line_style: RwSignal<LineStyle>,
    set_stroke_style: Rc<dyn Fn(StrokeStyle)>,
    set_curve_mode: Rc<dyn Fn(CurveMode)>,
    set_edge_style: Rc<dyn Fn(EdgeStyle)>,
    roundness: RwSignal<f64>,
    roundness_cb: Rc<dyn Fn(f64)>,
    is_text: Signal<bool>,
    is_line: Signal<bool>,
    is_rect: Signal<bool>,
    is_rounded: Signal<bool>,
) -> impl IntoView {
    let stroke_opts: &[(StrokeStyle, &str)] = &[
        (StrokeStyle::Solid, "Solid"),
        (StrokeStyle::Dashed, "Dashed"),
        (StrokeStyle::Dotted, "Dotted"),
    ];

    let curve_opts: &[(CurveMode, &str)] = &[
        (CurveMode::Straight, "Straight"),
        (CurveMode::Curved, "Curved"),
    ];

    let edge_opts: &[(EdgeStyle, &str)] =
        &[(EdgeStyle::Sharp, "Sharp"), (EdgeStyle::Rounded, "Rounded")];

    view! {
        <div class:hidden=move || is_text.get()>
            <div>
                <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                    "Stroke style"
                </div>
                {uniform_icon_seg(
                    stroke_opts,
                    Signal::derive(move || local_style.get().stroke_style),
                    set_stroke_style,
                    stroke_style_icon,
                )}
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
                    {uniform_icon_seg(
                        edge_opts,
                        Signal::derive(move || local_style.get().edge_style),
                        set_edge_style,
                        edge_style_icon,
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
    }
}
