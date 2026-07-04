use leptos::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[component]
pub fn App() -> impl IntoView {
    let on_click = move |_| {
        spawn_local(async {
            invoke("greet", JsValue::NULL).await;
        });
    };

    view! {
        <button on:click=on_click class="bg-emerald-500 text-white px-4 py-2 rounded">
            "Say hello to Rust console"
        </button>
    }
}
