use crate::canvas::combined_bounds;
use crate::canvas::Viewport;
use crate::model::{Element, ElementId, Scene};
use crate::skea;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

// ── Browser download helpers ──────────────────────────────────────────

pub fn download_string(content: &str, filename: &str) {
    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(content));
    let blob = web_sys::Blob::new_with_str_sequence(&parts).expect("failed to create Blob");
    let url = web_sys::Url::create_object_url_with_blob(&blob).expect("failed to create URL");
    let document = web_sys::window().unwrap().document().unwrap();
    let anchor = document.create_element("a").unwrap();
    anchor.set_attribute("href", &url).ok();
    anchor.set_attribute("download", filename).ok();
    anchor.set_attribute("style", "display:none").ok();
    document.body().unwrap().append_child(&anchor).ok();
    let _ = anchor
        .dyn_ref::<web_sys::HtmlElement>()
        .map(|el| el.click());
    document.body().unwrap().remove_child(&anchor).ok();
    web_sys::Url::revoke_object_url(&url).ok();
}

/// SVG export: walks elements and builds a minimal SVG string.
/// When `selected` is `Some` and non-empty, only matching elements are
/// included and the viewBox is set to the tight bounds of the selection.
pub fn scene_to_svg(
    scene: &Scene,
    viewport: &Viewport,
    screen: (f64, f64),
    selected: Option<&[ElementId]>,
) -> String {
    let (vb, elements): (String, &[Element]) = match selected {
        Some(ids) if !ids.is_empty() => {
            let bounds = combined_bounds(ids, scene.elements());
            if let Some((bx, by, bw, bh)) = bounds {
                let pad = 10.0;
                (
                    format!(
                        "{} {} {} {}",
                        bx - pad,
                        by - pad,
                        bw + pad * 2.0,
                        bh + pad * 2.0
                    ),
                    scene.elements(),
                )
            } else {
                return String::new();
            }
        }
        _ => (viewport.to_view_box(screen.0, screen.1), scene.elements()),
    };
    let mut out = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vb}">"#);
    for el in elements {
        if selected.is_none_or(|ids| ids.contains(&el.id())) {
            out.push_str(&el_svg(el));
        }
    }
    out.push_str("</svg>");
    out
}

/// Generate SVG with a custom crop viewBox, including all elements.
pub fn scene_to_svg_crop(scene: &Scene, crop: (f64, f64, f64, f64)) -> String {
    let (cx, cy, cw, ch) = crop;
    let vb = format!("{} {} {} {}", cx, cy, cw, ch);
    let mut out = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vb}">"#);
    for el in scene.elements() {
        out.push_str(&el_svg(el));
    }
    out.push_str("</svg>");
    out
}

