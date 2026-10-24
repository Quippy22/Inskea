use crate::canvas::settings::CanvasBg;
use crate::canvas::{combined_bounds, Viewport};
use crate::model::{Element, ElementId, Point, Rect, Scene};
use crate::ui::notify::{self, Notifications};
use leptos::set_timeout;
use std::time::Duration;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

fn with_extension(path: String, extension: &str) -> String {
    if path
        .to_ascii_lowercase()
        .ends_with(&format!(".{extension}"))
    {
        path
    } else {
        format!("{path}.{extension}")
    }
}

fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

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
    // Revoke after a beat: some browsers read the URL lazily.
    set_timeout(
        move || {
            web_sys::Url::revoke_object_url(&url).ok();
        },
        Duration::from_secs(1),
    );
}

/// SVG export: walks elements and builds a minimal SVG string.
/// When `selected` is `Some` and non-empty, only matching elements are
/// included and the viewBox is set to the tight bounds of the selection.
pub fn scene_to_svg(
    scene: &Scene,
    viewport: &Viewport,
    screen: Point,
    selected: Option<&[ElementId]>,
    bg: CanvasBg,
) -> String {
    let (view, width, height, elements): (Rect, f64, f64, Vec<Element>) = match selected {
        Some(ids) if !ids.is_empty() => {
            let elements: Vec<_> = ids
                .iter()
                .filter_map(|id| scene.element_by_id(*id))
                .collect();
            let bounds = combined_bounds(ids, &elements);
            if let Some(b) = bounds {
                let pad = 10.0;
                (
                    Rect::new(b.x - pad, b.y - pad, b.w + pad * 2.0, b.h + pad * 2.0),
                    b.w + pad * 2.0,
                    b.h + pad * 2.0,
                    elements,
                )
            } else {
                return String::new();
            }
        }
        _ => {
            let view = viewport.visible_rect(screen);
            (view, screen.x, screen.y, scene.elements_in_view(view))
        }
    };
    let element_body = svg_body(&elements, selected);
    svg_document(view, width, height, bg, &element_body)
}

pub fn scene_to_svg_crop(scene: &Scene, crop: Rect, bg: CanvasBg) -> String {
    let element_body = svg_body(&scene.elements_in_view(crop), None);
    svg_document(crop, crop.w, crop.h, bg, &element_body)
}

/// The on-screen canvas background. Exports paint it too: the default drawing
/// colour is white, which would be invisible on a viewer's white backdrop if
/// the image were transparent.
fn bg_color(bg: CanvasBg) -> &'static str {
    match bg {
        CanvasBg::Dark => "#1a1b26",
        CanvasBg::Light => "#ffffff",
    }
}

/// `width`/`height` give the SVG an intrinsic pixel size. Without them an SVG
/// loaded into an `<img>` falls back to a default 300×150 raster for PNG
/// export, regardless of the drawing's size.
fn svg_document(view: Rect, width: f64, height: f64, bg: CanvasBg, body: &str) -> String {
    let (x, y, w, h) = (view.x, view.y, view.w, view.h);
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="{x} {y} {w} {h}"><rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{}"/>{body}</svg>"#,
        bg_color(bg)
    )
}

/// Escape text content for XML. Attribute values in this exporter are numeric
/// or palette-derived, so only text content needs this.
fn escape_xml_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn svg_body(elements: &[Element], selected: Option<&[ElementId]>) -> String {
    let mut out = String::new();
    for el in elements {
        if selected.is_none_or(|ids| ids.contains(&el.id())) {
            out.push_str(&el_svg(el));
        }
    }
    out
}

