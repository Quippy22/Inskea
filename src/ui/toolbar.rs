use crate::canvas::{CanvasMode, Viewport};
use crate::model::{ElementId, Scene};
use crate::skea;
use crate::tauri_bridge;
use crate::ui::classes;
use crate::ui::components::{Dropdown, DropdownItem};
use crate::ui::icon;
use leptos::*;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

fn is_tauri() -> bool {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return false,
    };
    js_sys::Reflect::get(&window, &JsValue::from_str("__TAURI__"))
        .map(|v| !v.is_undefined() && !v.is_null())
        .unwrap_or(false)
}

fn window_size() -> (f64, f64) {
    let window = web_sys::window().expect("no global `window` exists");
    let w = window.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
    let h = window.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
    (w, h)
}

// ── Browser download helpers ──────────────────────────────────────────

fn download_string(content: &str, filename: &str) {
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

/// Crude SVG export: walks elements and builds a minimal SVG string.
fn scene_to_svg(scene: &Scene, viewport: &Viewport, screen: (f64, f64)) -> String {
    let vb = viewport.to_view_box(screen.0, screen.1);
    let mut out = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vb}">"#);
    for el in &scene.elements {
        out.push_str(&el_svg(el));
    }
    out.push_str("</svg>");
    out
}

fn el_svg(el: &crate::model::Element) -> String {
    use std::fmt::Write;
    match el {
        crate::model::Element::Rectangle(r) => {
            let fill = r.data.fill_color.map(|c| c.to_hex()).unwrap_or("none");
            format!(
                r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
                r.data.x, r.data.y, r.data.width, r.data.height, fill, r.data.stroke_color.to_hex(), r.data.stroke_width,
            )
        }
        crate::model::Element::Ellipse(e) => {
            let fill = e.data.fill_color.map(|c| c.to_hex()).unwrap_or("none");
            let cx = e.data.x + e.data.width / 2.0;
            let cy = e.data.y + e.data.height / 2.0;
            format!(
                r#"<ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
                cx, cy, e.data.width / 2.0, e.data.height / 2.0, fill, e.data.stroke_color.to_hex(), e.data.stroke_width,
            )
        }
        crate::model::Element::Line(l) => {
            format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}"/>"#,
                l.a.x, l.a.y, l.b.x, l.b.y, l.data.stroke_color.to_hex(), l.data.stroke_width,
            )
        }
        crate::model::Element::Arrow(a) => {
            let mut out = format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}"/>"#,
                a.a.x, a.a.y, a.b.x, a.b.y, a.data.stroke_color.to_hex(), a.data.stroke_width,
            );
            let dx = a.b.x - a.a.x;
            let dy = a.b.y - a.a.y;
            let len = dx.hypot(dy);
            if len > 1.0 {
                let ux = dx / len;
                let uy = dy / len;
                let hl = (a.data.stroke_width * 4.0).max(8.0);
                let hw = hl * 0.4;
                let bx = a.b.x - ux * hl;
                let by = a.b.y - uy * hl;
                let _ = write!(out, r#"<polyline points="{},{} {},{} {},{}" fill="none" stroke="{}" stroke-width="{}"/>"#,
                    a.b.x, a.b.y,
                    bx - uy * hw, by + ux * hw,
                    bx + uy * hw, by - ux * hw,
                    a.data.stroke_color.to_hex(), a.data.stroke_width,
                );
            }
            out
        }
        crate::model::Element::Text(t) => {
            let mut out = format!(
                r#"<text x="{}" y="{}" font-size="{}" fill="{}">"#,
                t.data.x, t.data.y + t.data.font_size, t.data.font_size, t.data.stroke_color.to_hex(),
            );
            for (i, line) in t.wrapped.display.split('\n').enumerate() {
                let esc = line.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
                let dy = if i == 0 { "0" } else { &format!("{}", t.data.font_size) };
                let _ = write!(out, r#"<tspan x="{}" dy="{}">{}</tspan>"#, t.data.x, dy, esc);
            }
            out.push_str("</text>");
            out
        }
        crate::model::Element::Freehand(f) => {
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
                d, f.data.stroke_color.to_hex(), f.data.stroke_width,
            )
        }
    }
}

