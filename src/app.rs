use leptos::*;
use crate::canvas::Canvas;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <div class="w-screen h-screen">
            <Canvas/>
        </div>
    }
}
