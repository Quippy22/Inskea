use crate::canvas::{CanvasMode, Viewport};
use crate::document::DocContext;
use crate::edit_ops;
use crate::model::elements::image::Image;
use crate::model::{Element, ElementId, Point};
use crate::tauri_bridge;
use crate::ui::export;
use crate::ui::notify;
use crate::util::window_size;
use leptos::*;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen_futures::JsFuture;

/// Accepted by the browser's file input; native dialogs use their own filter.
const BROWSER_ACCEPT: &str = "image/png,image/jpeg,image/gif,image/webp,image/svg+xml,image/bmp";

/// World-space gap between images imported in the same batch, so a multi-file
/// drop does not stack them all on one spot.
const CASCADE_STEP: f64 = 24.0;

/// Imports image files into the scene, from the file picker or a drop.
///
/// The bytes are embedded as data URLs in the document, so a saved `.skea`
/// file stays self-contained.
#[derive(Clone)]
pub struct ImageImport {
    pub doc: DocContext,
    pub viewport: RwSignal<Viewport>,
    pub canvas_mode: RwSignal<CanvasMode>,
    pub push_snapshot: Rc<dyn Fn()>,
}

impl ImageImport {
    /// Ask the user for one image and import it at the centre of the view.
    pub fn pick(&self) {
        if tauri_bridge::is_tauri() {
            let this = self.clone();
            spawn_local(async move {
                let Some(path) = tauri_bridge::pick_image_path(None).await else {
                    return;
                };
                match tauri_bridge::read_image_data(&path).await {
                    Ok(data_url) => this.place(vec![data_url], None),
                    Err(e) => notify::error(
                        this.doc.notifications,
                        format!("Could not import the image: {e}"),
                    ),
                }
            });
        } else {
            self.browser_pick();
        }
    }

    /// Import images by native path, optionally centred on a world point.
    pub fn import_paths(&self, paths: Vec<String>, at: Option<Point>) {
        let this = self.clone();
        spawn_local(async move {
            let mut data_urls = Vec::new();
            for path in paths {
                match tauri_bridge::read_image_data(&path).await {
                    Ok(data_url) => data_urls.push(data_url),
                    Err(e) => notify::error(
                        this.doc.notifications,
                        format!("Could not import the image: {e}"),
                    ),
                }
            }
            this.place(data_urls, at);
        });
    }

    /// Import browser `File` handles, optionally centred on a world point.
    pub fn import_files(&self, files: Vec<web_sys::File>, at: Option<Point>) {
        let this = self.clone();
        spawn_local(async move {
            let mut data_urls = Vec::new();
            for file in files {
                let mime = file.type_();
                // Some platforms leave the type blank; let `measure` reject
                // anything that cannot actually be decoded.
                if !mime.is_empty() && !mime.starts_with("image/") {
                    notify::error(
                        this.doc.notifications,
                        format!("{} is not an image.", file.name()),
                    );
                    continue;
                }
                if let Some(promise) = file_as_data_url(&file) {
                    if let Ok(value) = JsFuture::from(promise).await {
                        if let Some(data_url) = value.as_string() {
                            data_urls.push(data_url);
                            continue;
                        }
                    }
                }
                notify::error(
                    this.doc.notifications,
                    format!("Could not read {}.", file.name()),
                );
            }
            this.place(data_urls, at);
        });
    }

