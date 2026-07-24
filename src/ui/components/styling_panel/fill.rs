use crate::model::{Color, ElementStyle};
use crate::ui::components::styling_panel::color::ColorSlot;
use crate::ui::components::styling_panel::utils::swatch_row;
use crate::ui::components::ColorPickerButton;
use crate::ui::styles;
use leptos::*;
use std::rc::Rc;

pub fn fill_section(
    local_style: RwSignal<ElementStyle>,
    set_fill: Rc<dyn Fn(Option<Color>)>,
    active_slot: RwSignal<Option<ColorSlot>>,
    base_kind: Signal<bool>,
    color_swatches: &'static [&'static str],
) -> impl IntoView {
    view! {
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
                        color_swatches,
                        Rc::new(move |c| local_style.get().fill_color.as_ref() == Some(c)),
                        Rc::new(move |c| set_fill(Some(c))),
                    )}
                    <ColorPickerButton on_click=Rc::new(move || {
                        active_slot.set(Some(ColorSlot::Fill))
                    }) />
                </div>
            </div>
        </div>
    }
}