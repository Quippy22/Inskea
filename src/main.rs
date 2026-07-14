mod app;
mod canvas;
mod hotkeys;
mod model;
mod skea;
mod tauri_bridge;
mod ui;
mod util;

use app::App;
use leptos::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App /> });
}