    /// Measure each data URL, then add the images as one undoable edit.
    fn place(&self, data_urls: Vec<String>, at: Option<Point>) {
        if data_urls.is_empty() {
            return;
        }
        let this = self.clone();
        spawn_local(async move {
            let anchor = at.unwrap_or_else(|| this.view_centre());
            let mut images = Vec::new();
            for (index, data_url) in data_urls.into_iter().enumerate() {
                let Some((width, height)) = measure(&data_url).await else {
                    notify::error(
                        this.doc.notifications,
                        "Could not read that image.".to_string(),
                    );
                    continue;
                };
                let cascade = index as f64 * CASCADE_STEP;
                let centre = Point::new(anchor.x + cascade, anchor.y + cascade);
                images.push(Image::new(data_url.into(), width, height, centre));
            }
            if images.is_empty() {
                return;
            }

            let count = images.len();
            let mut ids = Vec::<ElementId>::new();
            edit_ops::mutate_scene(this.doc.scene, &this.push_snapshot, |scene| {
                for image in images {
                    scene.add_element(Element::Image(image));
                    ids.push(scene.next_id - 1);
                }
            });
            this.doc.selected_ids.set(ids);
            this.canvas_mode.set(CanvasMode::Select);
            this.doc.show_welcome.set(false);
            notify::info(
                this.doc.notifications,
                match count {
                    1 => "Imported image.".to_string(),
                    n => format!("Imported {n} images."),
                },
            );
        });
    }

    /// World point at the middle of the visible area.
    fn view_centre(&self) -> Point {
        let visible = self.viewport.get_untracked().visible_rect(window_size());
        Point::new(visible.x + visible.w / 2.0, visible.y + visible.h / 2.0)
    }

    /// Browser fallback for [`pick`]: a hidden file input.
    fn browser_pick(&self) {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let Ok(input) = document.create_element("input") else {
            return;
        };
        input.set_attribute("type", "file").ok();
        input.set_attribute("accept", BROWSER_ACCEPT).ok();
        input.set_attribute("style", "display:none").ok();
        let Some(body) = document.body() else {
            return;
        };
        body.append_child(&input).ok();

        let Ok(input_el) = input.clone().dyn_into::<web_sys::HtmlInputElement>() else {
            input.remove();
            return;
        };
        let this = self.clone();
        let input_for_change = input_el.clone();
        let document_for_cleanup = document.clone();

        let on_change = Closure::wrap(Box::new(move || {
            let input_el = input_for_change.clone();
            let files = input_el
                .files()
                .map(|list| (0..list.length()).filter_map(|i| list.item(i)).collect())
                .unwrap_or_default();
            if let Some(body) = document_for_cleanup.body() {
                body.remove_child(&input_el).ok();
            }
            this.import_files(files, None);
        }) as Box<dyn FnMut()>);

        input_el.set_onchange(Some(on_change.as_ref().unchecked_ref()));
        on_change.forget();
        input_el.click();
    }
}

/// Natural size of an image after its data URL loads.
async fn measure(data_url: &str) -> Option<(f64, f64)> {
    let image = web_sys::window()?
        .document()?
        .create_element("img")
        .ok()?
        .dyn_into::<web_sys::HtmlImageElement>()
        .ok()?;
    JsFuture::from(export::image_load_promise(&image, data_url))
        .await
        .ok()?;
    Some((image.natural_width() as f64, image.natural_height() as f64))
}

/// Read a browser file as a data URL. Resolves with `null` when reading fails.
fn file_as_data_url(file: &web_sys::File) -> Option<js_sys::Promise> {
    let reader = web_sys::FileReader::new().ok()?;
    let reader_for_promise = reader.clone();
    let promise = js_sys::Promise::new(&mut move |resolve, reject| {
        let reader_on_load = reader_for_promise.clone();
        let resolve_load = resolve.clone();
        let on_load = Closure::once(move || {
            let result = reader_on_load.result().unwrap_or(JsValue::NULL);
            let _ = resolve_load.call1(&JsValue::NULL, &result);
        });
        reader_for_promise.set_onload(Some(on_load.as_ref().unchecked_ref()));
        on_load.forget();

        let on_error = Closure::once(move || {
            let _ = reject.call1(&JsValue::NULL, &JsValue::NULL);
        });
        reader_for_promise.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        on_error.forget();
    });
    reader.read_as_data_url(file).ok()?;
    Some(promise)
}
