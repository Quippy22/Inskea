mod app;
mod canvas;
mod model;
mod tauri_bridge;

use app::App;
use leptos::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> });
}
