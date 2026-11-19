use crate::model::{ChartKind, ChartPalette, ChartStyle};
use crate::ui::components::styling_panel::utils::uniform_icon_seg;
use crate::ui::styles;
use leptos::*;
use std::rc::Rc;

const CHART_KIND_OPTS: &[(ChartKind, &str)] = &[
    (ChartKind::Bar, "Bar Chart"),
    (ChartKind::Line, "Line Chart"),
    (ChartKind::Area, "Area Chart"),
    (ChartKind::Donut, "Donut Chart"),
    (ChartKind::Sparkline, "Sparkline"),
];

fn chart_kind_icon(kind: ChartKind) -> View {
    let content = match kind {
        ChartKind::Bar => view! {
            <rect x="4" y="12" width="3.5" height="8" rx="0.8" />
            <rect x="10.25" y="5" width="3.5" height="15" rx="0.8" />
            <rect x="16.5" y="9" width="3.5" height="11" rx="0.8" />
        }
        .into_view(),
        ChartKind::Line => view! {
            <polyline points="3,17 9,10 14,14 21,6" />
            <circle cx="9" cy="10" r="1.5" fill="currentColor" />
            <circle cx="14" cy="14" r="1.5" fill="currentColor" />
        }
        .into_view(),
        ChartKind::Area => view! {
            <path d="M3,19 L3,14 L9,8 L15,12 L21,5 L21,19 Z" fill="currentColor" fill-opacity="0.25" />
            <polyline points="3,14 9,8 15,12 21,5" />
        }
        .into_view(),
        ChartKind::Donut => view! {
            <circle cx="12" cy="12" r="8" />
            <circle cx="12" cy="12" r="3.5" />
            <line x1="12" y1="4" x2="12" y2="8.5" />
            <line x1="19" y1="16" x2="15" y2="13.8" />
        }
        .into_view(),
        ChartKind::Sparkline => view! {
            <path d="M2,15 Q6,6 11,13 T22,7" />
            <circle cx="22" cy="7" r="1.8" fill="currentColor" />
        }
        .into_view(),
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
            class="w-4 h-4"
            aria-hidden="true"
        >
            {content}
        </svg>
    }
    .into_view()
}

#[allow(clippy::too_many_arguments)]
pub fn chart_section(
    is_chart: Signal<bool>,
    has_single_selected_chart: Signal<bool>,
    local_chart_style: RwSignal<ChartStyle>,
    set_chart_kind: Rc<dyn Fn(ChartKind)>,
    set_chart_palette: Rc<dyn Fn(ChartPalette)>,
    toggle_grid: Rc<dyn Fn()>,
    toggle_labels: Rc<dyn Fn()>,
    toggle_values: Rc<dyn Fn()>,
    toggle_legend: Rc<dyn Fn()>,
    on_edit_data: Rc<dyn Fn()>,
    is_horizontal: Signal<bool>,
) -> impl IntoView {
    view! {
        <div
            class=move || if is_horizontal.get() { "flex flex-row items-center gap-3 shrink-0" } else { "flex flex-col gap-3" }
            class:hidden=move || !is_chart.get()
        >
            <div class="shrink-0">
                <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                    "Chart type"
                </div>
                {uniform_icon_seg(
                    CHART_KIND_OPTS,
                    Signal::derive(move || local_chart_style.get().kind),
                    set_chart_kind,
                    chart_kind_icon,
                )}
            </div>

            <div class="shrink-0">
                <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                    "Palette"
                </div>
                <div class="grid grid-cols-3 gap-1">
                    {ChartPalette::ALL
                        .iter()
                        .map(|&pal| {
                            let label = pal.label();
                            let set_pal = set_chart_palette.clone();
                            let sample_col = crate::model::Color::new("#38bdf8");
                            let c0 = pal.color_at(0, &sample_col);
                            let c1 = pal.color_at(1, &sample_col);
                            let c2 = pal.color_at(2, &sample_col);
                            view! {
                                <button
                                    class=move || {
                                        let active = local_chart_style.get().palette == pal;
                                        if active {
                                            "flex items-center gap-1 px-1.5 py-1 rounded border border-accent bg-accent/10 text-[11px] text-accent transition-colors"
                                        } else {
                                            "flex items-center gap-1 px-1.5 py-1 rounded border border-border text-[11px] text-subtle hover:text-fg hover:bg-surface/50 transition-colors"
                                        }
                                    }
                                    title=format!("{label} palette")
                                    on:click=move |_| set_pal(pal)
                                >
                                    <span class="flex items-center -space-x-0.5 shrink-0">
                                        <span class="w-2 h-2 rounded-full" style:background-color=c0></span>
                                        <span class="w-2 h-2 rounded-full" style:background-color=c1></span>
                                        <span class="w-2 h-2 rounded-full" style:background-color=c2></span>
                                    </span>
                                    <span class="truncate">{label}</span>
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </div>

            <div class="shrink-0">
                <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                    "Display"
                </div>
                <div class="grid grid-cols-2 gap-1">
                    <button
                        class=move || {
                            if local_chart_style.get().show_grid {
                                styles::BTN_TEXT_ACTIVE
                            } else {
                                styles::BTN_TEXT
                            }
                        }
                        on:click={
                            let f = toggle_grid.clone();
                            move |_| f()
                        }
                    >
                        "Grid"
                    </button>
                    <button
                        class=move || {
                            if local_chart_style.get().show_labels {
                                styles::BTN_TEXT_ACTIVE
                            } else {
                                styles::BTN_TEXT
                            }
                        }
                        on:click={
                            let f = toggle_labels.clone();
                            move |_| f()
                        }
                    >
                        "Labels"
                    </button>
                    <button
                        class=move || {
                            if local_chart_style.get().show_values {
                                styles::BTN_TEXT_ACTIVE
                            } else {
                                styles::BTN_TEXT
                            }
                        }
                        on:click={
                            let f = toggle_values.clone();
                            move |_| f()
                        }
                    >
                        "Values"
                    </button>
                    <button
                        class=move || {
                            if local_chart_style.get().show_legend {
                                styles::BTN_TEXT_ACTIVE
                            } else {
                                styles::BTN_TEXT
                            }
                        }
                        on:click={
                            let f = toggle_legend.clone();
                            move |_| f()
                        }
                    >
                        "Legend"
                    </button>
                </div>
            </div>

            <Show when=move || has_single_selected_chart.get()>
                <button
                    class=move || {
                        if is_horizontal.get() {
                            format!("{} shrink-0 whitespace-nowrap px-2 font-medium", styles::BTN_TEXT_ACTIVE)
                        } else {
                            format!("{} w-full font-medium", styles::BTN_TEXT_ACTIVE)
                        }
                    }
                    on:click={
                        let f = on_edit_data.clone();
                        move |_| f()
                    }
                    title="Open the floating mini-sheet editor (or double-click the chart)"
                >
                    "Edit Chart Data…"
                </button>
            </Show>
        </div>
    }
}
