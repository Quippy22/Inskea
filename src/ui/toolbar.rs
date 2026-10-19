#![allow(clippy::redundant_locals)]
use crate::canvas::{CanvasMode, CropExportCallback, Viewport};
use crate::document::{DocContext, PendingAction, RequestAction};
use crate::model::Rect;
use crate::skea;
use crate::tauri_bridge;
use crate::ui::components::{Dropdown, DropdownItem, IconButton};
use crate::ui::export;
use crate::ui::file_ops;
use crate::ui::icon;
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
    on_undo: F1,
    on_redo: F2,
    can_undo: Signal<bool>,
    can_redo: Signal<bool>,
    export_crop_active: RwSignal<bool>,
    on_crop_export: RwSignal<Option<CropExportCallback>>,
    shortcuts_open: RwSignal<bool>,
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

    let export_hover = create_rw_signal(false);
    let export_sub_hover = create_rw_signal(false);
    let show_export = Signal::derive(move || export_hover.get() || export_sub_hover.get());

    let on_home = move || viewport.set(Viewport::default());

    let close_menu = move || {
        menu_open.set(false);
        file_hover.set(false);
        file_sub_hover.set(false);
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
        let request_action = request_action.clone();
        move || {
            close_menu();
            let doc = doc.clone();
            let run = Rc::new(move || {
                if tauri {
                    let doc = doc.clone();
                    spawn_local(async move {
                        let path = tauri_bridge::pick_open_path(None).await;
                        if let Some(path) = path {
                            match tauri_bridge::load_skea(&path).await {
                                Ok(c) => match skea::load_from_str(&c) {
                                    Ok(loaded) => doc.adopt(loaded, Some(path)),
                                    Err(e) => crate::ui::notify::error(
                                        doc.notifications,
                                        format!("That file is not a valid Inskea drawing: {e}"),
                                    ),
                                },
                                Err(e) => crate::ui::notify::error(
                                    doc.notifications,
                                    format!("Could not open the file: {e}"),
                                ),
                            }
                        }
                    });
                } else {
                    file_ops::browser_open(doc.clone());
                }
            });
            request_action(PendingAction {
                label: "import another drawing",
                run,
            });
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
            if tauri {
                spawn_local(async move {
                    let dir = tauri_bridge::get_documents_dir().await.ok();
                    let path = tauri_bridge::pick_save_path("untitled.skea", dir.as_deref()).await;
                    if let Some(path) = path {
                        let c = skea::save_to_string(&s);
                        match tauri_bridge::save_skea(&path, &c).await {
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
                export::browser_export_skea(s);
            }
        }
    };

    #[derive(Clone, Copy)]
    enum ExportFormat {
        Svg,
        Png,
    }

    let on_export = {
        let _close_menu = close_menu;
        let _export_crop_active = export_crop_active;
        let _on_crop_export = on_crop_export;
        let _doc = doc.clone();
        move |fmt: ExportFormat, selection: bool| {
            _close_menu();
            if selection {
                let scene = _doc.scene;
                let notifications = _doc.notifications;
                let on_crop_export = _on_crop_export;
                let export_crop_active = _export_crop_active;
                let close_menu = _close_menu;
                on_crop_export.set(Some(Rc::new(move |rect: Rect| {
                    close_menu();
                    let svg = scene.with(|s| export::scene_to_svg_crop(s, rect));
                    match fmt {
                        ExportFormat::Svg => {
                            if tauri {
                                export::tauri_export_svg(svg, true, notifications);
                            } else {
                                export::download_string(&svg, "selection.svg");
                            }
                        }
                        ExportFormat::Png => {
                            if tauri {
                                export::tauri_export_png(svg, true, notifications);
                            } else {
                                export::download_png_from_svg(
                                    svg,
                                    "selection.png".to_string(),
                                    notifications,
                                );
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
                .with(|s| export::scene_to_svg(s, &vp, size, None));
            let notifications = _doc.notifications;
            match fmt {
                ExportFormat::Svg => {
                    if tauri {
                        export::tauri_export_svg(svg, false, notifications);
                    } else {
                        export::download_string(&svg, "canvas.svg");
                    }
                }
                ExportFormat::Png => {
                    if tauri {
                        export::tauri_export_png(svg, false, notifications);
                    } else {
                        export::download_png_from_svg(svg, "canvas.png".to_string(), notifications);
                    }
                }
            }
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
        DropdownItem::Separator,
        DropdownItem::Action {
            label: "Import",
            on_click: Rc::new(on_import),
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
                let f = on_export;
                move || f(ExportFormat::Png, false)
            }),
        },
        DropdownItem::Action {
            label: "Selection",
            on_click: Rc::new({
                let f = on_export;
                move || f(ExportFormat::Png, true)
            }),
        },
        DropdownItem::Separator,
        DropdownItem::Header { label: "SVG" },
        DropdownItem::Action {
            label: "Full canvas",
            on_click: Rc::new({
                let f = on_export;
                move || f(ExportFormat::Svg, false)
            }),
        },
        DropdownItem::Action {
            label: "Selection",
            on_click: Rc::new({
                let f = on_export;
                move || f(ExportFormat::Svg, true)
            }),
        },
    ];

    // ── Mode button helper ─────────────────────────────────────────────

    let btn = move |mode: CanvasMode| -> &'static str {
        if canvas_mode.get() == mode {
            styles::BTN_TBAR_ACTIVE
        } else {
            styles::BTN_TBAR_INACTIVE
        }
    };

    view! {
        <div class=styles::CONTAINER_STATUSBAR>
            <div class=styles::TBAR_INNER>
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
                    class="flex items-center justify-center h-8 w-8 rounded-md opacity-40 cursor-default"
                    title="Pages"
                >
                    {icon::pages()}
                </button>
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
                                    <div class=styles::MENU_DROPDOWN>
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
