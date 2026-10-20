//! Browser persistence for settings and the autosaved drawing. The desktop
//! app uses Tauri commands for these; the browser keeps them in localStorage.

const SETTINGS_KEY: &str = "inskea.settings";
const AUTOSAVE_KEY: &str = "inskea.autosave";

fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

pub fn load_settings() -> Option<String> {
    local_storage()?.get_item(SETTINGS_KEY).ok().flatten()
}

pub fn save_settings(content: &str) {
    if let Some(storage) = local_storage() {
        let _ = storage.set_item(SETTINGS_KEY, content);
    }
}

pub fn load_autosave() -> Option<String> {
    local_storage()?.get_item(AUTOSAVE_KEY).ok().flatten()
}

/// Returns false when the browser refused the write (quota, private mode), so
/// callers must not treat the drawing as safely stored.
pub fn save_autosave(content: &str) -> bool {
    local_storage()
        .and_then(|storage| storage.set_item(AUTOSAVE_KEY, content).ok())
        .is_some()
}

pub fn clear_autosave() {
    if let Some(storage) = local_storage() {
        let _ = storage.remove_item(AUTOSAVE_KEY);
    }
}
