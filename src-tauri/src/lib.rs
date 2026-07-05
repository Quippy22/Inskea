use std::fs;
use tauri::Manager;

#[tauri::command]
fn save_skea(path: String, content: String) -> Result<(), String> {
    fs::write(&path, &content).map_err(|e| format!("failed to write file: {e}"))
}

#[tauri::command]
fn load_skea(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("failed to read file: {e}"))
}

#[tauri::command]
fn get_app_data_dir(app_handle: tauri::AppHandle) -> Result<String, String> {
    let mut path = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    path.push("documents");
    fs::create_dir_all(&path).map_err(|e| format!("failed to create app data dir: {e}"))?;
    Ok(path.to_string_lossy().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![save_skea, load_skea, get_app_data_dir])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            app.handle().plugin(tauri_plugin_dialog::init())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
