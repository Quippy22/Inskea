use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use tauri::{Emitter, Manager};

mod setup;

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Close-request state shared with the webview.
///
/// `armed` becomes true once the frontend listener is registered, so a close
/// during startup is not swallowed. `confirmed` is set when the frontend has
/// resolved the unsaved-changes prompt and wants the window to close.
struct CloseGuard {
    confirmed: AtomicBool,
    armed: AtomicBool,
}

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
    use super::{is_skea_arg, resolve_arg, validate_file_path, write_atomic};
    use std::path::Path;

    #[test]
    fn file_paths_must_be_absolute_files() {
        assert!(validate_file_path("drawing.skea").is_err());
        assert!(validate_file_path("/tmp/../drawing.skea").is_err());
        assert!(validate_file_path("/").is_err());
        assert!(validate_file_path("/tmp/drawing.skea").is_ok());
    }

    #[test]
    fn command_line_drawing_arguments_are_recognized() {
        assert!(is_skea_arg("drawing.skea"));
        assert!(is_skea_arg("file:///tmp/drawing.skea"));
        assert!(is_skea_arg("DRAWING.SKEA"));
        assert!(!is_skea_arg("--autosave"));
        assert!(!is_skea_arg("notes.txt"));
    }

    #[test]
    fn relative_drawing_arguments_resolve_against_the_cwd() {
        assert_eq!(
            resolve_arg("drawing.skea", Some(Path::new("/home/user"))).as_deref(),
            Some("/home/user/drawing.skea")
        );
        assert_eq!(
            resolve_arg("file:///tmp/drawing.skea", None).as_deref(),
            Some("/tmp/drawing.skea")
        );
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
fn set_window_title(window: tauri::Window, title: String) {
    let _ = window.set_title(&title);
}

/// Close the window after the frontend has resolved the unsaved-changes
/// prompt. The close request event handler checks this flag so the second
/// close attempt goes through.
#[tauri::command]
fn close_window(window: tauri::Window, state: tauri::State<CloseGuard>) {
    state.confirmed.store(true, Ordering::SeqCst);
    let _ = window.close();
}

/// Let close requests be intercepted. Called by the frontend only after its
/// close listener is registered; until then the window closes normally.
#[tauri::command]
fn arm_close_guard(state: tauri::State<CloseGuard>) {
    state.armed.store(true, Ordering::SeqCst);
}

#[tauri::command]
fn startup_file() -> Option<String> {
    let cwd = std::env::current_dir().ok();
    std::env::args()
        .skip(1)
        .find(|arg| is_skea_arg(arg))
        .and_then(|arg| resolve_arg(&arg, cwd.as_deref()))
}

/// Check whether a command-line argument names an Inskea drawing.
fn is_skea_arg(arg: &str) -> bool {
    if arg.starts_with('-') {
        return false;
    }
    arg.strip_prefix("file://")
        .unwrap_or(arg)
        .to_ascii_lowercase()
        .ends_with(".skea")
}

/// Turn an OS argument into an absolute path. File managers usually pass an
/// absolute path, but a relative path or `file://` URI is possible.
fn resolve_arg(arg: &str, cwd: Option<&Path>) -> Option<String> {
    let arg = arg.strip_prefix("file://").unwrap_or(arg);
    let path = Path::new(arg);
    let full = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd?.join(path)
    };
    full.to_str().map(str::to_string)
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
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            if let Some(path) = argv
                .iter()
                .skip(1)
                .find(|arg| is_skea_arg(arg))
                .and_then(|arg| resolve_arg(arg, Some(Path::new(&cwd))))
            {
                let _ = app.emit("inskea-open-file", path);
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .manage(CloseGuard {
            confirmed: AtomicBool::new(false),
            armed: AtomicBool::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            save_file,
            save_file_binary,
            save_skea,
            load_skea,
            set_window_title,
            close_window,
            arm_close_guard,
            startup_file,
            get_documents_dir,
            save_settings,
            load_settings,
            get_font_paths
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let guard = window.app_handle().state::<CloseGuard>();
                if guard.armed.load(Ordering::SeqCst)
                    && !guard.confirmed.load(Ordering::SeqCst)
                {
                    api.prevent_close();
                    let _ = window.emit("inskea-close-requested", ());
                }
            }
        })
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
