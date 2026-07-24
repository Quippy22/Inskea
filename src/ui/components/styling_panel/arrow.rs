use crate::model::LineStyle;
use crate::ui::styles;
use crate::ui::components::styling_panel::utils::arrowhead_icon;
use leptos::*;
use std::rc::Rc;

pub fn arrow_section(
    is_arrow: Signal<bool>,
    local_line_style: RwSignal<LineStyle>,
    set_start_arrowhead: Rc<dyn Fn()>,
    set_end_arrowhead: Rc<dyn Fn()>,
) -> impl IntoView {
    view! {
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
    }
}