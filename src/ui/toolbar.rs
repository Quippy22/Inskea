#![allow(clippy::redundant_locals)]
use crate::canvas::settings::CanvasSettings;
use crate::canvas::{CanvasMode, CropExportCallback, Viewport};
use crate::document::{DocContext, PendingAction, RequestAction};
use crate::model::{ChartKind, Rect};
use crate::skea;
use crate::tauri_bridge;
use crate::ui::chart_import::ChartImport;
use crate::ui::components::{Dropdown, DropdownItem, IconButton};
use crate::ui::export;
use crate::ui::file_ops;
use crate::ui::icon;
use crate::ui::image_import::ImageImport;
use crate::ui::notify;
use crate::ui::panel_zone::{
    animate_snap_transition, detect_snap_zone, grip_line_horizontal, grip_line_vertical,
    toolbar_container_class, DraggedPanel, PanelZone,
};
use crate::ui::styles;
use crate::util::window_size;
use leptos::*;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;

// ── ToolBar component ─────────────────────────────────────────────────

#[component]
pub fn ToolBar<F1, F2>(
    doc: DocContext,
    request_action: RequestAction,
    viewport: RwSignal<Viewport>,
    canvas_mode: RwSignal<CanvasMode>,
    selected_tool: RwSignal< crate::ui::dock::Tool>,
    eraser_active: RwSignal<bool>,
    on_undo: F1,
    on_redo: F2,
    can_undo: Signal<bool>,
    can_redo: Signal<bool>,
    export_crop_active: RwSignal<bool>,
    on_crop_export: RwSignal<Option<CropExportCallback>>,
    shortcuts_open: RwSignal<bool>,
    settings: RwSignal<CanvasSettings>,
    image_import: ImageImport,
    chart_import: ChartImport,
    #[prop(default = create_rw_signal(PanelZone::Top))] zone: RwSignal<PanelZone>,
    #[prop(default = create_rw_signal(None))] active_drag: RwSignal<
        Option<(DraggedPanel, Option<PanelZone>)>,
    >,
    #[prop(optional)] on_drop_snap: Option<Rc<dyn Fn()>>,
) -> impl IntoView
where
    F1: Fn() + 'static,
    F2: Fn() + 'static,
{
    let menu_open = create_rw_signal(false);
    let tauri = tauri_bridge::is_tauri();

    // Submenu hover tracking — one pair per fly-out panel
    let file_hover = create_rw_signal(false);
    let file_sub_hover = create_rw_signal(false);
    let show_file = Signal::derive(move || file_hover.get() || file_sub_hover.get());

    let tools_hover = create_rw_signal(false);
    let tools_sub_hover = create_rw_signal(false);
    let show_tools = Signal::derive(move || tools_hover.get() || tools_sub_hover.get());

    let import_hover = create_rw_signal(false);
    let import_sub_hover = create_rw_signal(false);
    let show_import = Signal::derive(move || import_hover.get() || import_sub_hover.get());

    let export_hover = create_rw_signal(false);
    let export_sub_hover = create_rw_signal(false);
    let show_export = Signal::derive(move || export_hover.get() || export_sub_hover.get());

    let on_home = move || viewport.set(Viewport::default());

    let close_menu = move || {
        menu_open.set(false);
        file_hover.set(false);
        file_sub_hover.set(false);
        tools_hover.set(false);
        tools_sub_hover.set(false);
        import_hover.set(false);
        import_sub_hover.set(false);
        export_hover.set(false);
        export_sub_hover.set(false);
    };

    // ── File actions ───────────────────────────────────────────────────

    let on_new = {
        let close_menu = close_menu;
        let doc = doc.clone();
        let request_action = request_action.clone();
        move || {
            close_menu();
            let doc = doc.clone();
            request_action(PendingAction {
                label: "start a new drawing",
                run: Rc::new(move || file_ops::file_new(doc.clone())),
            });
        }
    };
    let on_save_as = {
        let close_menu = close_menu;
        let doc = doc.clone();
        move || {
            close_menu();
            file_ops::file_save_as(doc.clone());
        }
    };
    let on_save = {
        let close_menu = close_menu;
        let doc = doc.clone();
        move || {
            close_menu();
            file_ops::file_save(doc.clone());
        }
    };
    let on_open = {
        let close_menu = close_menu;
        let doc = doc.clone();
        let request_action = request_action.clone();
        move || {
            close_menu();
            let doc = doc.clone();
            request_action(PendingAction {
                label: "open another drawing",
                run: Rc::new(move || file_ops::file_open(doc.clone())),
            });
        }
    };
    let on_import = {
        let close_menu = close_menu;
        let doc = doc.clone();
        let viewport = viewport;
        let canvas_mode = canvas_mode;
        let push_snapshot = image_import.push_snapshot.clone();
        move || {
            close_menu();
            if tauri {
                let doc = doc.clone();
                let push_snapshot = push_snapshot.clone();
                spawn_local(async move {
                    let dir = tauri_bridge::get_documents_dir().await.ok();
                    let path = tauri_bridge::pick_open_path(dir.as_deref()).await;
                    if let Some(path) = path {
                        match tauri_bridge::load_skea(&path).await {
                            Ok(c) => file_ops::import_drawing_from_str(
                                &doc,
                                viewport,
                                canvas_mode,
                                &push_snapshot,
                                &c,
                                None,
                            ),
                            Err(e) => crate::ui::notify::error(
                                doc.notifications,
                                format!("Could not open the file: {e}"),
                            ),
                        }
                    }
                });
            } else {
                file_ops::browser_import_drawing(
                    doc.clone(),
                    viewport,
                    canvas_mode,
                    push_snapshot.clone(),
                    None,
                );
            }
        }
    };
    let on_import_image = {
        let close_menu = close_menu;
        let image_import = image_import.clone();
        move || {
            close_menu();
            image_import.pick();
        }
    };

    // ── Export actions ─────────────────────────────────────────────────

    let on_export_skea = {
        let close_menu = close_menu;
        let doc = doc.clone();
        move || {
            close_menu();
            let s = doc.scene.get_untracked();
            let notifications = doc.notifications;
            let content = match skea::save_to_string(&s) {
                Ok(content) => content,
                Err(e) => {
                    crate::ui::notify::error(
                        notifications,
                        format!("Could not export the drawing: {e}"),
                    );
                    return;
                }
            };
            let stem =
                export::document_stem(doc.saved_path.get_untracked().as_deref(), "untitled");
            let default_name = format!("{stem}.skea");
            if tauri {
                spawn_local(async move {
                    let dir = tauri_bridge::get_documents_dir().await.ok();
                    let path = tauri_bridge::pick_save_path(&default_name, dir.as_deref()).await;
                    if let Some(path) = path {
                        match tauri_bridge::save_skea(&path, &content).await {
                            Ok(()) => crate::ui::notify::info(
                                notifications,
                                format!(
                                    "Exported {}",
                                    path.rsplit(['/', '\\']).next().unwrap_or(&path)
                                ),
                            ),
                            Err(e) => crate::ui::notify::error(
                                notifications,
                                format!("Could not export the drawing: {e}"),
                            ),
                        }
                    }
                });
            } else {
                export::download_string(&content, &default_name);
            }
        }
    };

    #[derive(Clone, Copy)]
    enum ExportFormat {
        Svg,
        Png,
    }

    #[derive(Clone, Copy)]
    enum ExportScope {
        Canvas,
        Selection,
        Crop,
    }

    let on_export = {
        let _close_menu = close_menu;
        let _export_crop_active = export_crop_active;
        let _on_crop_export = on_crop_export;
        let _doc = doc.clone();
        move |fmt: ExportFormat, scope: ExportScope| {
            _close_menu();
            let notifications = _doc.notifications;
            let bg = settings.get_untracked().theme;
            let stem =
                export::document_stem(_doc.saved_path.get_untracked().as_deref(), "canvas");

            if matches!(scope, ExportScope::Crop) {
                let scene = _doc.scene;
                let on_crop_export = _on_crop_export;
                let export_crop_active = _export_crop_active;
                let close_menu = _close_menu;
                let filename_stem = format!("{stem}-area");
                on_crop_export.set(Some(Rc::new(move |rect: Rect| {
                    close_menu();
                    let svg = scene.with(|s| export::scene_to_svg_crop(s, rect, bg));
                    match fmt {
                        ExportFormat::Svg => {
                            let filename = format!("{filename_stem}.svg");
                            if tauri {
                                export::tauri_export_svg_named(svg, filename, notifications);
                            } else {
                                export::download_string(&svg, &filename);
                            }
                        }
                        ExportFormat::Png => {
                            let filename = format!("{filename_stem}.png");
                            if tauri {
                                export::tauri_export_png_named(svg, filename, notifications);
                            } else {
                                export::download_png_from_svg(svg, filename, notifications);
                            }
                        }
                    }
                })));
                export_crop_active.set(true);
                notify::info(
                    notifications,
                    "Drag on the canvas to choose the export area. Esc cancels.",
                );
                return;
            }

            if matches!(scope, ExportScope::Selection) {
                let selected = _doc.selected_ids.get_untracked();
                if selected.is_empty() {
                    notify::info(
                        notifications,
                        "Drag on the canvas to choose the export area. Esc cancels.",
                    );
                    let scene = _doc.scene;
                    let on_crop_export = _on_crop_export;
                    let export_crop_active = _export_crop_active;
                    let close_menu = _close_menu;
                    let filename_stem = format!("{stem}-selection");
                    on_crop_export.set(Some(Rc::new(move |rect: Rect| {
                        close_menu();
                        let svg = scene.with(|s| export::scene_to_svg_crop(s, rect, bg));
                        match fmt {
                            ExportFormat::Svg => {
                                let filename = format!("{filename_stem}.svg");
                                if tauri {
                                    export::tauri_export_svg_named(svg, filename, notifications);
                                } else {
                                    export::download_string(&svg, &filename);
                                }
                            }
                            ExportFormat::Png => {
                                let filename = format!("{filename_stem}.png");
                                if tauri {
                                    export::tauri_export_png_named(svg, filename, notifications);
                                } else {
                                    export::download_png_from_svg(svg, filename, notifications);
                                }
                            }
                        }
                    })));
                    export_crop_active.set(true);
                    return;
                }

                let vp = viewport.get();
                let size = window_size();
                let svg = _doc
                    .scene
                    .with(|s| export::scene_to_svg(s, &vp, size, Some(&selected), bg));
                if svg.is_empty() {
                    notify::error(notifications, "Nothing to export.");
                    return;
                }
                let filename_stem = format!("{stem}-selection");
                match fmt {
                    ExportFormat::Svg => {
                        let filename = format!("{filename_stem}.svg");
                        if tauri {
                            export::tauri_export_svg_named(svg, filename, notifications);
                        } else {
                            export::download_string(&svg, &filename);
                        }
                    }
                    ExportFormat::Png => {
                        let filename = format!("{filename_stem}.png");
                        if tauri {
                            export::tauri_export_png_named(svg, filename, notifications);
                        } else {
                            export::download_png_from_svg(svg, filename, notifications);
                        }
                    }
                }
                return;
            }

            let vp = viewport.get();
            let size = window_size();
            let svg = _doc
                .scene
                .with(|s| export::scene_to_svg(s, &vp, size, None, bg));
            match fmt {
                ExportFormat::Svg => {
                    let filename = format!("{stem}.svg");
                    if tauri {
                        export::tauri_export_svg_named(svg, filename, notifications);
                    } else {
                        export::download_string(&svg, &filename);
                    }
                }
                ExportFormat::Png => {
                    let filename = format!("{stem}.png");
                    if tauri {
                        export::tauri_export_png_named(svg, filename, notifications);
                    } else {
                        export::download_png_from_svg(svg, filename, notifications);
                    }
                }
            }
        }
    };

    let on_export_frame = {
        let close_menu = close_menu;
        let doc = doc.clone();
        move |fmt: ExportFormat| {
            close_menu();
            let bg = settings.get_untracked().theme;
            let notifications = doc.notifications;
            let selected = doc.selected_ids.get_untracked();
            doc.scene.with_untracked(|s| {
                let target_id = selected
                    .iter()
                    .copied()
                    .find(|&id| matches!(s.element_by_id(id), Some(crate::model::Element::Frame(_))))
                    .or_else(|| s.frames_ordered().first().map(|f| f.data.id));
                match target_id {
                    Some(fid) => {
                        let as_png = matches!(fmt, ExportFormat::Png);
                        export::export_frame(s, fid, as_png, bg, notifications);
                    }
                    None => {
                        notify::error(notifications, "No frames in the drawing to export.");
                    }
                }
            });
        }
    };

    let on_export_all_frames_pdf = {
        let close_menu = close_menu;
        let doc = doc.clone();
        move || {
            close_menu();
            let bg = settings.get_untracked().theme;
            let notifications = doc.notifications;
            doc.scene.with_untracked(|s| {
                export::export_all_frames_pdf(s, bg, notifications);
            });
        }
    };

    let on_export_frame_pdf = {
        let close_menu = close_menu;
        let doc = doc.clone();
        move || {
            close_menu();
            let bg = settings.get_untracked().theme;
            let notifications = doc.notifications;
            let selected = doc.selected_ids.get_untracked();
            doc.scene.with_untracked(|s| {
                let target_id = selected
                    .iter()
                    .copied()
                    .find(|&id| matches!(s.element_by_id(id), Some(crate::model::Element::Frame(_))))
                    .or_else(|| s.frames_ordered().first().map(|f| f.data.id));
                match target_id {
                    Some(fid) => {
                        export::export_frame_pdf(s, fid, bg, notifications);
                    }
                    None => {
                        notify::error(notifications, "No frames in the drawing to export.");
                    }
                }
            });
        }
    };

    let on_export = Rc::new(on_export);

    let on_export_canvas_pdf = {
        let close_menu = close_menu;
        let doc = doc.clone();
        let _export_crop_active = export_crop_active;
        let _on_crop_export = on_crop_export;
        move |scope: ExportScope| {
            close_menu();
            let stem =
                export::document_stem(doc.saved_path.get_untracked().as_deref(), "canvas");
            let bg = settings.get_untracked().theme;
            let notifications = doc.notifications;

            if matches!(scope, ExportScope::Crop) {
                let scene = doc.scene;
                let on_crop_export = _on_crop_export;
                let export_crop_active = _export_crop_active;
                let default_name = format!("{stem}-area.pdf");
                on_crop_export.set(Some(Rc::new(move |rect: Rect| {
                    scene.with(|s| {
                        export::export_crop_pdf(s, rect, bg, default_name.clone(), notifications);
                    });
                })));
                export_crop_active.set(true);
                notify::info(
                    notifications,
                    "Drag on the canvas to choose the export area. Esc cancels.",
                );
                return;
            }

            let vp = viewport.get();
            let screen = window_size();
            let selected = doc.selected_ids.get_untracked();
            let (selected_ref, filename) = match scope {
                ExportScope::Selection => {
                    if selected.is_empty() {
                        notify::error(notifications, "Nothing selected to export.");
                        return;
                    }
                    (Some(selected.as_slice()), format!("{stem}-selection.pdf"))
                }
                _ => (None, format!("{stem}.pdf")),
            };
            doc.scene.with_untracked(|s| {
                export::export_canvas_pdf_named(
                    s,
                    &vp,
                    screen,
                    selected_ref,
                    bg,
                    filename,
                    notifications,
                );
            });
        }
    };

    // ── Dropdown item lists ────────────────────────────────────────────

    let file_items: Vec<DropdownItem> = vec![
        DropdownItem::Action {
            label: "New",
            on_click: Rc::new(on_new),
        },
        DropdownItem::Action {
            label: "Save",
            on_click: Rc::new(on_save),
        },
        DropdownItem::Action {
            label: "Save As",
            on_click: Rc::new(on_save_as),
        },
        DropdownItem::Action {
            label: "Open",
            on_click: Rc::new(on_open),
        },
    ];

    let tools_items: Vec<DropdownItem> = vec![
        DropdownItem::Action {
            label: "Chart",
            on_click: Rc::new({
                let ci = chart_import.clone();
                let close = close_menu;
                move || {
                    close();
                    ci.start_draw_chart(ChartKind::Bar);
                }
            }),
        },
        DropdownItem::Action {
            label: "Frame",
            on_click: Rc::new({
                let close = close_menu;
                move || {
                    close();
                    selected_tool.set(crate::ui::dock::Tool::Frame);
                    canvas_mode.set(CanvasMode::Draw);
                    eraser_active.set(false);
                }
            }),
        },
    ];

    let import_items: Vec<DropdownItem> = vec![
        DropdownItem::Action {
            label: "Drawing",
            on_click: Rc::new(on_import),
        },
        DropdownItem::Action {
            label: "Image",
            on_click: Rc::new(on_import_image),
        },
        DropdownItem::Action {
            label: "CSV / Data (Chart)",
            on_click: Rc::new({
                let ci = chart_import.clone();
                let close = close_menu;
                move || {
                    close();
                    ci.pick_csv();
                }
            }),
        },
    ];

    let export_items: Vec<DropdownItem> = vec![
        DropdownItem::Action {
            label: "Export .skea",
            on_click: Rc::new(on_export_skea),
        },
        DropdownItem::Separator,
        DropdownItem::Header { label: "PNG" },
        DropdownItem::Action {
            label: "Full canvas",
            on_click: Rc::new({
                let f = on_export.clone();
                move || f(ExportFormat::Png, ExportScope::Canvas)
            }),
        },
        DropdownItem::Action {
            label: "Selection",
            on_click: Rc::new({
                let f = on_export.clone();
                move || f(ExportFormat::Png, ExportScope::Selection)
            }),
        },
        DropdownItem::Action {
            label: "Selected area",
            on_click: Rc::new({
                let f = on_export.clone();
                move || f(ExportFormat::Png, ExportScope::Crop)
            }),
        },
        DropdownItem::Action {
            label: "Frame",
            on_click: Rc::new({
                let f = on_export_frame;
                move || f(ExportFormat::Png)
            }),
        },
        DropdownItem::Separator,
        DropdownItem::Header { label: "SVG" },
        DropdownItem::Action {
            label: "Full canvas",
            on_click: Rc::new({
                let f = on_export.clone();
                move || f(ExportFormat::Svg, ExportScope::Canvas)
            }),
        },
        DropdownItem::Action {
            label: "Selection",
            on_click: Rc::new({
                let f = on_export.clone();
                move || f(ExportFormat::Svg, ExportScope::Selection)
            }),
        },
        DropdownItem::Action {
            label: "Selected area",
            on_click: Rc::new({
                let f = on_export.clone();
                move || f(ExportFormat::Svg, ExportScope::Crop)
            }),
        },
        DropdownItem::Action {
            label: "Frame",
            on_click: Rc::new({
                let f = on_export_frame;
                move || f(ExportFormat::Svg)
            }),
        },
        DropdownItem::Separator,
        DropdownItem::Header { label: "PDF" },
        DropdownItem::Action {
            label: "All Frames (Gallery)",
            on_click: Rc::new({
                let f = on_export_all_frames_pdf;
                move || f()
            }),
        },
        DropdownItem::Action {
            label: "Frame",
            on_click: Rc::new({
                let f = on_export_frame_pdf;
                move || f()
            }),
        },
        DropdownItem::Action {
            label: "Full canvas",
            on_click: Rc::new({
                let f = on_export_canvas_pdf;
                move || f(ExportScope::Canvas)
            }),
        },
        DropdownItem::Action {
            label: "Selection",
            on_click: Rc::new({
                let f = on_export_canvas_pdf;
                move || f(ExportScope::Selection)
            }),
        },
        DropdownItem::Action {
            label: "Selected area",
            on_click: Rc::new({
                let f = on_export_canvas_pdf;
                move || f(ExportScope::Crop)
            }),
        },
    ];

    let is_dragging = create_rw_signal(false);
    let start_pos = create_rw_signal((0.0f64, 0.0f64));
    let toolbar_ref = create_node_ref::<html::Div>();

    let on_grip_pointerdown = move |ev: web_sys::PointerEvent| {
        if ev.button() != 0 {
            return;
        }
        ev.prevent_default();
        ev.stop_propagation();
        is_dragging.set(true);
        start_pos.set((ev.client_x() as f64, ev.client_y() as f64));
        active_drag.set(Some((DraggedPanel::Toolbar, None)));
    };

    let _ = window_event_listener(leptos::ev::pointermove, move |ev: web_sys::PointerEvent| {
        if !is_dragging.get_untracked() {
            return;
        }
        let (sx, sy) = start_pos.get_untracked();
        let cx = ev.client_x() as f64;
        let cy = ev.client_y() as f64;
        let dx = cx - sx;
        let dy = cy - sy;

        if let Some(div) = toolbar_ref.get() {
            let raw: &web_sys::HtmlElement = &div;
            let style = raw.style();
            let _ = style.set_property("transform", &format!("translate3d({dx}px, {dy}px, 0)"));
            let _ = style.set_property("transition", "none");
            let _ = style.set_property("z-index", "60");
        }

        let win_size = window_size();
        let snap = detect_snap_zone(cx, cy, win_size.x, win_size.y);
        active_drag.set(Some((DraggedPanel::Toolbar, snap)));
    });

    {
        let on_drop_snap = on_drop_snap.clone();
        let _ = window_event_listener(leptos::ev::pointerup, move |_ev: web_sys::PointerEvent| {
            if !is_dragging.get_untracked() {
                return;
            }
            is_dragging.set(false);

            let target_zone = match active_drag.get_untracked() {
                Some((DraggedPanel::Toolbar, Some(target))) => Some(target),
                _ => None,
            };
            active_drag.set(None);

            if let Some(div) = toolbar_ref.get() {
                let raw: &web_sys::HtmlElement = &div;
                let from_rect = raw.get_bounding_client_rect();
                let _ = raw.style().remove_property("z-index");

                if let Some(new_zone) = target_zone {
                    if new_zone != zone.get_untracked() {
                        zone.set(new_zone);
                        if let Some(ref cb) = on_drop_snap {
                            cb();
                        }
                    }
                }

                animate_snap_transition(raw, &from_rect);
            }
        });
    }

    let menu_dropdown_class = move || {
        match zone.get() {
            PanelZone::Top => styles::MENU_DROPDOWN,
            PanelZone::Bottom => "absolute bottom-full right-0 mb-1 z-50 min-w-40 rounded-lg bg-panel/95 backdrop-blur-sm border border-border shadow-xl py-1 pointer-events-auto",
            PanelZone::Left => "absolute top-0 left-full ml-1 z-50 min-w-40 rounded-lg bg-panel/95 backdrop-blur-sm border border-border shadow-xl py-1 pointer-events-auto",
            PanelZone::Right => "absolute top-0 right-full mr-1 z-50 min-w-40 rounded-lg bg-panel/95 backdrop-blur-sm border border-border shadow-xl py-1 pointer-events-auto",
        }
    };

    // ── Mode button helper ─────────────────────────────────────────────

    let btn = move |mode: CanvasMode| -> &'static str {
        if canvas_mode.get() == mode {
            styles::BTN_TBAR_ACTIVE
        } else {
            styles::BTN_TBAR_INACTIVE
        }
    };

    view! {
        <div
            node_ref=toolbar_ref
            class=move || toolbar_container_class(zone.get())
        >
            <div class=move || {
                let dir = if zone.get().is_horizontal() {
                    "flex-row"
                } else {
                    "flex-col"
                };
                format!("flex items-center gap-0.5 rounded-lg bg-panel/80 backdrop-blur-sm border border-border panel-glow pointer-events-auto p-0.5 {dir}")
            }>
                <div
                    class="cursor-grab active:cursor-grabbing p-1.5 text-subtle/60 hover:text-fg select-none rounded flex items-center justify-center transition-colors group"
                    on:pointerdown=on_grip_pointerdown
                    title="Drag to move toolbar"
                >
                    {move || {
                        if zone.get().is_horizontal() {
                            grip_line_vertical().into_view()
                        } else {
                            grip_line_horizontal().into_view()
                        }
                    }}
                </div>
                <button
                    class=move || btn(CanvasMode::Pan)
                    on:click=move |_| canvas_mode.set(CanvasMode::Pan)
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
                <IconButton on_click=on_home title="Home" class=styles::BTN_GHOST>
                    {icon::home()}
                </IconButton>
                <button
                    class=styles::BTN_GHOST
                    class:opacity-40=move || !can_undo.get()
                    class:cursor-not-allowed=move || !can_undo.get()
                    on:click=move |_| on_undo()
                    title="Undo"
                >
                    {icon::undo()}
                </button>
                <button
                    class="flex items-center justify-center h-8 rounded-md text-xs text-subtle hover:text-fg transition-colors tabular-nums min-w-[2.5rem]"
                    on:click=move |_| viewport.update(|vp| vp.zoom = 1.0)
                    title="Reset zoom to 100%"
                >
                    {move || {
                        let pct = (viewport.get().zoom * 100.0).round() as i64;
                        format!("{}%", pct)
                    }}
                </button>
                <button
                    class=styles::BTN_GHOST
                    class:opacity-40=move || !can_redo.get()
                    class:cursor-not-allowed=move || !can_redo.get()
                    on:click=move |_| on_redo()
                    title="Redo"
                >
                    {icon::redo()}
                </button>
                <div class="relative">
                    <IconButton
                        on_click=move || menu_open.update(|v| *v = !*v)
                        title="Menu"
                        class=styles::BTN_GHOST
                    >
                        {icon::menu()}
                    </IconButton>
                    {move || {
                        if menu_open.get() {
                            view! {
                                <>
                                    <div
                                        class="fixed inset-0 z-40"
                                        on:click=move |_| close_menu()
                                    ></div>
                                    <div class=menu_dropdown_class>
                                        // ── File ─────────────────────────────
                                        <div
                                            class="relative"
                                            on:mouseenter=move |_| file_hover.set(true)
                                            on:mouseleave=move |_| file_hover.set(false)
                                        >
                                            <button class=styles::MENU_ITEM>
                                                <span>"File"</span>
                                                {icon::chevron_right()}
                                            </button>
                                            <Dropdown
                                                show=show_file
                                                on_mouseenter=Rc::new({
                                                    let s = file_sub_hover;
                                                    move || s.set(true)
                                                })
                                                on_mouseleave=Rc::new({
                                                    let s = file_sub_hover;
                                                    move || s.set(false)
                                                })
                                                items=file_items.clone()
                                            />
                                        </div>

                                        // ── Tools ────────────────────────────
                                        <div
                                            class="relative"
                                            on:mouseenter=move |_| tools_hover.set(true)
                                            on:mouseleave=move |_| tools_hover.set(false)
                                        >
                                            <button class=styles::MENU_ITEM>
                                                <span>"Tools"</span>
                                                {icon::chevron_right()}
                                            </button>
                                            <Dropdown
                                                show=show_tools
                                                on_mouseenter=Rc::new({
                                                    let s = tools_sub_hover;
                                                    move || s.set(true)
                                                })
                                                on_mouseleave=Rc::new({
                                                    let s = tools_sub_hover;
                                                    move || s.set(false)
                                                })
                                                items=tools_items.clone()
                                            />
                                        </div>

                                        // ── Import ───────────────────────────
                                        <div
                                            class="relative"
                                            on:mouseenter=move |_| import_hover.set(true)
                                            on:mouseleave=move |_| import_hover.set(false)
                                        >
                                            <button class=styles::MENU_ITEM>
                                                <span>"Import"</span>
                                                {icon::chevron_right()}
                                            </button>
                                            <Dropdown
                                                show=show_import
                                                on_mouseenter=Rc::new({
                                                    let s = import_sub_hover;
                                                    move || s.set(true)
                                                })
                                                on_mouseleave=Rc::new({
                                                    let s = import_sub_hover;
                                                    move || s.set(false)
                                                })
                                                items=import_items.clone()
                                            />
                                        </div>

                                        // ── Export ───────────────────────────
                                        <div
                                            class="relative"
                                            on:mouseenter=move |_| export_hover.set(true)
                                            on:mouseleave=move |_| export_hover.set(false)
                                        >
                                            <button class=styles::MENU_ITEM>
                                                <span>"Export"</span>
                                                {icon::chevron_right()}
                                            </button>
                                            <Dropdown
                                                show=show_export
                                                on_mouseenter=Rc::new({
                                                    let s = export_sub_hover;
                                                    move || s.set(true)
                                                })
                                                on_mouseleave=Rc::new({
                                                    let s = export_sub_hover;
                                                    move || s.set(false)
                                                })
                                                items=export_items.clone()
                                            />
                                        </div>

                                        // ── Help ─────────────────────────────
                                        <button
                                            class=styles::MENU_ITEM
                                            on:click=move |_| {
                                                close_menu();
                                                shortcuts_open.set(true);
                                            }
                                        >
                                            <span>"Help"</span>
                                        </button>
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
