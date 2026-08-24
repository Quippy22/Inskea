//! Runtime setup helpers: enforce the app's on-disk directory layout and
//! install bundled dependencies (fonts) on first launch.
//!
//! Called from Tauri's `.setup()` hook before the window is shown, and
//! reused by the file commands so the app-data directory (and the sub-dirs
//! it relies on) always exist before anything reads or writes them.

use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// Font files the app needs, with their upstream sources.
const FONTS: &[(&str, &str)] = &[
    (
        "Cabin-Variable.ttf",
        "https://raw.githubusercontent.com/google/fonts/main/ofl/cabin/Cabin%5Bwdth%2Cwght%5D.ttf",
    ),
    (
        "Cabin-Italic-Variable.ttf",
        "https://raw.githubusercontent.com/google/fonts/main/ofl/cabin/Cabin-Italic%5Bwdth%2Cwght%5D.ttf",
    ),
    (
        "OFL.txt",
        "https://raw.githubusercontent.com/google/fonts/main/ofl/cabin/OFL.txt",
    ),
];

/// Resolve the app-data directory, creating it if needed.
pub fn app_data_dir(handle: &AppHandle) -> Result<PathBuf, String> {
    let dir = handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create app data dir: {e}"))?;
    Ok(dir)
}

/// Resolve the user's documents directory inside app-data, creating it if needed.
pub fn documents_dir(handle: &AppHandle) -> Result<PathBuf, String> {
    let mut dir = app_data_dir(handle)?;
    dir.push("documents");
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create documents dir: {e}"))?;
    Ok(dir)
}

/// Resolve the settings file path, creating its parent directory if needed.
pub fn settings_file(handle: &AppHandle) -> Result<PathBuf, String> {
    let mut dir = app_data_dir(handle)?;
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create settings dir: {e}"))?;
    dir.push("settings.toml");
    Ok(dir)
}

/// Resolve the bundled-fonts directory inside app-data, creating it if needed.
pub fn fonts_dir(handle: &AppHandle) -> Result<PathBuf, String> {
    let mut dir = app_data_dir(handle)?;
    dir.push("fonts");
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create fonts dir: {e}"))?;
    Ok(dir)
}

fn download(url: &str, file: &Path) -> Result<(), String> {
    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(30)))
            .build(),
    );
    let mut resp = agent
        .get(url)
        .call()
        .map_err(|e| format!("request failed: {e}"))?;
    let tmp = file.with_extension("tmp");
    let mut output = fs::File::create(&tmp).map_err(|e| format!("failed to create file: {e}"))?;
    std::io::copy(&mut resp.body_mut().as_reader(), &mut output)
        .map_err(|e| format!("failed to write file: {e}"))?;
    fs::rename(&tmp, file).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("failed to finalize file: {e}")
    })
}

/// Download any missing font files into the app-data fonts dir. Idempotent.
/// Returns the installed font file paths (regular + italic, in that order).
pub fn install_fonts(handle: &AppHandle) -> Result<Vec<PathBuf>, String> {
    let dir = fonts_dir(handle)?;
    for (name, url) in FONTS {
        let file = dir.join(name);
        if !file.exists() {
            download(url, &file)?;
        }
    }
    Ok(FONTS
        .iter()
        .filter(|(name, _)| name.ends_with(".ttf"))
        .map(|(name, _)| dir.join(name))
        .collect())
}

/// Create any directories the app relies on, and install bundled fonts.
/// Idempotent — safe on every launch. Font install is best-effort: a missing
/// or failed download must not block the app (fallback font is used instead).
pub fn run(handle: &AppHandle) -> Result<(), String> {
    documents_dir(handle)?;
    settings_file(handle)?;
    if let Err(e) = install_fonts(handle) {
        log::warn!("font install failed, falling back to system font: {e}");
    }
    Ok(())
}