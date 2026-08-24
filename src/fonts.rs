use crate::tauri_bridge;
use wasm_bindgen_futures::spawn_local;

/// Install the app font at runtime: resolve the downloaded font files in the
/// app-data dir and register `@font-face` rules that load them over the asset
/// protocol. Best-effort — a missing install falls back to the system font.
pub fn ensure_fonts_installed() {
    if !tauri_bridge::is_tauri() {
        return;
    }
    spawn_local(async move {
        let Ok(paths) = tauri_bridge::get_font_paths().await else {
            return;
        };
        let (Some(regular), Some(italic)) = (paths.first(), paths.get(1)) else {
            return;
        };
        let family = crate::model::elements::text::FONT_FAMILY;
        let regular = tauri_bridge::asset_url(regular);
        let italic = tauri_bridge::asset_url(italic);
        let css = format!(
            "@font-face {{ font-family: \"{family}\"; src: url(\"{regular}\") format(\"truetype\"); \
             font-weight: 100 900; font-style: normal; }}\n\
             @font-face {{ font-family: \"{family}\"; src: url(\"{italic}\") format(\"truetype\"); \
             font-weight: 100 900; font-style: italic; }}"
        );
        let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let Some(head) = doc.head() else {
            return;
        };
        if let Ok(style) = doc.create_element("style") {
            style.set_text_content(Some(&css));
            let _ = head.append_child(&style);
        }
    });
}