use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    pub async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "dialog"])]
    pub async fn save(options: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "dialog"])]
    pub async fn open(options: JsValue) -> JsValue;
}

fn invoke_error(e: JsValue) -> String {
    e.as_string().unwrap_or_else(|| format!("{e:?}"))
}

/// Check whether the app is running inside a Tauri webview.
pub fn is_tauri() -> bool {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return false,
    };
    js_sys::Reflect::get(&window, &JsValue::from_str("__TAURI__"))
        .map(|v| !v.is_undefined() && !v.is_null())
        .unwrap_or(false)
}

async fn pick_save_path_with_filter(
    default_name: &str,
    default_dir: Option<&str>,
    filter_name: &str,
    ext: &str,
) -> Option<String> {
    let mut opts = serde_json::json!({
        "defaultPath": default_name,
        "filters": [{ "name": filter_name, "extensions": [ext] }],
    });
    if let Some(dir) = default_dir {
        opts["defaultPath"] = serde_json::json!(format!("{dir}/{default_name}"));
    }
    let args = serde_wasm_bindgen::to_value(&opts).ok()?;
    let result = save(args).await;
    result.as_string()
}

/// Open a native save-file dialog for an Inskea drawing (.skea).
pub async fn pick_save_path(default_name: &str, default_dir: Option<&str>) -> Option<String> {
    pick_save_path_with_filter(default_name, default_dir, "Inskea Drawing", "skea").await
}

/// Open a native save-file dialog for SVG export.
pub async fn pick_save_path_svg(default_name: &str, default_dir: Option<&str>) -> Option<String> {
    pick_save_path_with_filter(default_name, default_dir, "SVG Image", "svg").await
}

/// Open a native save-file dialog for PNG export.
pub async fn pick_save_path_png(default_name: &str, default_dir: Option<&str>) -> Option<String> {
    pick_save_path_with_filter(default_name, default_dir, "PNG Image", "png").await
}

/// Open a native open-file dialog and return the chosen path, or `None` if cancelled.
/// If `default_dir` is provided, the dialog starts in that directory.
pub async fn pick_open_path(default_dir: Option<&str>) -> Option<String> {
    let mut opts = serde_json::json!({
        "filters": [{ "name": "Inskea Drawing", "extensions": ["skea"] }],
        "multiple": false,
    });
    if let Some(dir) = default_dir {
        opts["defaultPath"] = serde_json::json!(dir);
    }
    let args = serde_wasm_bindgen::to_value(&opts).ok()?;
    let result = open(args).await;
    result.as_string()
}

/// Get the platform-specific app data directory (creates it if missing).
pub async fn get_app_data_dir() -> Result<String, String> {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({}))
        .map_err(|e| format!("serialization error: {e}"))?;
    let result = invoke("get_app_data_dir", args)
        .await
        .map_err(invoke_error)?;
    result
        .as_string()
        .ok_or_else(|| "failed to get app data dir".into())
}

/// Save content to a file at the given path (no extension enforcement).
pub async fn save_file(path: &str, content: &str) -> Result<(), String> {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "path": path,
        "content": content,
    }))
    .map_err(|e| format!("serialization error: {e}"))?;
    invoke("save_file", args).await.map_err(invoke_error)?;
    Ok(())
}

/// Save binary data to a file at the given path.
pub async fn save_file_binary(path: &str, data: &[u8]) -> Result<(), String> {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "path": path,
        "data": data,
    }))
    .map_err(|e| format!("serialization error: {e}"))?;
    invoke("save_file_binary", args)
        .await
        .map_err(invoke_error)?;
    Ok(())
}

/// Save scene content to a file at the given path.
/// Auto-appends `.skea` extension if missing.
pub async fn save_skea(path: &str, content: &str) -> Result<(), String> {
    let path = if path.ends_with(".skea") {
        path.to_string()
    } else {
        format!("{path}.skea")
    };
    save_file(&path, content).await
}

/// Save settings (TOML string) to the app data directory.
pub async fn save_settings(content: &str) -> Result<(), String> {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "content": content,
    }))
    .map_err(|e| format!("serialization error: {e}"))?;
    invoke("save_settings", args).await.map_err(invoke_error)?;
    Ok(())
}

/// Load settings (TOML string) from the app data directory.
pub async fn load_settings() -> Result<String, String> {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({}))
        .map_err(|e| format!("serialization error: {e}"))?;
    let result = invoke("load_settings", args).await.map_err(invoke_error)?;
    result
        .as_string()
        .ok_or_else(|| "failed to load settings".into())
}

/// Load scene content from a file at the given path.
pub async fn load_skea(path: &str) -> Result<String, String> {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "path": path,
    }))
    .map_err(|e| format!("serialization error: {e}"))?;
    let result = invoke("load_skea", args).await.map_err(invoke_error)?;
    result
        .as_string()
        .ok_or_else(|| "failed to load file".into())
}