fn el_svg(el: &Element) -> String {
    use std::fmt::Write;
    match el {
        Element::Rectangle(r) => {
            let fill = r.data.fill_color.map(|c| c.to_hex()).unwrap_or("none");
            format!(
                r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
                r.data.world_point.x,
                r.data.world_point.y,
                r.data.width,
                r.data.height,
                fill,
                r.data.stroke_color.to_hex(),
                r.data.stroke_width,
            )
        }
        Element::Ellipse(e) => {
            let fill = e.data.fill_color.map(|c| c.to_hex()).unwrap_or("none");
            let cx = e.data.world_point.x + e.data.width / 2.0;
            let cy = e.data.world_point.y + e.data.height / 2.0;
            format!(
                r#"<ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
                cx,
                cy,
                e.data.width / 2.0,
                e.data.height / 2.0,
                fill,
                e.data.stroke_color.to_hex(),
                e.data.stroke_width,
            )
        }
        Element::Line(l) => {
            let d = crate::model::elements::path::path_d(&l.points, l.curve_mode);
            format!(
                r#"<path d="{}" fill="none" stroke="{}" stroke-width="{}"/>"#,
                d,
                l.data.stroke_color.to_hex(),
                l.data.stroke_width,
            )
        }
        Element::Arrow(a) => {
            let d = crate::model::elements::path::path_d(&a.points, a.curve_mode);
            let mut out = format!(
                r#"<path d="{}" fill="none" stroke="{}" stroke-width="{}"/>"#,
                d,
                a.data.stroke_color.to_hex(),
                a.data.stroke_width,
            );
            if a.points.len() >= 2 {
                let tail = &a.points[a.points.len() - 2];
                let tip = &a.points[a.points.len() - 1];
                let dx = tip.x - tail.x;
                let dy = tip.y - tail.y;
                let len = dx.hypot(dy);
                if len > 1.0 {
                    let ux = dx / len;
                    let uy = dy / len;
                    let hl = (a.data.stroke_width * 4.0).max(8.0);
                    let hw = hl * 0.4;
                    let bx = tip.x - ux * hl;
                    let by = tip.y - uy * hl;
                    let _ = write!(
                        out,
                        r#"<polyline points="{},{} {},{} {},{}" fill="none" stroke="{}" stroke-width="{}"/>"#,
                        tip.x,
                        tip.y,
                        bx - uy * hw,
                        by + ux * hw,
                        bx + uy * hw,
                        by - ux * hw,
                        a.data.stroke_color.to_hex(),
                        a.data.stroke_width,
                    );
                }
            }
            out
        }
        Element::Text(t) => {
            let mut out = format!(
                r#"<text x="{}" y="{}" font-size="{}" fill="{}">"#,
                t.data.world_point.x,
                t.data.world_point.y + t.data.font_size,
                t.data.font_size,
                t.data.stroke_color.to_hex(),
            );
            for (i, line) in t.wrapped.display.split('\n').enumerate() {
                let esc = line
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;");
                let dy = if i == 0 {
                    "0"
                } else {
                    &format!("{}", t.data.font_size)
                };
                let _ = write!(
                    out,
                    r#"<tspan x="{}" dy="{}">{}</tspan>"#,
                    t.data.world_point.x, dy, esc
                );
            }
            out.push_str("</text>");
            out
        }
        Element::Freehand(f) => {
            let mut d = String::new();
            let pts = &f.points;
            if !pts.is_empty() {
                let _ = write!(d, "M{} {}", pts[0].x, pts[0].y);
                if pts.len() > 1 {
                    let mx = (pts[0].x + pts[1].x) / 2.0;
                    let my = (pts[0].y + pts[1].y) / 2.0;
                    let _ = write!(d, " L{} {}", mx, my);
                    for i in 1..pts.len() - 1 {
                        let mid_x = (pts[i].x + pts[i + 1].x) / 2.0;
                        let mid_y = (pts[i].y + pts[i + 1].y) / 2.0;
                        let _ = write!(d, " Q{} {} {} {}", pts[i].x, pts[i].y, mid_x, mid_y);
                    }
                    let _ = write!(d, " L{} {}", pts[pts.len() - 1].x, pts[pts.len() - 1].y);
                }
            }
            format!(
                r#"<path d="{}" fill="none" stroke="{}" stroke-width="{}" stroke-linecap="round"/>"#,
                d,
                f.data.stroke_color.to_hex(),
                f.data.stroke_width,
            )
        }
    }
}

/// Create a JS Promise that resolves when the image finishes loading.
fn image_load_promise(img: &web_sys::HtmlImageElement, url: &str) -> js_sys::Promise {
    let img2 = img.clone();
    let img3 = img.clone();
    let url2 = String::from(url);
    js_sys::Promise::new(
        &mut move |resolve: js_sys::Function, reject: js_sys::Function| {
            let onload_cb = wasm_bindgen::closure::Closure::once(move || {
                let _ = resolve.call0(&JsValue::null());
            });
            let onerror_cb = wasm_bindgen::closure::Closure::once(move || {
                let _ = reject.call0(&JsValue::null());
            });
            img2.set_onload(Some(onload_cb.as_ref().unchecked_ref()));
            onload_cb.forget();
            img3.set_onerror(Some(onerror_cb.as_ref().unchecked_ref()));
            onerror_cb.forget();
            img2.set_src(&url2);
        },
    )
}

/// Create a JS Promise that resolves with the blob from canvas.to_blob().
fn canvas_to_blob_promise(canvas: &web_sys::HtmlCanvasElement) -> js_sys::Promise {
    let c = canvas.clone();
    js_sys::Promise::new(
        &mut move |resolve: js_sys::Function, _reject: js_sys::Function| {
            let cb = wasm_bindgen::closure::Closure::once(move |blob: Option<web_sys::Blob>| {
                let val = match blob {
                    Some(b) => JsValue::from(b),
                    None => JsValue::null(),
                };
                let _ = resolve.call1(&JsValue::null(), &val);
            });
            let _ = c.to_blob(cb.as_ref().unchecked_ref());
            cb.forget();
        },
    )
}

