use crate::model::{CurveMode, EdgeStyle, ElementStyle, LineStyle, StrokeStyle};
use crate::ui::components::styling_panel::utils::uniform_icon_seg;
use crate::ui::components::NumberSlider;
use crate::ui::{icon, styles};
use leptos::*;
use std::rc::Rc;

fn preview_svg(content: View) -> View {
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="w-5 h-5"
            aria-hidden="true"
            focusable="false"
        >
            {content}
        </svg>
    }
    .into_view()
}

fn stroke_style_icon(style: StrokeStyle) -> View {
    let content = match style {
        StrokeStyle::Solid => view! { <line x1="3" y1="12" x2="21" y2="12" /> }.into_view(),
        StrokeStyle::Dashed => view! {
            <line x1="3" y1="12" x2="8" y2="12" />
            <line x1="10" y1="12" x2="15" y2="12" />
            <line x1="17" y1="12" x2="21" y2="12" />
        }
        .into_view(),
        StrokeStyle::Dotted => view! {
            <circle cx="4" cy="12" r="1.2" fill="currentColor" stroke="none" />
            <circle cx="12" cy="12" r="1.2" fill="currentColor" stroke="none" />
            <circle cx="20" cy="12" r="1.2" fill="currentColor" stroke="none" />
        }
        .into_view(),
    };
    preview_svg(content)
}

fn curve_mode_icon(mode: CurveMode) -> View {
    match mode {
        CurveMode::Straight => icon::line_straight().into_view(),
        CurveMode::Curved => icon::line_curved().into_view(),
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
                    {uniform_icon_seg(
                        curve_opts,
                        Signal::derive(move || local_line_style.get().curve_mode),
                        set_curve_mode,
                        curve_mode_icon,
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
