use crate::model::elements::text::MIN_FONT_SIZE;
use crate::model::StylingKind;
use crate::ui::components::NumberSlider;
use leptos::*;
use std::rc::Rc;

pub fn sliders_section(
    stroke_size: RwSignal<f64>,
    stroke_size_cb: Rc<dyn Fn(f64)>,
    opacity: RwSignal<f64>,
    opacity_cb: Rc<dyn Fn(f64)>,
    text_size: RwSignal<f64>,
    text_size_cb: Rc<dyn Fn(f64)>,
    kind: Signal<StylingKind>,
) -> impl IntoView {
    view! {
        <div class:hidden=move || matches!(kind.get(), StylingKind::Text | StylingKind::Image)>
            <NumberSlider
                value=stroke_size
                min=1.0
                max=5.0
                increment=1.0
                label="Stroke size"
                on_change=stroke_size_cb
            />
            <NumberSlider
                value=opacity
                min=5.0
                max=100.0
                increment=5.0
                label="Opacity"
                on_change=opacity_cb
            />
        </div>
        <div class:hidden=move || kind.get() != StylingKind::Text>
            <NumberSlider
                value=text_size
                min=MIN_FONT_SIZE
                max=200.0
                increment=1.0
                label="Font size"
                on_change=text_size_cb
            />
        </div>
    }
}
