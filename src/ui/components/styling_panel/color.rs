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
}

pub fn color_section(
    local_style: RwSignal<ElementStyle>,
    set_color: Rc<dyn Fn(Color)>,
    active_slot: RwSignal<Option<ColorSlot>>,
    color_swatches: &'static [&'static str],
) -> impl IntoView {
    view! {
        <div>
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
