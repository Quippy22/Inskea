use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use tauri::{Emitter, Manager};

mod setup;

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Give the process the same AppUserModelID the NSIS shortcut uses, so the
/// taskbar, jump lists, and notifications group under the installed app.
#[cfg(windows)]
fn set_app_user_model_id(identifier: &str) {
    use windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

    let app_id: Vec<u16> = identifier.encode_utf16().chain(std::iter::once(0)).collect();
    // A failed call leaves the default process identity in place; startup continues.
    let _ = unsafe { SetCurrentProcessExplicitAppUserModelID(app_id.as_ptr()) };
}

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

/// Image formats the editor can embed, with the MIME type used in their data URL.
const IMAGE_MIME_TYPES: &[(&str, &str)] = &[
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("webp", "image/webp"),
    ("svg", "image/svg+xml"),
    ("bmp", "image/bmp"),
];

/// The MIME type for a supported image path, or `None` for other files.
fn image_mime(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    IMAGE_MIME_TYPES
        .iter()
        .find(|(name, _)| *name == extension)
        .map(|(_, mime)| *mime)
}

/// Read an image file and return it as a data URL the canvas can render.
#[tauri::command]
fn read_image_data(path: String) -> Result<String, String> {
    use base64::Engine;

    let path = validate_file_path(&path)?;
    let mime = image_mime(&path)
        .ok_or_else(|| "unsupported image type (expected PNG, JPEG, GIF, WebP, SVG, or BMP)".to_string())?;
    let bytes = fs::read(path).map_err(|e| format!("failed to read image: {e}"))?;
    let data = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{mime};base64,{data}"))
}

#[cfg(test)]
mod tests {
    use super::{image_mime, is_skea_arg, resolve_arg, validate_file_path, write_atomic};
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
    fn image_extensions_map_to_mime_types() {
        assert_eq!(image_mime(Path::new("/tmp/photo.PNG")), Some("image/png"));
        assert_eq!(image_mime(Path::new("/tmp/logo.svg")), Some("image/svg+xml"));
        assert_eq!(image_mime(Path::new("/tmp/scan.jpeg")), Some("image/jpeg"));
        assert_eq!(image_mime(Path::new("/tmp/drawing.skea")), None);
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
    // `args()` panics on non-UTF-8 arguments; a file path may not be UTF-8.
    std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
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

/// Return the installed font files as base64, so PNG export can embed them in
/// the intermediate SVG. An SVG loaded into an `<img>` cannot use the app's
/// `@font-face` rules, and Cabin is not installed system-wide.
#[tauri::command]
fn get_font_data(app_handle: tauri::AppHandle) -> Result<Vec<String>, String> {
    use base64::Engine;
    let mut fonts = Vec::new();
    for path in setup::installed_font_paths(&app_handle)? {
        // Keep the regular/italic positions meaningful: stop if one is
        // missing rather than returning the italic font as index 0.
        if !path.is_file() {
            break;
        }
        let bytes = fs::read(&path).map_err(|e| format!("failed to read font: {e}"))?;
        fonts.push(base64::engine::general_purpose::STANDARD.encode(bytes));
    }
    Ok(fonts)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let context = tauri::generate_context!();

    #[cfg(windows)]
    set_app_user_model_id(context.config().identifier.as_str());

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
            read_image_data,
            set_window_title,
            close_window,
            arm_close_guard,
            startup_file,
            get_documents_dir,
            save_settings,
            load_settings,
            get_font_paths,
            get_font_data
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let guard = window.app_handle().state::<CloseGuard>();
                if guard.armed.load(Ordering::SeqCst) && !guard.confirmed.load(Ordering::SeqCst) {
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
        .run(context)
        .expect("error while running tauri application");
}
