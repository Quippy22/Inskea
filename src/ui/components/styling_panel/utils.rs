use crate::model::{Color, Element, ElementId, ElementStyle, LineStyle, Scene};
use crate::ui::dock::Tool;
use crate::ui::styles;
use leptos::*;
use std::rc::Rc;

pub fn read_panel_state(
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

pub fn single_selected_element(scene: Scene, ids: Vec<ElementId>) -> Option<Element> {
    if ids.len() != 1 {
        return None;
    }
    let id = ids[0];
    scene.elements().iter().find(|e| e.id() == id).cloned()
}

pub fn arrowhead_icon(on: bool, is_end: bool) -> impl IntoView {
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

pub fn uniform_seg<T: PartialEq + Copy + 'static>(
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

pub fn swatch_row(
    color_swatches: &'static [&'static str],
    is_active: Rc<dyn Fn(&Color) -> bool>,
    on_click: Rc<dyn Fn(Color)>,
) -> Vec<impl IntoView> {
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
                        if is_active(&cc) { styles::BTN_SWATCH_SEL } else { styles::BTN_SWATCH_OFF }
                    }
                    style:background-color=hex
                    on:click=move |_| on_click(c.clone())
                />
            }
        })
        .collect()
}
