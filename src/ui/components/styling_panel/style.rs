use crate::model::{CurveMode, EdgeStyle, ElementStyle, LineStyle, StrokeStyle};
use crate::ui::components::styling_panel::utils::uniform_seg;
use crate::ui::components::NumberSlider;
use crate::ui::styles;
use leptos::*;
use std::rc::Rc;

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
                {uniform_seg(
                    stroke_opts,
                    Signal::derive(move || local_style.get().stroke_style),
                    set_stroke_style,
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
    }
}
