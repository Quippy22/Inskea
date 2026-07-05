use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    pub async fn invoke(cmd: &str, args: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "dialog"])]
    pub async fn save(options: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "dialog"])]
    pub async fn open(options: JsValue) -> JsValue;
}

/// Open a native save-file dialog and return the chosen path, or `None` if cancelled.
/// If `default_dir` is provided, the dialog starts in that directory.
pub async fn pick_save_path(default_name: &str, default_dir: Option<&str>) -> Option<String> {
    let mut opts = serde_json::json!({
        "defaultPath": default_name,
        "filters": [{ "name": "Inskea Drawing", "extensions": ["skea"] }],
    });
    if let Some(dir) = default_dir {
        opts["defaultPath"] = serde_json::json!(format!("{dir}/{default_name}"));
    }
    let args = serde_wasm_bindgen::to_value(&opts).ok()?;
    let result = save(args).await;
    result.as_string()
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
    let result = invoke("get_app_data_dir", args).await;
    result
        .as_string()
        .ok_or_else(|| "failed to get app data dir".into())
}

/// Save scene content to a file at the given path.
/// Auto-appends `.skea` extension if missing.
pub async fn save_skea(path: &str, content: &str) -> Result<(), String> {
    let path = if path.ends_with(".skea") {
        path.to_string()
    } else {
        format!("{path}.skea")
    };
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "path": path,
        "content": content,
    }))
    .map_err(|e| format!("serialization error: {e}"))?;
    invoke("save_skea", args).await;
    Ok(())
}

/// Load scene content from a file at the given path.
pub async fn load_skea(path: &str) -> Result<String, String> {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "path": path,
    }))
    .map_err(|e| format!("serialization error: {e}"))?;
    let result = invoke("load_skea", args).await;
    result
        .as_string()
        .ok_or_else(|| "failed to load file".into())
}
