use leptos::*;
use std::rc::Rc;

#[component]
pub fn ColorPickerButton(on_click: Rc<dyn Fn()>) -> impl IntoView {
    view! {
        <button
            on:click=move |_| on_click()
            title="Custom color"
            class="w-7 h-7 rounded-md border border-border cursor-pointer flex-shrink-0 overflow-hidden transition-transform hover:scale-110"
            style="background: conic-gradient(red, yellow, lime, aqua, blue, magenta, red)"
        ></button>
    }
}
