//! Browser persistence for settings and the autosaved drawing. The desktop
//! app uses Tauri commands for these; the browser keeps them in localStorage.

const SETTINGS_KEY: &str = "inskea.settings";
const AUTOSAVE_KEY: &str = "inskea.autosave";
const CATALOGUE_KEY: &str = "inskea.catalogue";
const LAST_SEEN_VERSION_KEY: &str = "inskea.last_seen_version";

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

/// The browser's global-catalogue blob, or `None` before the first save.
pub fn load_catalogue() -> Option<String> {
    local_storage()?.get_item(CATALOGUE_KEY).ok().flatten()
}

/// Returns false when the browser refused the write.
pub fn save_catalogue(content: &str) -> bool {
    local_storage()
        .and_then(|storage| storage.set_item(CATALOGUE_KEY, content).ok())
        .is_some()
}

/// The version of the app the user last saw the What's New modal for.
pub fn load_last_seen_version() -> Option<String> {
    local_storage()?.get_item(LAST_SEEN_VERSION_KEY).ok().flatten()
}

/// Save the current app version as the last seen version.
pub fn save_last_seen_version(version: &str) {
    if let Some(storage) = local_storage() {
        let _ = storage.set_item(LAST_SEEN_VERSION_KEY, version);
    }
}

const PANEL_ZONES_KEY: &str = "inskea.panel_zones";

/// Loads the persisted panel zones (tools_zone, shapes_zone) from browser storage.
pub fn load_panel_zones() -> Option<(crate::ui::panel_zone::PanelZone, crate::ui::panel_zone::PanelZone)> {
    let raw = local_storage()?.get_item(PANEL_ZONES_KEY).ok().flatten()?;
    serde_json::from_str(&raw).ok()
}

/// Persists the panel zones to browser storage.
pub fn save_panel_zones(tools: crate::ui::panel_zone::PanelZone, shapes: crate::ui::panel_zone::PanelZone) {
    if let Some(storage) = local_storage() {
        if let Ok(serialized) = serde_json::to_string(&(tools, shapes)) {
            let _ = storage.set_item(PANEL_ZONES_KEY, &serialized);
        }
    }
}

