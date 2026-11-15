use crate::canvas::{CanvasMode, Viewport};
use crate::document::DocContext;
use crate::edit_ops;
use crate::model::{Chart, ChartKind, ChartStyle, Element, ElementId, ElementStyle, Point};
use crate::tauri_bridge;
use crate::ui::dock::Tool;
use crate::ui::notify;
use crate::util::window_size;
use leptos::*;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

/// Accepted file extensions/MIME types for tabular chart import in the browser.
const BROWSER_ACCEPT_CSV: &str = ".csv,.tsv,.txt,text/csv,text/tab-separated-values,text/plain";

/// World-space offset when multiple CSV files are imported in one batch.
const CASCADE_STEP: f64 = 28.0;

/// Handles creating charts from the toolbar menu, drawing tools, or CSV/TSV/Markdown imports.
#[derive(Clone)]
pub struct ChartImport {
    pub doc: DocContext,
    pub viewport: RwSignal<Viewport>,
    pub canvas_mode: RwSignal<CanvasMode>,
    pub selected_tool: RwSignal<Tool>,
    pub default_style: RwSignal<ElementStyle>,
    pub default_chart_style: RwSignal<ChartStyle>,
    pub chart_editing_id: RwSignal<Option<ElementId>>,
    pub push_snapshot: Rc<dyn Fn()>,
}

impl ChartImport {
    /// Insert a new chart of the requested `ChartKind` at the centre of the viewport.
    pub fn insert_chart(&self, kind: ChartKind) {
        self.default_chart_style.update(|cs| cs.kind = kind);
        let centre = self.view_centre();
        let style = self.default_style.get_untracked();
        let mut chart_style = self.default_chart_style.get_untracked();
        chart_style.kind = kind;

        let mut chart = Chart::new_at(centre, kind, style.stroke_color);
        chart.data.style = style;
        chart.apply_chart_style(&chart_style);

        let mut new_id = 0;
        edit_ops::mutate_scene(self.doc.scene, &self.push_snapshot, |scene| {
            scene.add_element(Element::Chart(chart));
            new_id = scene.next_id - 1;
        });
        self.doc.selected_ids.set(vec![new_id]);
        self.canvas_mode.set(CanvasMode::Select);
        self.doc.show_welcome.set(false);
    }

    /// Switch the active tool to `Tool::Chart` in Draw mode so the user can drag out a chart.
    pub fn start_draw_chart(&self, kind: ChartKind) {
        self.default_chart_style.update(|cs| cs.kind = kind);
        self.selected_tool.set(Tool::Chart);
        self.canvas_mode.set(CanvasMode::Draw);
        self.doc.show_welcome.set(false);
    }

    /// Open the file picker to import a `.csv` / `.tsv` file as a chart (or into the selected chart).
    pub fn pick_csv(&self) {
        if tauri_bridge::is_tauri() {
            let this = self.clone();
            spawn_local(async move {
                let dir = tauri_bridge::get_documents_dir().await.ok();
                let Some(path) = tauri_bridge::pick_csv_path(dir.as_deref()).await else {
                    return;
                };
                this.import_paths(vec![path], None);
            });
        } else {
            self.browser_pick_csv();
        }
    }

    /// Import CSV/TSV files by native path.
    pub fn import_paths(&self, paths: Vec<String>, at: Option<Point>) {
        let this = self.clone();
        spawn_local(async move {
            let mut entries = Vec::new();
            for path in paths {
                let title_hint = file_stem_title(&path);
                match tauri_bridge::load_skea(&path).await {
                    Ok(content) => entries.push((content, title_hint)),
                    Err(e) => notify::error(
                        this.doc.notifications,
                        format!("Could not read tabular data file: {e}"),
                    ),
                }
            }
            this.place_tabular_batch(entries, at);
        });
    }