fn el_svg(el: &Element) -> String {
    use std::fmt::Write;
    match el {
        Element::Rectangle(r) => {
            let fill = r
                .data
                .style
                .fill_color
                .as_ref()
                .map(|c| c.to_hex())
                .unwrap_or_else(|| "none".to_string());
            let (rx, ry) = match r.data.style.edge_style {
                crate::model::elements::EdgeStyle::Rounded => {
                    (r.data.style.roundness, r.data.style.roundness)
                }
                crate::model::elements::EdgeStyle::Sharp => (0.0, 0.0),
            };
            let rect = format!(
                r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" ry="{}" fill="{}" stroke="{}" stroke-width="{}" stroke-dasharray="{}" stroke-linejoin="{}" opacity="{}"/>"#,
                r.data.world_point.x,
                r.data.world_point.y,
                r.data.width,
                r.data.height,
                rx,
                ry,
                fill,
                r.data.style.stroke_color.to_hex(),
                r.data.style.stroke_width,
                r.data.style.stroke_style.stroke_dasharray(),
                r.data.style.edge_style.stroke_linejoin(),
                r.data.style.opacity,
            );
            if r.data.rotation == 0.0 {
                rect
            } else {
                let cx = r.data.world_point.x + r.data.width / 2.0;
                let cy = r.data.world_point.y + r.data.height / 2.0;
                let deg = r.data.rotation.to_degrees();
                format!(r#"<g transform="rotate({deg} {cx} {cy})">{rect}</g>"#)
            }
        }
        Element::Ellipse(e) => {
            let fill = e
                .data
                .style
                .fill_color
                .as_ref()
                .map(|c| c.to_hex())
                .unwrap_or_else(|| "none".to_string());
            let cx = e.data.world_point.x + e.data.width / 2.0;
            let cy = e.data.world_point.y + e.data.height / 2.0;
            let ellipse = format!(
                r#"<ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="{}" stroke="{}" stroke-width="{}" stroke-dasharray="{}" stroke-linejoin="{}" opacity="{}"/>"#,
                cx,
                cy,
                e.data.width / 2.0,
                e.data.height / 2.0,
                fill,
                e.data.style.stroke_color.to_hex(),
                e.data.style.stroke_width,
                e.data.style.stroke_style.stroke_dasharray(),
                e.data.style.edge_style.stroke_linejoin(),
                e.data.style.opacity,
            );
            if e.data.rotation == 0.0 {
                ellipse
            } else {
                let deg = e.data.rotation.to_degrees();
                format!(r#"<g transform="rotate({deg} {cx} {cy})">{ellipse}</g>"#)
            }
        }
        Element::Line(l) => {
            let d = crate::model::elements::path::path_d(&l.points, l.line_style.curve_mode);
            let opacity = l.data.style.opacity;
            let linejoin = l.data.style.edge_style.stroke_linejoin();
            let linecap = match l.data.style.edge_style {
                crate::model::elements::EdgeStyle::Sharp => "butt",
                crate::model::elements::EdgeStyle::Rounded => "round",
            };
            let dash = l.data.style.stroke_style.stroke_dasharray();
            // Same stroke attributes as the on-screen render, including the
            // arrowheads (which inherit them from their group there).
            let attrs = format!(
                r#"stroke-linecap="{linecap}" stroke-linejoin="{linejoin}" stroke-dasharray="{dash}""#
            );
            let mut out = format!(
                r#"<path d="{}" fill="none" stroke="{}" stroke-width="{}" {attrs} opacity="{}"/>"#,
                d,
                l.data.style.stroke_color.to_hex(),
                l.data.style.stroke_width,
                opacity,
            );
            if l.points.len() >= 2 {
                let sw = l.data.style.stroke_width;
                if l.line_style.has_start_arrowhead {
                    let pts = crate::model::elements::utils::arrowhead_polyline(
                        &l.points[1],
                        &l.points[0],
                        sw,
                    );
                    if !pts.is_empty() {
                        let _ = write!(
                            out,
                            r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="{}" {attrs} opacity="{}"/>"#,
                            pts,
                            l.data.style.stroke_color.to_hex(),
                            sw,
                            opacity,
                        );
                    }
                }
                if l.line_style.has_end_arrowhead {
                    let pts = crate::model::elements::utils::arrowhead_polyline(
                        &l.points[l.points.len() - 2],
                        &l.points[l.points.len() - 1],
                        sw,
                    );
                    if !pts.is_empty() {
                        let _ = write!(
                            out,
                            r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="{}" {attrs} opacity="{}"/>"#,
                            pts,
                            l.data.style.stroke_color.to_hex(),
                            sw,
                            opacity,
                        );
                    }
                }
            }
            out
        }
        Element::Text(t) => {
            let anchor = t.align.anchor();
            let anchor_x = match t.align {
                crate::model::TextAlign::Left => t.data.world_point.x,
                crate::model::TextAlign::Center => t.data.world_point.x + t.data.width / 2.0,
                crate::model::TextAlign::Right => t.data.world_point.x + t.data.width,
            };
            // Clamp exactly like the on-screen render, so the exported text
            // sits on the same baseline at the same size.
            let fs = t
                .data
                .style
                .font_size
                .max(crate::model::elements::text::MIN_FONT_SIZE);
            let mut out = format!(
                r#"<text x="{}" y="{}" text-anchor="{}" font-size="{}" font-family="{}" fill="{}" opacity="{}">"#,
                anchor_x,
                t.data.world_point.y + fs * crate::model::elements::text::TEXT_ASCENT_RATIO,
                anchor,
                fs,
                crate::model::elements::text::FONT_STACK,
                t.data.style.stroke_color.to_hex(),
                t.data.style.opacity,
            );
            for (i, line) in t.wrapped.lines.iter().enumerate() {
                let esc = escape_xml_text(line);
                let dy = if i == 0 { 0.0 } else { fs * 1.2 };
                let _ = write!(
                    out,
                    r#"<tspan x="{}" dy="{}">{}</tspan>"#,
                    anchor_x, dy, esc
                );
            }
            out.push_str("</text>");
            if t.data.rotation == 0.0 {
                out
            } else {
                let cx = t.data.world_point.x + t.data.width / 2.0;
                let cy = t.data.world_point.y + t.data.height / 2.0;
                let deg = t.data.rotation.to_degrees();
                format!(r#"<g transform="rotate({deg} {cx} {cy})">{out}</g>"#)
            }
        }
        Element::Freehand(f) => {
            // Reuse the on-screen smoothing so the exported stroke has the
            // same shape, and mirror the stroke attributes exactly.
            let d = crate::model::elements::freehand::build_smooth_path(&f.points);
            let linejoin = f.data.style.edge_style.stroke_linejoin();
            let linecap = match f.data.style.edge_style {
                crate::model::elements::EdgeStyle::Sharp => "butt",
                crate::model::elements::EdgeStyle::Rounded => "round",
            };
            format!(
                r#"<path d="{}" fill="none" stroke="{}" stroke-width="{}" stroke-linecap="{}" stroke-linejoin="{}" stroke-dasharray="{}" opacity="{}"/>"#,
                d,
                f.data.style.stroke_color.to_hex(),
                f.data.style.stroke_width,
                linecap,
                linejoin,
                f.data.style.stroke_style.stroke_dasharray(),
                f.data.style.opacity,
            )
        }
        Element::Image(i) => {
            // The data URL is embedded in the document, so exports stay
            // self-contained and work when loaded as an <img> for PNG.
            let image = format!(
                r#"<image x="{}" y="{}" width="{}" height="{}" href="{}" preserveAspectRatio="none" opacity="{}"/>"#,
                i.data.world_point.x,
                i.data.world_point.y,
                i.data.width,
                i.data.height,
                i.src,
                i.data.style.opacity,
            );
            if i.data.rotation == 0.0 {
                image
            } else {
                let cx = i.data.world_point.x + i.data.width / 2.0;
                let cy = i.data.world_point.y + i.data.height / 2.0;
                let deg = i.data.rotation.to_degrees();
                format!(r#"<g transform="rotate({deg} {cx} {cy})">{image}</g>"#)
            }
        }
    }
}

/// Create a JS Promise that resolves when the image finishes loading.
pub(crate) fn image_load_promise(img: &web_sys::HtmlImageElement, url: &str) -> js_sys::Promise {
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
/// Resolves with `null` when the canvas cannot be encoded, so callers get an
/// error instead of awaiting forever.
fn canvas_to_blob_promise(canvas: &web_sys::HtmlCanvasElement) -> js_sys::Promise {
    let c = canvas.clone();
    js_sys::Promise::new(
        &mut move |resolve: js_sys::Function, _reject: js_sys::Function| {
            let resolve_err = resolve.clone();
            let cb = wasm_bindgen::closure::Closure::once(move |blob: Option<web_sys::Blob>| {
                let val = match blob {
                    Some(b) => JsValue::from(b),
                    None => JsValue::null(),
                };
                let _ = resolve.call1(&JsValue::null(), &val);
            });
            if c.to_blob(cb.as_ref().unchecked_ref()).is_err() {
                let _ = resolve_err.call1(&JsValue::null(), &JsValue::null());
            }
            cb.forget();
        },
    )
}

fn svg_blob(svg: &str) -> Option<web_sys::Blob> {
    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(svg));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("image/svg+xml");
    web_sys::Blob::new_with_str_sequence_and_options(&parts, &opts).ok()
}

/// Embed the app font in the SVG used for rasterization. An SVG loaded into
/// an `<img>` cannot use `@font-face` rules from the surrounding document, so
/// native PNG export would otherwise fall back to a different font. Browsers
/// already share their installed fonts with the image, so they are untouched.
async fn svg_with_fonts(svg: String) -> String {
    if !crate::tauri_bridge::is_tauri() {
        return svg;
    }
    let Ok(fonts) = crate::tauri_bridge::get_font_data().await else {
        return svg;
    };
    let css = font_face_css(&fonts);
    if css.is_empty() {
        return svg;
    }
    inject_styles(svg, &css)
}

fn font_face_css(fonts: &[String]) -> String {
    let family = crate::model::elements::text::FONT_FAMILY;
    let mut css = String::new();
    for (i, data) in fonts.iter().enumerate() {
        let style = if i == 0 { "normal" } else { "italic" };
        css.push_str(&format!(
            "@font-face {{ font-family: \"{family}\"; \
             src: url(\"data:font/ttf;base64,{data}\") format(\"truetype\"); \
             font-weight: 100 900; font-style: {style}; }}"
        ));
    }
    css
}

/// Insert a `<style>` block after the opening `<svg>` tag.
fn inject_styles(svg: String, css: &str) -> String {
    match svg.find('>') {
        Some(pos) => {
            let (open, rest) = svg.split_at(pos + 1);
            format!("{open}<style>{css}</style>{rest}")
        }
        None => svg,
    }
}

/// Shared SVG→canvas→blob pipeline used by both browser and Tauri PNG export.
async fn svg_to_png_blob(svg: &str) -> Option<web_sys::Blob> {
    let svg = svg_with_fonts(svg.to_string()).await;
    let document = web_sys::window()?.document()?;
    let blob = svg_blob(&svg)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).ok()?;
    let img = document
        .create_element("img")
        .ok()?
        .dyn_into::<web_sys::HtmlImageElement>()
        .ok()?;
    if wasm_bindgen_futures::JsFuture::from(image_load_promise(&img, &url))
        .await
        .is_err()
    {
        web_sys::Url::revoke_object_url(&url).ok();
        return None;
    }
    let canvas = document
        .create_element("canvas")
        .ok()?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .ok()?;
    let (w, h) = (img.natural_width() as u32, img.natural_height() as u32);
    canvas.set_width(w.max(1));
    canvas.set_height(h.max(1));
    let ctx = canvas
        .get_context("2d")
        .ok()
        .flatten()?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .ok()?;
    let _ = ctx.draw_image_with_html_image_element(&img, 0.0, 0.0);
    let png = wasm_bindgen_futures::JsFuture::from(canvas_to_blob_promise(&canvas))
        .await
        .ok()
        .and_then(|v| v.dyn_into::<web_sys::Blob>().ok());
    web_sys::Url::revoke_object_url(&url).ok();
    png
}

pub fn download_png_from_svg(svg: String, filename: String, notifications: Notifications) {
    spawn_local(async move {
        let Some(blob) = svg_to_png_blob(&svg).await else {
            notify::error(notifications, "Could not render the PNG export.");
            return;
        };
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
            notify::error(notifications, "Could not render the PNG export.");
            return;
        };
        let Ok(anchor) = document.create_element("a") else {
            return;
        };
        anchor.set_attribute("href", &url).ok();
        anchor.set_attribute("download", &filename).ok();
        anchor.set_attribute("style", "display:none").ok();
        if let Some(body) = document.body() {
            body.append_child(&anchor).ok();
            let _ = anchor
                .dyn_ref::<web_sys::HtmlElement>()
                .map(|element| element.click());
            body.remove_child(&anchor).ok();
        }
        // Revoke after a beat: some browsers read the URL lazily.
        set_timeout(
            move || {
                web_sys::Url::revoke_object_url(&url).ok();
            },
            Duration::from_secs(1),
        );
    });
}

/// Tauri-native SVG export: shows a save dialog, writes the SVG string to disk.
pub fn tauri_export_svg(svg: String, selection: bool, notifications: Notifications) {
    spawn_local(async move {
        let dir = crate::tauri_bridge::get_documents_dir().await.ok();
        let name = if selection {
            "selection.svg"
        } else {
            "canvas.svg"
        };
        let Some(path) = crate::tauri_bridge::pick_save_path_svg(name, dir.as_deref()).await else {
            return;
        };
        let path = with_extension(path, "svg");
        match crate::tauri_bridge::save_file(&path, &svg).await {
            Ok(()) => notify::info(notifications, format!("Exported {}", file_name(&path))),
            Err(e) => notify::error(notifications, format!("Could not export the image: {e}")),
        }
    });
}

/// Tauri-native PNG export: renders SVG to canvas, converts to blob, saves as PNG.
pub fn tauri_export_png(svg: String, selection: bool, notifications: Notifications) {
    spawn_local(async move {
        let Some(blob) = svg_to_png_blob(&svg).await else {
            notify::error(notifications, "Could not render the PNG export.");
            return;
        };
        let name = if selection {
            "selection.png"
        } else {
            "canvas.png"
        };
        let dir = crate::tauri_bridge::get_documents_dir().await.ok();
        let Some(path) = crate::tauri_bridge::pick_save_path_png(name, dir.as_deref()).await else {
            return;
        };
        let path = with_extension(path, "png");
        let ab_promise = js_sys::Promise::new(&mut {
            let blob2 = blob.clone();
            move |resolve: js_sys::Function, reject: js_sys::Function| {
                let reader = web_sys::FileReader::new().expect("FileReader");
                let reader_c = reader.clone();
                let reject_err = reject.clone();
                let reject_now = reject.clone();
                let cb = wasm_bindgen::closure::Closure::once(move || match reader_c.result() {
                    Ok(val) if !val.is_null() && !val.is_undefined() => {
                        let _ = resolve.call1(&JsValue::null(), &val);
                    }
                    _ => {
                        let _ = reject.call0(&JsValue::null());
                    }
                });
                reader.set_onload(Some(cb.as_ref().unchecked_ref()));
                cb.forget();
                let err_cb = wasm_bindgen::closure::Closure::once(move || {
                    let _ = reject_err.call0(&JsValue::null());
                });
                reader.set_onerror(Some(err_cb.as_ref().unchecked_ref()));
                err_cb.forget();
                if reader.read_as_array_buffer(&blob2).is_err() {
                    let _ = reject_now.call0(&JsValue::null());
                }
            }
        });
        if let Ok(val) = wasm_bindgen_futures::JsFuture::from(ab_promise).await {
            if val.is_null() || val.is_undefined() {
                notify::error(notifications, "Could not render the PNG export.");
                return;
            }
            let uint8 = js_sys::Uint8Array::new(&val);
            let bytes = uint8.to_vec();
            match crate::tauri_bridge::save_file_binary(&path, &bytes).await {
                Ok(()) => notify::info(notifications, format!("Exported {}", file_name(&path))),
                Err(e) => notify::error(notifications, format!("Could not export the image: {e}")),
            }
        } else {
            notify::error(notifications, "Could not render the PNG export.");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::elements::rect::Rectangle;
    use crate::model::elements::{EdgeStyle, StrokeStyle};
    use crate::model::ElementData;

    #[test]
    fn full_export_uses_the_screen_pixel_size() {
        let scene = Scene::new();
        let svg = scene_to_svg(
            &scene,
            &Viewport::default(),
            Point::new(800.0, 600.0),
            None,
            CanvasBg::Dark,
        );
        assert!(svg.contains(r#"width="800""#));
        assert!(svg.contains(r#"height="600""#));
        assert!(svg.contains("viewBox="));
        assert!(svg.contains(r##"fill="#1a1b26""##));
    }

    #[test]
    fn selection_export_is_sized_to_the_selection_bounds() {
        let mut scene = Scene::new();
        let mut rect = Rectangle {
            data: ElementData::new(0),
        };
        rect.data.world_point = Point::new(10.0, 20.0);
        rect.data.width = 100.0;
        rect.data.height = 50.0;
        scene.add_element(Element::Rectangle(rect));

        let svg = scene_to_svg(
            &scene,
            &Viewport::default(),
            Point::new(800.0, 600.0),
            Some(&[1]),
            CanvasBg::Light,
        );
        assert!(svg.contains(r#"width="120""#));
        assert!(svg.contains(r#"height="70""#));
    }

    #[test]
    fn rectangle_export_keeps_corners_and_dashes() {
        let mut scene = Scene::new();
        let mut rect = Rectangle {
            data: ElementData::new(0),
        };
        rect.data.style.edge_style = EdgeStyle::Rounded;
        rect.data.style.roundness = 12.0;
        rect.data.style.stroke_style = StrokeStyle::Dashed;
        scene.add_element(Element::Rectangle(rect));

        let svg = scene_to_svg_crop(&scene, Rect::new(0.0, 0.0, 200.0, 200.0), CanvasBg::Dark);
        assert!(svg.contains(r#"rx="12""#));
        assert!(svg.contains(r#"stroke-dasharray="6 3""#));
    }

    #[test]
    fn image_export_embeds_the_data_url_and_rotation() {
        use crate::model::elements::image::Image;

        let mut scene = Scene::new();
        let image = Image::new(
            "data:image/png;base64,AAAA".to_string(),
            100.0,
            50.0,
            Point::new(50.0, 25.0),
        );
        scene.add_element(Element::Image(image));

        let svg = scene_to_svg_crop(&scene, Rect::new(0.0, 0.0, 200.0, 200.0), CanvasBg::Dark);
        assert!(svg.contains(r#"<image x="0" y="0" width="100" height="50""#));
        assert!(svg.contains(r#"href="data:image/png;base64,AAAA""#));
        assert!(svg.contains(r#"preserveAspectRatio="none""#));
    }

    #[test]
    fn text_content_is_escaped() {
        assert_eq!(escape_xml_text("a & <b>"), "a &amp; &lt;b&gt;");
    }

    #[test]
    fn font_data_is_embedded_in_the_svg_root() {
        let css = font_face_css(&["AAAA".to_string(), "BBBB".to_string()]);
        let svg = inject_styles(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><rect/></svg>"#.to_string(),
            &css,
        );
        assert!(svg.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg"><style>@font-face"#));
        assert!(svg.contains("data:font/ttf;base64,AAAA"));
        assert!(svg.contains("data:font/ttf;base64,BBBB"));
        assert!(svg.contains("font-style: normal"));
        assert!(svg.contains("font-style: italic"));
        assert!(svg.contains(r#"<rect/></svg>"#));
    }
}
