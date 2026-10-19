use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

mod setup;

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Write a file through a sibling temp file so an interrupted save cannot
/// truncate the previous contents.
fn write_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "file path must have a parent directory".to_string())?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "file path must name a UTF-8 file".to_string())?;
    let id = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = parent.join(format!(".{name}.{}.{id}.tmp", std::process::id()));

    fs::write(&tmp, data).map_err(|e| format!("failed to write file: {e}"))?;
    if let Err(first_error) = fs::rename(&tmp, path) {
        // Windows does not replace an existing destination atomically.
        if path.exists() {
            let _ = fs::remove_file(path);
            fs::rename(&tmp, path).map_err(|e| {
                let _ = fs::remove_file(&tmp);
                format!("failed to write file: {e}")
            })
        } else {
            let _ = fs::remove_file(&tmp);
            Err(format!("failed to write file: {first_error}"))
        }
    } else {
        Ok(())
    }
}

fn validate_file_path(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    if path.as_os_str().is_empty() {
        return Err("file path cannot be empty".into());
    }
    if !path.is_absolute() {
        return Err("file path must be absolute".into());
    }
    if path.file_name().is_none() {
        return Err("file path must name a file".into());
    }
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err("file path cannot contain parent-directory components".into());
    }
    Ok(path)
}

#[tauri::command]
fn save_file(path: String, content: String) -> Result<(), String> {
    let path = validate_file_path(&path)?;
    write_atomic(&path, content.as_bytes())
}

#[tauri::command]
fn save_file_binary(path: String, data: Vec<u8>) -> Result<(), String> {
    let path = validate_file_path(&path)?;
    write_atomic(&path, &data)
}

#[tauri::command]
fn save_skea(path: String, content: String) -> Result<(), String> {
    save_file(path, content)
}

#[tauri::command]
fn load_skea(path: String) -> Result<String, String> {
    let path = validate_file_path(&path)?;
    fs::read_to_string(path).map_err(|e| format!("failed to read file: {e}"))
}

#[cfg(test)]
mod tests {
    use super::{validate_file_path, write_atomic};

    #[test]
    fn file_paths_must_be_absolute_files() {
        assert!(validate_file_path("drawing.skea").is_err());
        assert!(validate_file_path("/tmp/../drawing.skea").is_err());
        assert!(validate_file_path("/").is_err());
        assert!(validate_file_path("/tmp/drawing.skea").is_ok());
    }

    #[test]
    fn atomic_write_replaces_existing_contents() {
        let dir = std::env::temp_dir().join(format!("inskea-atomic-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("drawing.skea");

        write_atomic(&path, b"first").unwrap();
        write_atomic(&path, b"second").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"second");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[tauri::command]
fn get_documents_dir(app_handle: tauri::AppHandle) -> Result<String, String> {
    setup::documents_dir(&app_handle).map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn save_settings(app_handle: tauri::AppHandle, content: String) -> Result<(), String> {
    let path = setup::settings_file(&app_handle)?;
    write_atomic(&path, content.as_bytes())
}

#[tauri::command]
fn load_settings(app_handle: tauri::AppHandle) -> Result<String, String> {
    let path = setup::settings_file(&app_handle)?;
    if path.exists() {
        fs::read_to_string(&path).map_err(|e| format!("failed to read settings: {e}"))
    } else {
        Ok(String::new())
    }
}

#[tauri::command]
fn get_font_paths(app_handle: tauri::AppHandle) -> Result<Vec<String>, String> {
    setup::install_fonts(&app_handle).map(|paths| {
        paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            save_file,
            save_file_binary,
            save_skea,
            load_skea,
            get_documents_dir,
            save_settings,
            load_settings,
            get_font_paths
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            app.handle().plugin(tauri_plugin_dialog::init())?;
            setup::run(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