    /// Import browser `File` objects containing CSV/TSV data.
    pub fn import_files(&self, files: Vec<web_sys::File>, at: Option<Point>) {
        if files.is_empty() {
            return;
        }
        let this = self.clone();
        spawn_local(async move {
            let mut entries = Vec::new();
            for file in files {
                let name = file.name();
                let title_hint = file_stem_title(&name);
                match read_file_text(&file).await {
                    Ok(text) => entries.push((text, title_hint)),
                    Err(_) => notify::error(
                        this.doc.notifications,
                        format!("Could not read {name}."),
                    ),
                }
            }
            this.place_tabular_batch(entries, at);
        });
    }

    /// Parse raw tabular text (CSV, TSV, or Markdown table) and either update the single
    /// selected chart or insert a new chart on the canvas.
    pub fn import_tabular_text(
        &self,
        text: &str,
        fallback_title: Option<String>,
        at: Option<Point>,
    ) -> bool {
        let Some((parsed_title, items)) = Chart::parse_tabular_data(text) else {
            return false;
        };
        let title_to_use = parsed_title.or(fallback_title);

        // If a single chart is currently selected and `at` wasn't a specific drop location,
        // update that chart's data directly.
        let selected = self.doc.selected_ids.get_untracked();
        if at.is_none() && selected.len() == 1 {
            let sel_id = selected[0];
            let is_chart = self
                .doc
                .scene
                .with_untracked(|s| matches!(s.element_by_id(sel_id), Some(Element::Chart(_))));
            if is_chart {
                edit_ops::mutate_scene(self.doc.scene, &self.push_snapshot, |scene| {
                    if let Some(mut element) = scene.element_by_id_mut(sel_id) {
                        if let Element::Chart(chart) = &mut *element {
                            if let Some(t) = title_to_use {
                                chart.title = t;
                            }
                            chart.items = items;
                        }
                    }
                    scene.refresh_connectors_for(sel_id);
                });
                notify::info(
                    self.doc.notifications,
                    "Updated chart data from tabular input.".to_string(),
                );
                return true;
            }
        }

        let centre = at.unwrap_or_else(|| self.view_centre());
        let style = self.default_style.get_untracked();
        let chart_style = self.default_chart_style.get_untracked();
        let mut chart = Chart::new_at(centre, chart_style.kind, style.stroke_color);
        chart.data.style = style;
        chart.apply_chart_style(&chart_style);
        if let Some(t) = title_to_use {
            chart.title = t;
        }
        chart.items = items;

        let mut new_id = 0;
        edit_ops::mutate_scene(self.doc.scene, &self.push_snapshot, |scene| {
            scene.add_element(Element::Chart(chart));
            new_id = scene.next_id - 1;
        });
        self.doc.selected_ids.set(vec![new_id]);
        self.canvas_mode.set(CanvasMode::Select);
        self.doc.show_welcome.set(false);
        notify::info(
            self.doc.notifications,
            "Created chart from tabular data.".to_string(),
        );
        true
    }

    fn place_tabular_batch(&self, entries: Vec<(String, Option<String>)>, at: Option<Point>) {
        if entries.is_empty() {
            return;
        }
        if entries.len() == 1 {
            let (text, title_hint) = &entries[0];
            if !self.import_tabular_text(text, title_hint.clone(), at) {
                notify::error(
                    self.doc.notifications,
                    "Could not find valid label/value rows in that file.".to_string(),
                );
            }
            return;
        }

        let anchor = at.unwrap_or_else(|| self.view_centre());
        let style = self.default_style.get_untracked();
        let chart_style = self.default_chart_style.get_untracked();
        let mut charts = Vec::new();

        for (index, (text, title_hint)) in entries.into_iter().enumerate() {
            let Some((parsed_title, items)) = Chart::parse_tabular_data(&text) else {
                continue;
            };
            let cascade = index as f64 * CASCADE_STEP;
            let centre = Point::new(anchor.x + cascade, anchor.y + cascade);
            let mut chart = Chart::new_at(centre, chart_style.kind, style.stroke_color);
            chart.data.style = style.clone();
            chart.apply_chart_style(&chart_style);
            if let Some(t) = parsed_title.or(title_hint) {
                chart.title = t;
            }
            chart.items = items;
            charts.push(chart);
        }

        if charts.is_empty() {
            notify::error(
                self.doc.notifications,
                "Could not find valid label/value rows in the selected files.".to_string(),
            );
            return;
        }

        let count = charts.len();
        let mut ids = Vec::new();
        edit_ops::mutate_scene(self.doc.scene, &self.push_snapshot, |scene| {
            for chart in charts {
                scene.add_element(Element::Chart(chart));
                ids.push(scene.next_id - 1);
            }
        });
        self.doc.selected_ids.set(ids);
        self.canvas_mode.set(CanvasMode::Select);
        self.doc.show_welcome.set(false);
        notify::info(
            self.doc.notifications,
            format!("Imported {count} charts."),
        );
    }