fn download_png_from_svg(svg: String, filename: String) {
    spawn_local(async move {
        let document = web_sys::window().unwrap().document().unwrap();

        let parts = js_sys::Array::new();
        parts.push(&JsValue::from_str(&svg));
        let blob = web_sys::Blob::new_with_str_sequence(&parts).expect("blob");
        let url = web_sys::Url::create_object_url_with_blob(&blob).expect("url");

        let img = document
            .create_element("img")
            .unwrap()
            .dyn_into::<web_sys::HtmlImageElement>()
            .unwrap();

        let (tx, rx) = std::sync::mpsc::channel();
        let cb = wasm_bindgen::closure::Closure::once(move || { let _ = tx.send(()); });
        img.set_onload(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
        img.set_src(&url);

        let _ = rx.recv();

        let canvas = document
            .create_element("canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();
        let w = img.width().max(1);
        let h = img.height().max(1);
        canvas.set_width(w);
        canvas.set_height(h);

        let ctx = canvas
            .get_context("2d")
            .ok()
            .flatten()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();
        let _ = ctx.draw_image_with_html_image_element(&img, 0.0, 0.0);

        let fname = filename.clone();
        let cb2 = wasm_bindgen::closure::Closure::once(move |blob: Option<web_sys::Blob>| {
            if let Some(blob) = blob {
                let url2 = web_sys::Url::create_object_url_with_blob(&blob).expect("url2");
                let a = document.create_element("a").unwrap();
                a.set_attribute("href", &url2).ok();
                a.set_attribute("download", &fname).ok();
                a.set_attribute("style", "display:none").ok();
                document.body().unwrap().append_child(&a).ok();
                let _ = a.dyn_ref::<web_sys::HtmlElement>().map(|e| e.click());
                document.body().unwrap().remove_child(&a).ok();
                web_sys::Url::revoke_object_url(&url2).ok();
            }
        });
        let _ = canvas.to_blob(cb2.as_ref().unchecked_ref());
        cb2.forget();
        web_sys::Url::revoke_object_url(&url).ok();
    });
}

fn browser_export_skea(scene: Scene) {
    let content = skea::save_to_string(&scene);
    download_string(&content, "untitled.skea");
}

fn browser_import(scene: RwSignal<Scene>) {
    let document = web_sys::window().unwrap().document().unwrap();
    let input = document
        .create_element("input")
        .expect("failed to create input");
    input.set_attribute("type", "file").ok();
    input.set_attribute("accept", ".skea").ok();
    input.set_attribute("style", "display:none").ok();
    document.body().unwrap().append_child(&input).ok();

    let input_el = input
        .dyn_ref::<web_sys::HtmlInputElement>()
        .expect("input is not HtmlInputElement")
        .clone();
    let input_el2 = input_el.clone();
    let doc = document.clone();

    let on_change = Closure::wrap(Box::new(move || {
        let input_el = input_el.clone();
        let doc = doc.clone();
        if let Some(file) = input_el.files().and_then(|f| f.item(0)) {
            let reader = web_sys::FileReader::new().expect("failed to create FileReader");
            let reader_c = reader.clone();
            let scene_c = scene;
            let on_load = Closure::wrap(Box::new(move || {
                if let Ok(val) = reader_c.result() {
                    if let Some(text) = val.as_string() {
                        match skea::load_from_str(&text) {
                            Ok(loaded) => scene_c.set(loaded),
                            Err(e) => {
                                web_sys::console::error_1(&format!("parse error: {e}").into());
                            }
                        }
                    }
                }
            }) as Box<dyn FnMut()>);
            reader.set_onload(Some(on_load.as_ref().unchecked_ref()));
            on_load.forget();
            let _ = reader.read_as_text(&file);
        }
        doc.body().unwrap().remove_child(&input_el).ok();
    }) as Box<dyn FnMut()>);

    input_el2.set_onchange(Some(on_change.as_ref().unchecked_ref()));
    on_change.forget();
    input_el2.click();
}

// ── ToolBar component ─────────────────────────────────────────────────

#[component]
pub fn ToolBar<F1, F2>(
    scene: RwSignal<Scene>,
    viewport: RwSignal<Viewport>,
    canvas_mode: RwSignal<CanvasMode>,
    on_undo: F1,
    on_redo: F2,
    can_undo: Signal<bool>,
    can_redo: Signal<bool>,
    _selected_ids: RwSignal<Vec<ElementId>>,
) -> impl IntoView
where
    F1: Fn() + 'static,
    F2: Fn() + 'static,
{
    let menu_open = create_rw_signal(false);
    let saved_path = create_rw_signal::<Option<String>>(None);
    let tauri = is_tauri();

    // Submenu hover tracking — one pair per fly-out panel
    let file_hover = create_rw_signal(false);
    let file_sub_hover = create_rw_signal(false);
    let show_file = Signal::derive(move || file_hover.get() || file_sub_hover.get());

    let export_hover = create_rw_signal(false);
    let export_sub_hover = create_rw_signal(false);
    let show_export = Signal::derive(move || export_hover.get() || export_sub_hover.get());

    let on_home = move |_| viewport.set(Viewport::default());

    let close_menu = move || {
        menu_open.set(false);
        file_hover.set(false);
        file_sub_hover.set(false);
        export_hover.set(false);
        export_sub_hover.set(false);
    };

    // ── File actions ───────────────────────────────────────────────────

    let on_new = {
        let close_menu = close_menu.clone();
        move || { close_menu(); saved_path.set(None); scene.set(Scene::new()); }
    };
    let on_save_as = {
        let close_menu = close_menu.clone();
        move || {
            close_menu();
            spawn_local(async move {
                let dir = tauri_bridge::get_app_data_dir().await.ok();
                let path = tauri_bridge::pick_save_path("untitled.skea", dir.as_deref()).await;
                if let Some(path) = path {
                    saved_path.set(Some(path.clone()));
                    let s = scene.get();
                    let c = skea::save_to_string(&s);
                    let _ = tauri_bridge::save_skea(&path, &c).await;
                }
            });
        }
    };
    let on_save = {
        let close_menu = close_menu.clone();
        let on_save_as = on_save_as.clone();
        move || {
            close_menu();
            let saved = saved_path.get();
            if let Some(path) = saved {
                let s = scene.get();
                spawn_local(async move {
                    let c = skea::save_to_string(&s);
                    let _ = tauri_bridge::save_skea(&path, &c).await;
                });
            } else {
                on_save_as();
            }
        }
    };
    let on_open = {
        let close_menu = close_menu.clone();
        move || {
            close_menu();
            spawn_local(async move {
                let dir = tauri_bridge::get_app_data_dir().await.ok();
                let path = tauri_bridge::pick_open_path(dir.as_deref()).await;
                if let Some(path) = path {
                    saved_path.set(Some(path.clone()));
                    match tauri_bridge::load_skea(&path).await {
                        Ok(c) => match skea::load_from_str(&c) {
                            Ok(loaded) => scene.set(loaded),
                            Err(e) => web_sys::console::error_1(&format!("parse: {e}").into()),
                        },
                        Err(e) => web_sys::console::error_1(&format!("load: {e}").into()),
                    }
                }
            });
        }
    };
    let on_import = {
        let close_menu = close_menu.clone();
        move || {
            close_menu();
            if tauri {
                spawn_local(async move {
                    let path = tauri_bridge::pick_open_path(None).await;
                    if let Some(path) = path {
                        saved_path.set(Some(path.clone()));
                        match tauri_bridge::load_skea(&path).await {
                            Ok(c) => match skea::load_from_str(&c) {
                                Ok(loaded) => scene.set(loaded),
                                Err(e) => web_sys::console::error_1(&format!("parse: {e}").into()),
                            },
                            Err(e) => web_sys::console::error_1(&format!("load: {e}").into()),
                        }
                    }
                });
            } else {
                browser_import(scene);
            }
        }
    };

    // ── Export actions ─────────────────────────────────────────────────

    let on_export_skea = {
        let close_menu = close_menu.clone();
        move || {
            close_menu();
            let s = scene.get();
            if tauri {
                spawn_local(async move {
                    let dir = tauri_bridge::get_app_data_dir().await.ok();
                    let path = tauri_bridge::pick_save_path("untitled.skea", dir.as_deref()).await;
                    if let Some(path) = path {
                        let c = skea::save_to_string(&s);
                        let _ = tauri_bridge::save_skea(&path, &c).await;
                    }
                });
            } else {
                browser_export_skea(s);
            }
        }
    };

    let on_export_svg = {
        let close_menu = close_menu.clone();
        move |selection: bool| {
            close_menu();
            let s = scene.get();
            let vp = viewport.get();
            let size = window_size();
            let svg = scene_to_svg(&s, &vp, size);
            download_string(&svg, if selection { "selection.svg" } else { "canvas.svg" });
        }
    };

    let on_export_png = {
        let close_menu = close_menu.clone();
        move |selection: bool| {
            close_menu();
            let s = scene.get();
            let vp = viewport.get();
            let size = window_size();
            let svg = scene_to_svg(&s, &vp, size);
            download_png_from_svg(svg, if selection { "selection.png" } else { "canvas.png" }.to_string());
        }
    };

    // ── Dropdown item lists ────────────────────────────────────────────

    let file_items: Vec<DropdownItem> = vec![
        DropdownItem::Action { label: "New", on_click: Rc::new(on_new) },
        DropdownItem::Action { label: "Save", on_click: Rc::new(on_save) },
        DropdownItem::Action { label: "Save As", on_click: Rc::new(on_save_as) },
        DropdownItem::Action { label: "Open", on_click: Rc::new(on_open) },
        DropdownItem::Separator,
        DropdownItem::Action { label: "Import", on_click: Rc::new(on_import) },
    ];

    let export_items: Vec<DropdownItem> = vec![
        DropdownItem::Action { label: "Export .skea", on_click: Rc::new(on_export_skea) },
        DropdownItem::Separator,
        DropdownItem::Header { label: "PNG" },
        DropdownItem::Action { label: "Full canvas", on_click: Rc::new({ let f = on_export_png.clone(); move || f(false) }) },
        DropdownItem::Action { label: "Selection", on_click: Rc::new({ let f = on_export_png.clone(); move || f(true) }) },
        DropdownItem::Separator,
        DropdownItem::Header { label: "SVG" },
        DropdownItem::Action { label: "Full canvas", on_click: Rc::new({ let f = on_export_svg.clone(); move || f(false) }) },
        DropdownItem::Action { label: "Selection", on_click: Rc::new({ let f = on_export_svg.clone(); move || f(true) }) },
    ];

    // ── Mode button helper ─────────────────────────────────────────────

    let btn = move |mode: CanvasMode| -> &'static str {
        if canvas_mode.get() == mode {
            classes::BTN_TBAR_ACTIVE
        } else {
            classes::BTN_TBAR_INACTIVE
        }
    };

    view! {
        <div class=classes::CONTAINER_STATUSBAR>
            <div class=classes::TBAR_INNER>
                <button
                    class=move || btn(CanvasMode::Hand)
                    on:click=move |_| canvas_mode.set(CanvasMode::Hand)
                    title="Hand / Pan"
                >
                    {icon::hand()}
                </button>
                <button
                    class=move || btn(CanvasMode::Select)
                    on:click=move |_| canvas_mode.set(CanvasMode::Select)
                    title="Select"
                >
                    {icon::cursor()}
                </button>
                <button
                    class=move || btn(CanvasMode::Draw)
                    on:click=move |_| canvas_mode.set(CanvasMode::Draw)
                    title="Draw"
                >
                    {icon::pencil()}
                </button>
                <div class=classes::SEP_V />
                <button class=classes::BTN_GHOST on:click=on_home title="Home">
                    {icon::home()}
                </button>
                <button
                    class=classes::BTN_GHOST
                    class:opacity-40=move || !can_undo.get()
                    class:cursor-not-allowed=move || !can_undo.get()
                    on:click=move |_| on_undo()
                    title="Undo"
                >
                    {icon::undo()}
                </button>
                <button
                    class=classes::BTN_GHOST
                    class:opacity-40=move || !can_redo.get()
                    class:cursor-not-allowed=move || !can_redo.get()
                    on:click=move |_| on_redo()
                    title="Redo"
                >
                    {icon::redo()}
                </button>
                <div class=classes::SEP_V />
                <div class="relative">
                    <button
                        class=classes::BTN_GHOST
                        on:click=move |_| menu_open.update(|v| *v = !*v)
                        title="Menu"
                    >
                        {icon::menu()}
                    </button>
                    {move || {
                        if menu_open.get() {
                            view! {
                                <>
                                    <div
                                        class="fixed inset-0 z-40"
                                        on:click=move |_| close_menu()
                                    ></div>
                                    <div class=classes::MENU_DROPDOWN>
                                        // ── File ─────────────────────────────
                                        <div
                                            class="relative"
                                            on:mouseenter=move |_| file_hover.set(true)
                                            on:mouseleave=move |_| file_hover.set(false)
                                        >
                                            <button class=classes::MENU_ITEM>
                                                <span>"File"</span>
                                                {icon::chevron_right()}
                                            </button>
                                            <Dropdown
                                                show=show_file
                                                on_mouseenter=Rc::new({ let s = file_sub_hover; move || s.set(true) })
                                                on_mouseleave=Rc::new({ let s = file_sub_hover; move || s.set(false) })
                                                items=file_items.clone()
                                            />
                                        </div>

                                        // ── Export ───────────────────────────
                                        <div
                                            class="relative"
                                            on:mouseenter=move |_| export_hover.set(true)
                                            on:mouseleave=move |_| export_hover.set(false)
                                        >
                                            <button class=classes::MENU_ITEM>
                                                <span>"Export"</span>
                                                {icon::chevron_right()}
                                            </button>
                                            <Dropdown
                                                show=show_export
                                                on_mouseenter=Rc::new({ let s = export_sub_hover; move || s.set(true) })
                                                on_mouseleave=Rc::new({ let s = export_sub_hover; move || s.set(false) })
                                                items=export_items.clone()
                                            />
                                        </div>
                                    </div>
                                </>
                            }
                                .into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
