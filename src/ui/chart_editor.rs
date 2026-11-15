use crate::canvas::Viewport;
use crate::edit_ops;
use crate::model::{Bounds, Chart, ChartItem, Element, ElementId, Point, Scene};
use crate::ui::chart_import::ChartImport;
use crate::ui::export;
use crate::ui::{icon, styles};
use crate::util::window_size;
use leptos::*;
use std::rc::Rc;

/// Floating mini-sheet data editor that opens when the user double-clicks a chart
/// on the canvas or clicks `"Edit Chart Data…"` in the styling panel.
#[component]
pub fn ChartDataEditor(
    scene: RwSignal<Scene>,
    chart_editing_id: RwSignal<Option<ElementId>>,
    viewport: RwSignal<Viewport>,
    chart_import: ChartImport,
    push_snapshot: Rc<dyn Fn()>,
) -> impl IntoView {
    let paste_open = create_rw_signal(false);
    let paste_text = create_rw_signal(String::new());
    let push_stored = store_value(push_snapshot);
    let import_stored = store_value(chart_import);

    create_effect(move |_| {
        if chart_editing_id.get().is_none() {
            paste_open.set(false);
            paste_text.set(String::new());
        }
    });

    view! {
        <Show when=move || chart_editing_id.get().is_some()>
            {move || {
                let Some(chart_id) = chart_editing_id.get() else {
                    return view! {}.into_view();
                };
                let chart_opt = scene.with(|s| match s.element_by_id(chart_id) {
                    Some(Element::Chart(c)) => Some(c.clone()),
                    _ => None,
                });
                let Some(chart) = chart_opt else {
                    return view! {}.into_view();
                };

                // Compute screen position anchored to the right of the chart's bounding box,
                // clamped inside the viewport so it never goes off-screen.
                let vp = viewport.get();
                let win_size = window_size();
                let win_w = win_size.x;
                let win_h = win_size.y;
                let bounds = chart.bounds();
                let right_world = Point::new(bounds.x + bounds.w, bounds.y);
                let left_world = Point::new(bounds.x, bounds.y);
                let right_screen = vp.world_to_screen(right_world, win_size);
                let left_screen = vp.world_to_screen(left_world, win_size);

                let card_w = 304.0_f64;
                let card_h = 380.0_f64;
                let margin = 16.0_f64;

                let x = if right_screen.x + margin + card_w <= win_w - margin {
                    right_screen.x + margin
                } else if left_screen.x - margin - card_w >= 88.0 {
                    left_screen.x - margin - card_w
                } else {
                    (win_w - card_w - margin).max(88.0)
                };
                let y = right_screen.y.clamp(60.0, (win_h - card_h - margin).max(60.0));

                let title_val = chart.title.clone();
                let items = chart.items.clone();
                let palette = chart.palette;
                let stroke_col = chart.data.style.stroke_color;
                let push_snapshot = push_stored.get_value();
                let push_for_title = push_snapshot.clone();
                let push_for_add = push_snapshot.clone();
                let push_for_paste = push_snapshot.clone();
                let chart_import_csv = import_stored.get_value();

                view! {
                    <div
                        class=format!(
                            "{} fixed z-50 w-[19rem] p-3 flex flex-col gap-2.5 select-none",
                            styles::PANEL
                        )
                        style:left=format!("{x:.0}px")
                        style:top=format!("{y:.0}px")
                        on:pointerdown=|ev| ev.stop_propagation()
                        on:wheel=|ev| ev.stop_propagation()
                    >
                        // Header row
                        <div class="flex items-center justify-between border-b border-border pb-2">
                            <div class="flex items-center gap-1.5 text-xs font-medium text-fg">
                                <span class="text-accent">{icon::chart()}</span>
                                <span>"Chart Data"</span>
                            </div>
                            <button
                                class=styles::BTN_GHOST
                                title="Close (Esc)"
                                on:click=move |_| chart_editing_id.set(None)
                            >
                                "✕"
                            </button>
                        </div>

                        // Chart title input
                        <div class="flex flex-col gap-1">
                            <label class=styles::SETTINGS_LABEL>"Chart Title"</label>
                            <input
                                type="text"
                                placeholder="Optional chart title…"
                                prop:value=title_val
                                class="w-full h-7 px-2 text-xs rounded border border-border bg-surface/40 text-fg placeholder:text-subtle/60 focus:outline-none focus:border-accent"
                                on:change=move |ev| {
                                    let new_title = event_target_value(&ev);
                                    edit_ops::mutate_scene(scene, &push_for_title, |s| {
                                        if let Some(mut el) = s.element_by_id_mut(chart_id) {
                                            if let Element::Chart(c) = &mut *el {
                                                c.title = new_title.clone();
                                            }
                                        }
                                        s.refresh_connectors_for(chart_id);
                                    });
                                }
                            />
                        </div>

                        // Mini-sheet rows header
                        <div class="flex items-center justify-between text-[11px] text-subtle px-1">
                            <span>"Label"</span>
                            <span class="mr-7">"Value"</span>
                        </div>

                        // Rows list
                        <div class="flex flex-col gap-1.5 max-h-52 overflow-y-auto pr-0.5">
                            {items
                                .iter()
                                .enumerate()
                                .map(|(idx, item)| {
                                    let row_label = item.label.clone();
                                    let row_value = format_editable_number(item.value);
                                    let swatch_hex = palette.color_at(idx, &stroke_col);
                                    let can_delete = items.len() > 1;
                                    let push_lbl = push_snapshot.clone();
                                    let push_val = push_snapshot.clone();
                                    let push_del = push_snapshot.clone();

                                    view! {
                                        <div class="flex items-center gap-1.5">
                                            <span
                                                class="w-2.5 h-2.5 rounded-full shrink-0"
                                                style:background-color=swatch_hex
                                            ></span>
                                            <input
                                                type="text"
                                                prop:value=row_label
                                                placeholder=format!("Item {}", idx + 1)
                                                class="flex-1 min-w-0 h-7 px-2 text-xs rounded border border-border bg-surface/40 text-fg focus:outline-none focus:border-accent"
                                                on:change=move |ev| {
                                                    let val = event_target_value(&ev);
                                                    edit_ops::mutate_scene(scene, &push_lbl, |s| {
                                                        if let Some(mut el) = s.element_by_id_mut(chart_id) {
                                                            if let Element::Chart(c) = &mut *el {
                                                                if let Some(it) = c.items.get_mut(idx) {
                                                                    it.label = val.clone();
                                                                }
                                                            }
                                                        }
                                                    });
                                                }
                                            />
                                            <input
                                                type="number"
                                                step="any"
                                                prop:value=row_value
                                                class="w-20 h-7 px-2 text-xs text-right tabular-nums rounded border border-border bg-surface/40 text-fg focus:outline-none focus:border-accent"
                                                on:change=move |ev| {
                                                    let raw = event_target_value(&ev);
                                                    if let Ok(parsed) = raw.trim().parse::<f64>() {
                                                        if parsed.is_finite() {
                                                            let clamped = parsed.max(0.0);
                                                            edit_ops::mutate_scene(scene, &push_val, |s| {
                                                                if let Some(mut el) = s.element_by_id_mut(chart_id) {
                                                                    if let Element::Chart(c) = &mut *el {
                                                                        if let Some(it) = c.items.get_mut(idx) {
                                                                            it.value = clamped;
                                                                        }
                                                                    }
                                                                }
                                                                s.refresh_connectors_for(chart_id);
                                                            });
                                                        }
                                                    }
                                                }
                                            />
                                            <button
                                                class="flex items-center justify-center w-6 h-6 rounded text-subtle hover:text-fg hover:bg-surface/60 disabled:opacity-30 disabled:pointer-events-none transition-colors"
                                                disabled=!can_delete
                                                title="Remove row"
                                                on:click=move |_| {
                                                    edit_ops::mutate_scene(scene, &push_del, |s| {
                                                        if let Some(mut el) = s.element_by_id_mut(chart_id) {
                                                            if let Element::Chart(c) = &mut *el {
                                                                if c.items.len() > 1 && idx < c.items.len() {
                                                                    c.items.remove(idx);
                                                                }
                                                            }
                                                        }
                                                        s.refresh_connectors_for(chart_id);
                                                    });
                                                }
                                            >
                                                "−"
                                            </button>
                                        </div>
                                    }
                                })
                                .collect_view()}
                        </div>

                        // Action buttons: + Add Row, Paste CSV/Table, Import File, Export CSV
                        <div class="flex items-center gap-1.5 pt-1 border-t border-border">
                            <button
                                class=format!("{} flex-1", styles::BTN_TEXT)
                                on:click=move |_| {
                                    edit_ops::mutate_scene(scene, &push_for_add, |s| {
                                        if let Some(mut el) = s.element_by_id_mut(chart_id) {
                                            if let Element::Chart(c) = &mut *el {
                                                let next_n = c.items.len() + 1;
                                                let avg = if c.items.is_empty() {
                                                    25.0
                                                } else {
                                                    (c.items.iter().map(|i| i.value).sum::<f64>()
                                                        / c.items.len() as f64)
                                                        .round()
                                                        .max(10.0)
                                                };
                                                c.items.push(ChartItem::new(format!("Item {next_n}"), avg));
                                            }
                                        }
                                        s.refresh_connectors_for(chart_id);
                                    });
                                }
                            >
                                "+ Add Row"
                            </button>
                            <button
                                class=move || {
                                    if paste_open.get() {
                                        styles::BTN_TEXT_ACTIVE
                                    } else {
                                        styles::BTN_TEXT
                                    }
                                }
                                title="Paste CSV, TSV, or Markdown table"
                                on:click=move |_| paste_open.update(|v| *v = !*v)
                            >
                                "Paste Table"
                            </button>
                            <button
                                class=styles::BTN_TEXT
                                title="Import CSV/TSV file into this chart"
                                on:click=move |_| {
                                    chart_import_csv.pick_csv();
                                }
                            >
                                "CSV…"
                            </button>
                            <button
                                class=styles::BTN_TEXT
                                title="Download chart data as .csv"
                                on:click=move |_| {
                                    let csv_opt = scene.with_untracked(|s| match s.element_by_id(chart_id) {
                                        Some(Element::Chart(c)) => Some(c.to_csv_string()),
                                        _ => None,
                                    });
                                    if let Some(csv) = csv_opt {
                                        export::download_string(&csv, "chart-data.csv");
                                    }
                                }
                            >
                                "↓"
                            </button>
                        </div>

                        // Expandable Paste CSV / Markdown box
                        <Show when=move || paste_open.get()>
                            <div class="flex flex-col gap-1.5 pt-1 border-t border-border">
                                <textarea
                                    rows="4"
                                    placeholder="Paste CSV, TSV, or Markdown table:&#10;Q1, 24&#10;Q2, 38&#10;Q3, 31"
                                    prop:value=move || paste_text.get()
                                    class="w-full p-2 text-xs font-mono rounded border border-border bg-surface/40 text-fg placeholder:text-subtle/60 focus:outline-none focus:border-accent"
                                    on:input=move |ev| paste_text.set(event_target_value(&ev))
                                ></textarea>
                                <div class="flex justify-end gap-1.5">
                                    <button
                                        class=styles::BTN_TEXT
                                        on:click=move |_| paste_open.set(false)
                                    >
                                        "Cancel"
                                    </button>
                                    <button
                                        class=styles::BTN_TEXT_ACTIVE
                                        on:click={
                                            let push_for_paste = push_for_paste.clone();
                                            move |_| {
                                                let raw = paste_text.get_untracked();
                                                if let Some((parsed_title, new_items)) =
                                                    Chart::parse_tabular_data(&raw)
                                                {
                                                    edit_ops::mutate_scene(
                                                        scene,
                                                        &push_for_paste,
                                                        |s| {
                                                            if let Some(mut el) =
                                                                s.element_by_id_mut(chart_id)
                                                            {
                                                                if let Element::Chart(c) = &mut *el {
                                                                    if let Some(t) = parsed_title {
                                                                        c.title = t;
                                                                    }
                                                                    c.items = new_items;
                                                                }
                                                            }
                                                            s.refresh_connectors_for(chart_id);
                                                        },
                                                    );
                                                    paste_text.set(String::new());
                                                    paste_open.set(false);
                                                }
                                            }
                                        }
                                    >
                                        "Apply Data"
                                    </button>
                                </div>
                            </div>
                        </Show>
                    </div>
                }
                    .into_view()
            }}
        </Show>
    }
}

fn format_editable_number(val: f64) -> String {
    if (val - val.round()).abs() < 1e-4 {
        format!("{:.0}", val.round())
    } else {
        format!("{:.2}", val)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::format_editable_number;

    #[test]
    fn editable_number_formatting() {
        assert_eq!(format_editable_number(42.0), "42");
        assert_eq!(format_editable_number(12.5), "12.5");
        assert_eq!(format_editable_number(std::f64::consts::PI), "3.14");
    }
}