fn svg_blob(svg: &str) -> web_sys::Blob {
    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(svg));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("image/svg+xml");
    web_sys::Blob::new_with_str_sequence_and_options(&parts, &opts).expect("blob")
}

/// Shared SVG→canvas→blob pipeline used by both browser and Tauri PNG export.
async fn svg_to_png_blob(svg: &str) -> Option<web_sys::Blob> {
    let document = web_sys::window().unwrap().document().unwrap();
    let blob = svg_blob(svg);
    let url = web_sys::Url::create_object_url_with_blob(&blob).expect("url");
    let img = document
        .create_element("img")
        .unwrap()
        .dyn_into::<web_sys::HtmlImageElement>()
        .unwrap();
    let _ = wasm_bindgen_futures::JsFuture::from(image_load_promise(&img, &url)).await;
    let canvas = document
        .create_element("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
    let (w, h) = (img.natural_width() as u32, img.natural_height() as u32);
    canvas.set_width(w.max(1));
    canvas.set_height(h.max(1));
    let ctx = canvas
        .get_context("2d")
        .ok()
        .flatten()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();
    let _ = ctx.draw_image_with_html_image_element(&img, 0.0, 0.0);
    let png = wasm_bindgen_futures::JsFuture::from(canvas_to_blob_promise(&canvas))
        .await
        .ok()
        .and_then(|v| v.dyn_into::<web_sys::Blob>().ok());
    web_sys::Url::revoke_object_url(&url).ok();
    png
}

pub fn download_png_from_svg(svg: String, filename: String) {
    spawn_local(async move {
        let document = web_sys::window().unwrap().document().unwrap();
        if let Some(blob) = svg_to_png_blob(&svg).await {
            let url2 = web_sys::Url::create_object_url_with_blob(&blob).expect("url2");
            let a = document.create_element("a").unwrap();
            a.set_attribute("href", &url2).ok();
            a.set_attribute("download", &filename).ok();
            a.set_attribute("style", "display:none").ok();
            document.body().unwrap().append_child(&a).ok();
            let _ = a.dyn_ref::<web_sys::HtmlElement>().map(|e| e.click());
            document.body().unwrap().remove_child(&a).ok();
            web_sys::Url::revoke_object_url(&url2).ok();
        }
    });
}

pub fn browser_export_skea(scene: Scene) {
    let content = skea::save_to_string(&scene);
    download_string(&content, "untitled.skea");
}

/// Tauri-native SVG export: shows a save dialog, writes the SVG string to disk.
pub fn tauri_export_svg(svg: String, selection: bool) {
    spawn_local(async move {
        let dir = crate::tauri_bridge::get_app_data_dir().await.ok();
        let name = if selection {
            "selection.svg"
        } else {
            "canvas.svg"
        };
        let path = crate::tauri_bridge::pick_save_path_svg(name, dir.as_deref()).await;
        if let Some(path) = path {
            let _ = crate::tauri_bridge::save_file(&path, &svg).await;
        }
    });
}

/// Tauri-native PNG export: renders SVG to canvas, converts to blob, saves as PNG.
pub fn tauri_export_png(svg: String, selection: bool) {
    spawn_local(async move {
        if let Some(blob) = svg_to_png_blob(&svg).await {
            let name = if selection {
                "selection.png"
            } else {
                "canvas.png"
            };
            let dir = crate::tauri_bridge::get_app_data_dir().await.ok();
            let path = crate::tauri_bridge::pick_save_path_png(name, dir.as_deref()).await;
            if let Some(path) = path {
                let ab_promise = js_sys::Promise::new(&mut {
                    let blob2 = blob.clone();
                    move |resolve: js_sys::Function, _reject: js_sys::Function| {
                        let reader = web_sys::FileReader::new().expect("FileReader");
                        let reader_c = reader.clone();
                        let cb = wasm_bindgen::closure::Closure::once(move || {
                            let val = reader_c.result().unwrap_or(JsValue::null());
                            let _ = resolve.call1(&JsValue::null(), &val);
                        });
                        reader.set_onload(Some(cb.as_ref().unchecked_ref()));
                        cb.forget();
                        let _ = reader.read_as_array_buffer(&blob2);
                    }
                });
                if let Ok(val) = wasm_bindgen_futures::JsFuture::from(ab_promise).await {
                    let uint8 = js_sys::Uint8Array::new(&val);
                    let bytes = uint8.to_vec();
                    let _ = crate::tauri_bridge::save_file_binary(&path, &bytes).await;
                }
            }
        }
    });
}
