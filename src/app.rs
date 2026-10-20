use std::rc::Rc;

use crate::canvas::settings::{CanvasBg, CanvasSettings, CenterStyle, GridSize, GridStyle};
use crate::canvas::{Canvas, CanvasMode, CropExportCallback, Viewport};
use crate::document::{DocContext, PendingAction, RequestAction};
use crate::edit_ops::Clipboard;
use crate::hotkeys::{register_hotkeys, HotkeysContext, ShortcutsModal};
use crate::model::{Color, ElementId, ElementStyle, LineStyle, Point, Scene};
use crate::skea;
use crate::tauri_bridge;
use crate::ui::components::StylingPanel;
use crate::ui::dock::{Dock, Tool};
use crate::ui::notify::{self, Toasts};
use crate::ui::settings::{from_toml, to_toml};
use crate::ui::styles;
use crate::ui::{file_ops, SettingsPanel, ToolBar, UnsavedChangesDialog};
use leptos::*;
use std::time::Duration;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

/// Update the window title on both targets: `document.title` for the browser,
/// and the native window through Tauri (which does not track document.title).
fn update_window_title(title: &str) {
    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
        document.set_title(title);
    }
    if tauri_bridge::is_tauri() {
        tauri_bridge::set_window_title(title);
    }
}

#[component]
pub fn App() -> impl IntoView {
    let cursor_screen = create_rw_signal((0.0_f64, 0.0_f64));
    let cursor_world = create_rw_signal(Point::new(0.0, 0.0));
    let viewport = create_rw_signal(Viewport::default());

    let selected_tool = create_rw_signal(Tool::Rectangle);
    let selected_color = create_rw_signal(Color::new(Color::WHITE));
    let canvas_mode = create_rw_signal(CanvasMode::Select);

    let scene = create_rw_signal(Scene::new());
    let selected_ids = create_rw_signal(Vec::<ElementId>::new());
    let eraser_active = create_rw_signal(false);
    let shortcuts_open = create_rw_signal(false);

    let export_crop_active = create_rw_signal(false);
    let on_crop_export = create_rw_signal::<Option<CropExportCallback>>(None);
    let on_text_commit = create_rw_signal::<Option<Rc<dyn Fn()>>>(None);

    let settings = create_rw_signal(CanvasSettings {
        center_style: CenterStyle::Crosshair,
        grid_style: GridStyle::Dot,
        grid_size: GridSize::new(30.0),
        autosave: false,
        canvas_bg: CanvasBg::Dark,
    });

    let clipboard = create_rw_signal(Clipboard::default());

    let undo_stack = create_rw_signal(Vec::<Scene>::new());
    let redo_stack = create_rw_signal(Vec::<Scene>::new());

    let push_snapshot = Rc::new(move || {
        undo_stack.update(|s| {
            s.push(scene.get());
            if s.len() > 100 {
                s.remove(0);
            }
        });
        redo_stack.set(Vec::new());
    });

    // Pops the most recent undo snapshot, but only when it still matches the
    // current scene. Aborted interactions (a freehand click, Esc mid-stroke,
    // dismissing an untouched new text) end exactly at the state their
    // snapshot recorded, so undo history gets no no-op entries; anything that
    // changed the scene in between is left alone.
    let discard_snapshot = Rc::new(move || {
        let current = scene.get_untracked();
        undo_stack.update(|s| {
            if s.last() == Some(&current) {
                s.pop();
            }
        });
    });

    let do_undo = {
        let scene = scene;
        let undo_stack = undo_stack;
        let redo_stack = redo_stack;
        move || {
            let mut prev = None;
            undo_stack.update(|s| prev = s.pop());
            if let Some(prev) = prev {
                let current = scene.get();
                scene.set(prev);
                redo_stack.update(|s| s.push(current));
            }
        }
    };

    let do_redo = {
        let scene = scene;
        let undo_stack = undo_stack;
        let redo_stack = redo_stack;
        move || {
            let mut next = None;
            redo_stack.update(|s| next = s.pop());
            if let Some(next) = next {
                let current = scene.get();
                scene.set(next);
                undo_stack.update(|s| s.push(current));
            }
        }
    };

    // `with` instead of `get`: these memos run whenever the history changes
    // and must not deep-clone the (up to 100-scene) stacks.
    let can_undo = Signal::derive(move || undo_stack.with(|s| !s.is_empty()));
    let can_redo = Signal::derive(move || redo_stack.with(|s| !s.is_empty()));

    let saved_path = create_rw_signal::<Option<String>>(None);

    let reset_history = {
        let undo_stack = undo_stack;
        let redo_stack = redo_stack;
        Rc::new(move || {
            undo_stack.set(Vec::new());
            redo_stack.set(Vec::new());
        })
    };

    let baseline = create_rw_signal(Scene::new());
    let dirty = create_rw_signal(false);
    let notifications = create_rw_signal(Vec::new());

    let clear_autosave = Rc::new(|| {
        if !tauri_bridge::is_tauri() {
            crate::storage::clear_autosave();
        }
    });

    let doc = DocContext {
        scene,
        saved_path,
        selected_ids,
        baseline,
        dirty,
        notifications,
        generation: create_rw_signal(0_u32),
        reset_history: reset_history.clone(),
        clear_autosave,
    };

    let is_tauri = tauri_bridge::is_tauri();

    // Actions that replace or discard the document ask for confirmation when
    // there are unsaved edits. The action itself is stored so it can run after
    // the dialog resolves.
    let pending_action = create_rw_signal::<Option<PendingAction>>(None);
    let request_action: RequestAction = {
        let doc = doc.clone();
        Rc::new(move |action: PendingAction| {
            if doc.is_dirty_now() {
                pending_action.set(Some(action));
            } else {
                (action.run)();
            }
        })
    };

    if is_tauri {
        let request = request_action.clone();
        tauri_bridge::on_close_requested(move || {
            // Text edits live outside the scene until committed; flush one so
            // the dirty check below sees it.
            if let Some(commit) = on_text_commit.get_untracked() {
                commit();
            }
            request(PendingAction {
                label: "close Inskea",
                run: Rc::new(tauri_bridge::close_window),
            });
        });

        // A drawing passed on the command line, or forwarded by a second
        // instance, opens through the same unsaved-changes guard.
        let open_file: Rc<dyn Fn(String)> = {
            let doc = doc.clone();
            let request = request_action.clone();
            Rc::new(move |path: String| {
                let doc = doc.clone();
                request(PendingAction {
                    label: "open another drawing",
                    run: Rc::new(move || file_ops::open_path(doc.clone(), path.clone())),
                });
            })
        };
        let listener_open = open_file.clone();
        tauri_bridge::on_open_file(move |path| listener_open(path));
        spawn_local(async move {
            if let Some(path) = tauri_bridge::startup_file().await {
                open_file(path);
            }
        });
    } else if let Some(window) = web_sys::window() {
        let doc = doc.clone();
        let handler = wasm_bindgen::closure::Closure::wrap(Box::new(
            move |event: web_sys::BeforeUnloadEvent| {
                if let Some(commit) = on_text_commit.get_untracked() {
                    commit();
                }
                if doc.is_dirty_now() {
                    event.prevent_default();
                    event.set_return_value("You have unsaved changes.");
                }
            },
        ) as Box<dyn FnMut(_)>);
        let _ = window
            .add_event_listener_with_callback("beforeunload", handler.as_ref().unchecked_ref());
        handler.forget();
    }

    let dialog_save = {
        let doc = doc.clone();
        Callback::new(move |_: ()| {
            let doc = doc.clone();
            file_ops::file_save_with(
                doc,
                Rc::new(move |saved| {
                    if let Some(action) = pending_action.get_untracked() {
                        if saved {
                            pending_action.set(None);
                            (action.run)();
                        }
                    }
                }),
            );
        })
    };
    let dialog_discard = Callback::new(move |_: ()| {
        if let Some(action) = pending_action.get_untracked() {
            pending_action.set(None);
            (action.run)();
        }
    });
    let dialog_cancel = Callback::new(move |_: ()| pending_action.set(None));

    // Escape dismisses the unsaved-changes dialog. Shortcuts are blocked while
    // it is open, so this is the only key it listens for.
    let modal_open = Signal::derive(move || pending_action.get().is_some());
    {
        let _ = window_event_listener(leptos::ev::keydown, move |ev: leptos::ev::KeyboardEvent| {
            if ev.key() == "Escape" && pending_action.get_untracked().is_some() {
                ev.prevent_default();
                dialog_cancel.call(());
            }
        });
    }

    // Window title: the file name plus a `*` while the scene differs from
    // what was last written to disk.
    {
        let saved_path = saved_path;
        create_effect(move |_| {
            let is_dirty = dirty.get();
            let name = saved_path
                .get()
                .as_deref()
                .and_then(|path| std::path::Path::new(path).file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("Untitled")
                .to_string();
            let title = format!("{name}{} - Inskea", if is_dirty { " *" } else { "" });
            update_window_title(&title);
        });
    }

    // Re-derive the dirty flag shortly after edits settle. Comparing the
    // whole scene is too heavy for drag frames, but cheap at this cadence.
    {
        let doc = doc.clone();
        let dirty_generation = create_rw_signal(0_u64);
        create_effect(move |_| {
            scene.with(|_| ());
            let generation = dirty_generation.get_untracked().wrapping_add(1);
            dirty_generation.set(generation);
            let doc = doc.clone();
            set_timeout(
                move || {
                    if dirty_generation.get_untracked() == generation {
                        dirty.set(doc.is_dirty_now());
                    }
                },
                Duration::from_millis(250),
            );
        });
    }

    let ctx = HotkeysContext {
        canvas_mode,
        selected_tool,
        eraser_active,
        doc: doc.clone(),
        request_action: request_action.clone(),
        clipboard,
        cursor_world,
        shortcuts_open,
        modal_open,
        push_snapshot: push_snapshot.clone(),
        do_undo: Rc::new(do_undo),
        do_redo: Rc::new(do_redo),
    };

    register_hotkeys(ctx);

    let initialized = create_rw_signal(false);

    if is_tauri {
        crate::fonts::ensure_fonts_installed();
        spawn_local(async move {
            if let Ok(content) = tauri_bridge::load_settings().await {
                if let Some((cs, gs, gz, auto, bg)) = from_toml(&content) {
                    settings.set(CanvasSettings {
                        center_style: cs,
                        grid_style: gs,
                        grid_size: gz,
                        autosave: auto,
                        canvas_bg: bg,
                    });
                }
            }
            initialized.set(true);
        });
    } else {
        if let Some(content) = crate::storage::load_settings() {
            if let Some((cs, gs, gz, auto, bg)) = from_toml(&content) {
                settings.set(CanvasSettings {
                    center_style: cs,
                    grid_style: gs,
                    grid_size: gz,
                    autosave: auto,
                    canvas_bg: bg,
                });
            }
        }
        initialized.set(true);
    }

    // Recover the browser autosave, if one exists and autosave is enabled.
    if !is_tauri && settings.get_untracked().autosave {
        if let Some(content) = crate::storage::load_autosave() {
            if let Ok(restored) = skea::load_from_str(&content) {
                if !restored.elements().is_empty() {
                    doc.restore(restored);
                    notify::info(doc.notifications, "Restored the autosaved drawing.");
                }
            }
        }
    }

    create_effect(move |_| {
        let s = settings.get();
        if initialized.get() {
            let content = to_toml(
                s.center_style,
                s.grid_style,
                s.grid_size,
                s.autosave,
                s.canvas_bg,
            );
            if is_tauri {
                spawn_local(async move {
                    let _ = tauri_bridge::save_settings(&content).await;
                });
            } else {
                crate::storage::save_settings(&content);
            }
        }
    });

    // ── Autosave ───────────────────────────────────────────────────────────
    // Writes the scene back a moment after the last change so a burst of edits
    // collapses into one save. On desktop it targets the document path (and
    // asks for one once if there is none); in the browser it targets
    // localStorage so a reload restores the drawing.
    let autosave_generation = create_rw_signal(0_u64);
    let autosave_prompted = create_rw_signal(false);
    let autosave_was_on = create_rw_signal(false);
    let autosave_doc = doc.clone();
    create_effect(move |_| {
        // Track the inputs without cloning the scene on every drag frame;
        // the deep clone only happens once per debounced save.
        scene.with(|_| ());
        saved_path.with(|_| ());
        if saved_path.get_untracked().is_some() {
            autosave_prompted.set(false);
        }
        let autosave_on = settings.get().autosave;
        // Turning autosave back on after cancelling the file prompt should
        // ask for a file again.
        if autosave_on && !autosave_was_on.get_untracked() {
            autosave_prompted.set(false);
        }
        autosave_was_on.set(autosave_on);
        if !autosave_on {
            return;
        }
        let generation = autosave_generation.get_untracked().wrapping_add(1);
        autosave_generation.set(generation);
        let doc = autosave_doc.clone();
        set_timeout(
            move || {
                if autosave_generation.get_untracked() != generation {
                    return;
                }
                if !doc.is_dirty_now() {
                    return;
                }
                let saved = doc.scene.get_untracked();
                let content = match skea::save_to_string(&saved) {
                    Ok(content) => content,
                    Err(e) => {
                        notify::error(doc.notifications, format!("Could not autosave: {e}"));
                        return;
                    }
                };
                if !is_tauri {
                    if crate::storage::save_autosave(&content) {
                        doc.mark_saved(None, saved);
                    } else {
                        notify::error(doc.notifications, "Could not autosave to browser storage.");
                    }
                    return;
                }
                let Some(path) = saved_path.get_untracked() else {
                    if !autosave_prompted.get_untracked() {
                        autosave_prompted.set(true);
                        let settings = settings;
                        let notifications = doc.notifications;
                        file_ops::file_save_with(
                            doc.clone(),
                            Rc::new(move |saved| {
                                if !saved {
                                    settings.update(|s| s.autosave = false);
                                    notify::error(
                                        notifications,
                                        "Autosave could not save this drawing.",
                                    );
                                }
                            }),
                        );
                    }
                    return;
                };
                let notifications = doc.notifications;
                let generation = doc.generation_now();
                spawn_local(async move {
                    match tauri_bridge::save_skea(&path, &content).await {
                        Ok(()) => doc.mark_saved_for(generation, Some(path), saved),
                        Err(e) => {
                            if doc.generation_now() == generation {
                                notify::error(notifications, format!("Could not autosave: {e}"));
                            }
                        }
                    }
                });
            },
            Duration::from_millis(800),
        );
    });

    // ── Default style for new shapes (draw-mode panel changes) ────────────
    let default_style = create_rw_signal(ElementStyle {
        stroke_color: selected_color.get_untracked(),
        ..Default::default()
    });
    let default_line_style = create_rw_signal(LineStyle::default());

    create_effect(move |_| {
        let is_arrow = selected_tool.get() == Tool::Arrow;
        default_line_style.update(|ls| ls.has_end_arrowhead = is_arrow);
    });

    // ── Panel visibility ───────────────────────────────────────────────────
    let show_panel = Signal::derive(move || {
        let mode = canvas_mode.get();
        if mode == CanvasMode::Select && !selected_ids.get().is_empty() {
            return true;
        }
        if mode == CanvasMode::Draw && !eraser_active.get() {
            return true;
        }
        false
    });

    view! {
        <div class=move || {
            let bg = if settings.get().canvas_bg == CanvasBg::Dark { "bg-bg" } else { "bg-white" };
            format!("w-screen h-screen {bg} text-fg")
        }>
            <Canvas
                cursor_screen=cursor_screen
                cursor_world=cursor_world
                viewport=viewport
                selected_tool=selected_tool
                selected_color=selected_color
                canvas_mode=canvas_mode
                scene=scene
                eraser_active=eraser_active
                settings=settings
                push_snapshot=push_snapshot.clone()
                discard_snapshot=discard_snapshot.clone()
                export_crop_active=export_crop_active
                on_crop_export=on_crop_export
                on_text_commit=on_text_commit
                shortcuts_open=shortcuts_open
                selected_ids=selected_ids
                default_style=default_style
                default_line_style=default_line_style
            />
            <ToolBar
                doc=doc.clone()
                request_action=request_action.clone()
                viewport=viewport
                canvas_mode=canvas_mode
                on_undo=do_undo
                on_redo=do_redo
                can_undo=can_undo
                can_redo=can_redo
                export_crop_active=export_crop_active
                on_crop_export=on_crop_export
                shortcuts_open=shortcuts_open
                settings=settings
            />
            <Dock selected_tool=selected_tool canvas_mode=canvas_mode eraser_active=eraser_active />
            <SettingsPanel settings=settings />
            <ShortcutsModal shortcuts_open=shortcuts_open />
            <UnsavedChangesDialog
                pending=pending_action
                on_save=dialog_save
                on_discard=dialog_discard
                on_cancel=dialog_cancel
            />
            <Toasts notifications=notifications />

            <div
                class="fixed left-[4.5rem] top-1/2 -translate-y-1/2 z-50"
                class:hidden=move || !show_panel.get()
            >
                <div class=styles::PANEL>
                    <div class="py-2 px-3">
                        <StylingPanel
                            scene=scene
                            selected_ids=selected_ids
                            selected_tool=selected_tool
                            default_style=default_style
                            default_line_style=default_line_style
                            clipboard=clipboard
                            push_snapshot=push_snapshot.clone()
                            canvas_mode=canvas_mode
                            cursor_world=cursor_world
                        />
                    </div>
                </div>
            </div>
        </div>
    }
}
