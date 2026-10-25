use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    pub async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    pub fn convert_file_src(path: &str) -> String;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "dialog"])]
    pub async fn save(options: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "dialog"])]
    pub async fn open(options: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], catch)]
    async fn listen(event: &str, handler: &JsValue) -> Result<JsValue, JsValue>;
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
    pick_open_path_with_filter("Inskea Drawing", &["skea"], default_dir).await
}

/// Open a native open-file dialog for an image, or `None` if cancelled.
pub async fn pick_image_path(default_dir: Option<&str>) -> Option<String> {
    pick_open_path_with_filter(
        "Image",
        &["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp"],
        default_dir,
    )
    .await
}

async fn pick_open_path_with_filter(
    filter_name: &str,
    extensions: &[&str],
    default_dir: Option<&str>,
) -> Option<String> {
    let mut opts = serde_json::json!({
        "filters": [{ "name": filter_name, "extensions": extensions }],
        "multiple": false,
    });
    if let Some(dir) = default_dir {
        opts["defaultPath"] = serde_json::json!(dir);
    }
    let args = serde_wasm_bindgen::to_value(&opts).ok()?;
    let result = open(args).await;
    result.as_string()
}

/// Read an image file as a `data:` URL, including its MIME type.
pub async fn read_image_data(path: &str) -> Result<String, String> {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({ "path": path }))
        .map_err(|e| format!("serialization error: {e}"))?;
    let result = invoke("read_image_data", args)
        .await
        .map_err(invoke_error)?;
    result
        .as_string()
        .ok_or_else(|| "failed to read image".into())
}

/// Get the default folder for drawings, `<Documents>/Inskea` (creates it if
/// missing).
pub async fn get_documents_dir() -> Result<String, String> {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({}))
        .map_err(|e| format!("serialization error: {e}"))?;
    let result = invoke("get_documents_dir", args)
        .await
        .map_err(invoke_error)?;
    result
        .as_string()
        .ok_or_else(|| "failed to get documents dir".into())
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

/// Append the `.skea` extension unless the path already has it.
pub fn with_skea_extension(path: &str) -> String {
    if path.to_ascii_lowercase().ends_with(".skea") {
        path.to_string()
    } else {
        format!("{path}.skea")
    }
}

/// Save scene content to a file at the given path.
/// Auto-appends `.skea` extension if missing.
pub async fn save_skea(path: &str, content: &str) -> Result<(), String> {
    save_file(&with_skea_extension(path), content).await
}

/// Set the native window title. Tauri windows do not follow `document.title`,
/// so the app calls this whenever the document or dirty state changes.
pub fn set_window_title(title: &str) {
    let args = match serde_wasm_bindgen::to_value(&serde_json::json!({ "title": title })) {
        Ok(args) => args,
        Err(_) => return,
    };
    wasm_bindgen_futures::spawn_local(async move {
        let _ = invoke("set_window_title", args).await;
    });
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

/// Get the paths of the installed font files (regular + italic).
/// The fonts are downloaded into the app-data dir on first launch.
pub async fn get_font_paths() -> Result<Vec<String>, String> {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({}))
        .map_err(|e| format!("serialization error: {e}"))?;
    let result = invoke("get_font_paths", args).await.map_err(invoke_error)?;
    let v: Vec<String> =
        serde_wasm_bindgen::from_value(result).map_err(|e| format!("parse error: {e}"))?;
    Ok(v)
}

/// Get the installed font files as base64 strings (regular, italic).
pub async fn get_font_data() -> Result<Vec<String>, String> {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({}))
        .map_err(|e| format!("serialization error: {e}"))?;
    let result = invoke("get_font_data", args).await.map_err(invoke_error)?;
    serde_wasm_bindgen::from_value(result).map_err(|e| format!("parse error: {e}"))
}

/// Turn an absolute file path into an `asset://` URL the webview can load.
pub fn asset_url(path: &str) -> String {
    convert_file_src(path)
}

/// Close the native window, bypassing the unsaved-changes intercept.
pub fn close_window() {
    wasm_bindgen_futures::spawn_local(async move {
        let args = JsValue::from(js_sys::Object::new());
        let _ = invoke("close_window", args).await;
    });
}

/// Register a listener for the native window close request. The Rust side
/// intercepts the OS close button and asks the frontend whether it may close.
pub fn on_close_requested(callback: impl Fn() + 'static) {
    wasm_bindgen_futures::spawn_local(async move {
        let handler = wasm_bindgen::closure::Closure::wrap(Box::new(move |_event: JsValue| {
            callback();
        }) as Box<dyn FnMut(JsValue)>);
        if listen("inskea-close-requested", handler.as_ref())
            .await
            .is_ok()
        {
            let args = JsValue::from(js_sys::Object::new());
            let _ = invoke("arm_close_guard", args).await;
            handler.forget();
        }
    });
}

/// The drawing path passed on the command line at launch, if any.
pub async fn startup_file() -> Option<String> {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({})).ok()?;
    let result = invoke("startup_file", args).await.ok()?;
    result.as_string()
}

/// Listen for a drawing path forwarded when a second instance is launched.
pub fn on_open_file(callback: impl Fn(String) + 'static) {
    wasm_bindgen_futures::spawn_local(async move {
        let handler = wasm_bindgen::closure::Closure::wrap(Box::new(move |event: JsValue| {
            if let Ok(payload) = js_sys::Reflect::get(&event, &JsValue::from_str("payload")) {
                if let Some(path) = payload.as_string() {
                    callback(path);
                }
            }
        }) as Box<dyn FnMut(JsValue)>);
        let _ = listen("inskea-open-file", handler.as_ref()).await;
        handler.forget();
    });
}

/// Read a numeric field from a JS object, accepting integers and floats.
fn number_field(object: &JsValue, key: &str) -> Option<f64> {
    js_sys::Reflect::get(object, &JsValue::from_str(key))
        .ok()
        .and_then(|value| value.as_f64())
}

/// Listen for OS file drag-and-drop. The callback receives the dropped paths
/// and the drop position in physical pixels, when the platform reports one.
pub fn on_file_drop(callback: impl Fn(Vec<String>, Option<(f64, f64)>) + 'static) {
    wasm_bindgen_futures::spawn_local(async move {
        let handler = wasm_bindgen::closure::Closure::wrap(Box::new(move |event: JsValue| {
            let Ok(payload) = js_sys::Reflect::get(&event, &JsValue::from_str("payload")) else {
                return;
            };
            let paths = js_sys::Reflect::get(&payload, &JsValue::from_str("paths"))
                .ok()
                .map(|paths| {
                    js_sys::Array::from(&paths)
                        .iter()
                        .filter_map(|value| value.as_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if paths.is_empty() {
                return;
            }
            let position = js_sys::Reflect::get(&payload, &JsValue::from_str("position"))
                .ok()
                .and_then(|position| {
                    Some((number_field(&position, "x")?, number_field(&position, "y")?))
                });
            callback(paths, position);
        }) as Box<dyn FnMut(JsValue)>);
        let _ = listen("tauri://drag-drop", handler.as_ref()).await;
        handler.forget();
    });
}
