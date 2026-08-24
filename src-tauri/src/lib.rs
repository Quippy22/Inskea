use std::fs;

mod setup;

#[tauri::command]
fn save_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, &content).map_err(|e| format!("failed to write file: {e}"))
}

#[tauri::command]
fn save_file_binary(path: String, data: Vec<u8>) -> Result<(), String> {
    fs::write(&path, &data).map_err(|e| format!("failed to write file: {e}"))
}

#[tauri::command]
fn save_skea(path: String, content: String) -> Result<(), String> {
    save_file(path, content)
}

#[tauri::command]
fn load_skea(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("failed to read file: {e}"))
}

#[tauri::command]
fn get_app_data_dir(app_handle: tauri::AppHandle) -> Result<String, String> {
    setup::documents_dir(&app_handle).map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn save_settings(app_handle: tauri::AppHandle, content: String) -> Result<(), String> {
    let path = setup::settings_file(&app_handle)?;
    fs::write(&path, &content).map_err(|e| format!("failed to write settings: {e}"))
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
        .invoke_handler(tauri::generate_handler![save_file, save_file_binary, save_skea, load_skea, get_app_data_dir, save_settings, load_settings, get_font_paths])
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
