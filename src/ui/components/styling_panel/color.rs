use crate::model::{Color, ElementStyle};
use crate::ui::components::styling_panel::utils::swatch_row;
use crate::ui::components::ColorPickerButton;
use crate::ui::styles;
use leptos::*;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq)]
pub enum ColorSlot {
    Stroke,
    Fill,
    EmbeddedText,
}

pub fn embedded_text_color_section(
    local_color: RwSignal<Color>,
    set_color: Rc<dyn Fn(Color)>,
    active_slot: RwSignal<Option<ColorSlot>>,
    color_swatches: &'static [&'static str],
    has_text: Signal<bool>,
) -> impl IntoView {
    view! {
        <div class:hidden=move || !has_text.get()>
            <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                "Text color"
            </div>
            <div class="flex gap-1 items-center flex-wrap">
                {swatch_row(
                    color_swatches,
                    Rc::new(move |c| local_color.get() == *c),
                    Rc::new(move |c| set_color(c)),
                )}
                <ColorPickerButton on_click=Rc::new(move || {
                    active_slot.set(Some(ColorSlot::EmbeddedText))
                }) />
            </div>
        </div>
    }
}

pub fn color_section(
    local_style: RwSignal<ElementStyle>,
    set_color: Rc<dyn Fn(Color)>,
    active_slot: RwSignal<Option<ColorSlot>>,
    color_swatches: &'static [&'static str],
    hide_stroke: Signal<bool>,
) -> impl IntoView {
    view! {
        <div class:hidden=move || hide_stroke.get()>
            <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                "Color"
            </div>
            <div class="flex gap-1 items-center flex-wrap">
                {swatch_row(
                    color_swatches,
                    Rc::new(move |c| local_style.get().stroke_color == *c),
                    Rc::new(move |c| set_color(c)),
                )}
                <ColorPickerButton on_click=Rc::new(move || {
                    active_slot.set(Some(ColorSlot::Stroke))
                }) />
            </div>
        </div>
    }
}