    fn view_centre(&self) -> Point {
        let visible = self.viewport.get_untracked().visible_rect(window_size());
        Point::new(visible.x + visible.w / 2.0, visible.y + visible.h / 2.0)
    }

    fn browser_pick_csv(&self) {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let Ok(input) = document.create_element("input") else {
            return;
        };
        input.set_attribute("type", "file").ok();
        input.set_attribute("accept", BROWSER_ACCEPT_CSV).ok();
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
            let files: Vec<web_sys::File> = input_el
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

/// Check whether a file path has a tabular data extension (`.csv` or `.tsv`).
pub fn is_tabular_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".csv") || lower.ends_with(".tsv")
}

/// Check whether a browser `File` is a CSV/TSV file by name or MIME type.
pub fn is_tabular_file(file: &web_sys::File) -> bool {
    let mime = file.type_().to_ascii_lowercase();
    if mime == "text/csv" || mime == "text/tab-separated-values" || mime == "application/csv" {
        return true;
    }
    is_tabular_path(&file.name())
}

fn file_stem_title(path_or_name: &str) -> Option<String> {
    let file_name = path_or_name.rsplit(['/', '\\']).next().unwrap_or(path_or_name);
    let stem = match file_name.rfind('.') {
        Some(idx) if idx > 0 => &file_name[..idx],
        _ => file_name,
    };
    let cleaned = stem.replace(['_', '-'], " ").trim().to_string();
    (!cleaned.is_empty()).then_some(cleaned)
}

async fn read_file_text(file: &web_sys::File) -> Result<String, ()> {
    let reader = web_sys::FileReader::new().map_err(|_| ())?;
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let reader_ok = reader.clone();
        let resolve = resolve.clone();
        let reject_onload = reject.clone();
        let reject_onerror = reject.clone();
        let onload = Closure::once(move || {
            if let Ok(res) = reader_ok.result() {
                let _ = resolve.call1(&JsValue::NULL, &res);
            } else {
                let _ = reject_onload.call0(&JsValue::NULL);
            }
        });
        let onerror = Closure::once(move || {
            let _ = reject_onerror.call0(&JsValue::NULL);
        });
        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
        reader.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onload.forget();
        onerror.forget();
        if reader.read_as_text(file).is_err() {
            let _ = reject.call0(&JsValue::NULL);
        }
    });
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|_| ())?;
    value.as_string().ok_or(())
}

#[cfg(test)]
mod tests {
    use super::{file_stem_title, is_tabular_path};

    #[test]
    fn tabular_path_detection() {
        assert!(is_tabular_path("revenue_2026.csv"));
        assert!(is_tabular_path("/tmp/metrics.TSV"));
        assert!(!is_tabular_path("photo.png"));
        assert!(!is_tabular_path("drawing.skea"));
    }

    #[test]
    fn file_stem_to_human_title() {
        assert_eq!(
            file_stem_title("/home/user/quarterly_revenue-2026.csv"),
            Some("quarterly revenue 2026".to_string())
        );
    }
}
