use crate::canvas::settings::CanvasSettings;
use crate::canvas::{CanvasMode, Viewport};
use crate::edit_ops;
use crate::model::{Bounds, ElementId, Frame, FramePreset, Scene};
use crate::ui::components::IconButton;
use crate::ui::dock::Tool;
use crate::ui::notify::Notifications;
use crate::ui::{export, icon, styles};
use crate::util::window_size;
use leptos::*;
use std::rc::Rc;

/// Context menu state when right-clicking a frame row in the Frames gallery.
#[derive(Clone, Copy, Debug)]
pub struct FrameMenu {
    pub id: ElementId,
    pub index: usize,
    pub x: f64,
    pub y: f64,
}

/// Shared state for the top-right Frames panel and the Frame Read / View Mode overlay.
#[derive(Clone)]
pub struct FramesContext {
    pub open: RwSignal<bool>,
    pub scene: RwSignal<Scene>,
    pub selected_ids: RwSignal<Vec<ElementId>>,
    pub viewport: RwSignal<Viewport>,
    pub canvas_mode: RwSignal<CanvasMode>,
    pub selected_tool: RwSignal<Tool>,
    pub eraser_active: RwSignal<bool>,
    pub push_snapshot: Rc<dyn Fn()>,
    pub notifications: Notifications,
    pub settings: RwSignal<CanvasSettings>,
    /// Currently inline-renaming frame ID and its draft text.
    pub renaming_id: RwSignal<Option<ElementId>>,
    pub rename_draft: RwSignal<String>,
    /// Active 0-based frame index when Read / View mode is open (`None` when normal editing).
    pub read_mode_index: RwSignal<Option<usize>>,
    /// Right-click context menu on a frame row in the Frames gallery.
    pub menu: RwSignal<Option<FrameMenu>>,
}

impl FramesContext {
    /// Zoom and pan the viewport so `frame_id` fits comfortably in the window.
    pub fn focus_frame(&self, frame_id: ElementId) {
        let bounds = self.scene.with_untracked(|s| {
            s.element_by_id(frame_id).map(|el| el.bounds())
        });
        if let Some(rect) = bounds {
            let screen = window_size();
            self.viewport
                .update(|vp| vp.fit_rect(rect, screen, 72.0, 0.1, 20.0));
        }
    }

    /// Focus and select `frame_id` in Select mode.
    pub fn select_and_focus(&self, frame_id: ElementId) {
        self.focus_frame(frame_id);
        self.selected_ids.set(vec![frame_id]);
        self.canvas_mode.set(CanvasMode::Select);
    }

    /// Export `frame_id` as PNG (`as_png = true`) or SVG (`as_png = false`).
    pub fn export_frame(&self, frame_id: ElementId, as_png: bool) {
        let bg = self.settings.get_untracked().theme;
        let notifications = self.notifications;
        self.scene
            .with_untracked(|s| export::export_frame(s, frame_id, as_png, bg, notifications));
    }

    /// Export `frame_id` as a 1-page PDF.
    pub fn export_frame_pdf(&self, frame_id: ElementId) {
        let bg = self.settings.get_untracked().theme;
        let notifications = self.notifications;
        self.scene
            .with_untracked(|s| export::export_frame_pdf(s, frame_id, bg, notifications));
    }

    /// Export all frames in document sequence as a multi-page PDF document.
    pub fn export_all_frames_pdf(&self) {
        let bg = self.settings.get_untracked().theme;
        let notifications = self.notifications;
        self.scene
            .with_untracked(|s| export::export_all_frames_pdf(s, bg, notifications));
    }

    /// Delete `frame_id` from the scene.
    pub fn delete_frame(&self, frame_id: ElementId) {
        edit_ops::mutate_scene(self.scene, &self.push_snapshot, |s| {
            s.remove_by_id(frame_id);
        });
        self.selected_ids.update(|ids| ids.retain(|&id| id != frame_id));
    }

    /// Start inline renaming a frame.
    pub fn begin_rename(&self, frame: &Frame) {
        self.rename_draft.set(frame.name.clone());
        self.renaming_id.set(Some(frame.data.id));
    }

    /// Commit the current inline rename draft.
    pub fn commit_rename(&self) {
        let Some(id) = self.renaming_id.get_untracked() else {
            return;
        };
        let draft = self.rename_draft.get_untracked();
        self.renaming_id.set(None);
        let trimmed = draft.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        let changed = self.scene.with_untracked(|s| {
            s.frames_ordered()
                .into_iter()
                .find(|f| f.data.id == id)
                .is_some_and(|f| f.name != trimmed)
        });
        if changed {
            edit_ops::mutate_scene(self.scene, &self.push_snapshot, |s| {
                s.rename_frame(id, trimmed);
            });
        }
    }

    /// Move `frame_id` up (`delta = -1`) or down (`delta = 1`) in the frame order.
    pub fn move_frame(&self, frame_id: ElementId, delta: isize) {
        let frames = self.scene.with_untracked(|s| s.frames_ordered());
        let Some(cur) = frames.iter().position(|f| f.data.id == frame_id) else {
            return;
        };
        let target = cur as isize + delta;
        if target < 0 || target >= frames.len() as isize {
            return;
        }
        edit_ops::mutate_scene(self.scene, &self.push_snapshot, |s| {
            s.reorder_frame(frame_id, target as usize);
        });
    }

    /// Enter Read / View mode at the given 0-based frame index (or 0 if none specified).
    pub fn start_read_mode(&self, start_index: usize) {
        let frames = self.scene.with_untracked(|s| s.frames_ordered());
        if frames.is_empty() {
            return;
        }
        let idx = start_index.min(frames.len() - 1);
        self.open.set(false);
        self.menu.set(None);
        self.renaming_id.set(None);
        self.selected_ids.set(Vec::new());
        self.read_mode_index.set(Some(idx));
        self.focus_frame(frames[idx].data.id);
    }

    /// Step to a specific 0-based page index in Read / View mode.
    pub fn go_to_read_index(&self, index: usize) {
        let frames = self.scene.with_untracked(|s| s.frames_ordered());
        if frames.is_empty() {
            self.read_mode_index.set(None);
            return;
        }
        let idx = index.min(frames.len() - 1);
        self.read_mode_index.set(Some(idx));
        self.focus_frame(frames[idx].data.id);
    }

    /// Step to the next frame in Read / View mode.
    pub fn next_read_frame(&self) {
        let Some(cur) = self.read_mode_index.get_untracked() else {
            return;
        };
        let len = self.scene.with_untracked(|s| s.frame_count());
        if cur + 1 < len {
            self.go_to_read_index(cur + 1);
        }
    }

    /// Step to the previous frame in Read / View mode.
    pub fn prev_read_frame(&self) {
        let Some(cur) = self.read_mode_index.get_untracked() else {
            return;
        };
        if cur > 0 {
            self.go_to_read_index(cur - 1);
        }
    }

    /// Exit Read / View mode.
    pub fn exit_read_mode(&self) {
        self.read_mode_index.set(None);
    }

    /// Switch the canvas to Draw mode with `Tool::Frame`.
    pub fn activate_frame_tool(&self) {
        self.open.set(false);
        self.menu.set(None);
        self.selected_tool.set(Tool::Frame);
        self.canvas_mode.set(CanvasMode::Draw);
        self.eraser_active.set(false);
    }
}

/// Top-right 4-square (`layout-grid`) button and dropdown panel for managing ordered frames.
#[component]
pub fn FramesPanel(context: FramesContext) -> impl IntoView {
    let open = context.open;
    let close = {
        let context = context.clone();
        move || {
            context.commit_rename();
            context.menu.set(None);
            open.set(false);
        }
    };
    let toggle = {
        let context = context.clone();
        move || {
            let next = !open.get_untracked();
            if !next {
                context.commit_rename();
                context.menu.set(None);
            }
            open.set(next);
        }
    };

    let frames_memo = {
        let scene = context.scene;
        create_memo(move |_| scene.with(|s| s.frames_ordered()))
    };

    view! {
        <div class="relative">
            <IconButton on_click=toggle title="Frames" class=styles::BTN_COLLAPSE>
                {icon::frames_grid()}
            </IconButton>

            {move || {
                if !open.get() {
                    return view! {}.into_view();
                }
                let frames = frames_memo.get();
                let count = frames.len();
                let ctx_read = context.clone();
                let ctx_draw = context.clone();
                let ctx_pdf = context.clone();
                let close_backdrop = close.clone();
                let close_btn = close.clone();

                view! {
                    <>
                        <div
                            class="fixed inset-0 z-40"
                            on:click=move |_| close_backdrop()
                        ></div>
                        <div class=styles::CATALOGUE_WINDOW>
                            <div class="flex items-center justify-between">
                                <div class="flex items-center gap-2">
                                    <span class="text-sm font-medium text-fg">"Frames"</span>
                                    <span class="rounded bg-surface/60 px-1.5 py-0.5 text-[10px] font-mono text-subtle tabular-nums">
                                        {count}
                                    </span>
                                </div>
                                <button
                                    class="text-subtle hover:text-fg transition-colors"
                                    on:click=move |_| close_btn()
                                    title="Close"
                                >
                                    {icon::close()}
                                </button>
                            </div>

                            <div class="mt-2 max-h-[50vh] overflow-y-auto pr-0.5">
                                {if frames.is_empty() {
                                    view! {
                                        <p class="py-6 text-center text-xs text-subtle">
                                            "No frames yet. Use the Frame tool to add pages or artboards."
                                        </p>
                                    }
                                        .into_view()
                                } else {
                                    let total = frames.len();
                                    view! {
                                        <div class="flex flex-col gap-1">
                                            {frames
                                                .into_iter()
                                                .enumerate()
                                                .map(|(idx, frame)| {
                                                    frame_row(idx, total, frame, context.clone())
                                                })
                                                .collect_view()}
                                        </div>
                                    }
                                        .into_view()
                                }}
                            </div>

                            <div class="mt-2 flex gap-1.5">
                                <button
                                    class=format!("{} flex-1", styles::BTN_TEXT)
                                    on:click=move |_| ctx_draw.activate_frame_tool()
                                    title="Draw a new frame on the canvas"
                                >
                                    <span class="mr-1.5">{icon::frame()}</span>
                                    "New frame"
                                </button>
                                <button
                                    class=format!(
                                        "{} flex-1 disabled:opacity-40 disabled:pointer-events-none",
                                        styles::BTN_TEXT
                                    )
                                    disabled=count == 0
                                    on:click=move |_| ctx_pdf.export_all_frames_pdf()
                                    title="Export the whole gallery of frames as a multi-page PDF"
                                >
                                    <span class="mr-1.5">{icon::book_open_small()}</span>
                                    "Export PDF"
                                </button>
                                <button
                                    class=format!(
                                        "{} flex-1 disabled:opacity-40 disabled:pointer-events-none",
                                        styles::BTN_TEXT_ACTIVE
                                    )
                                    disabled=count == 0
                                    on:click=move |_| ctx_read.start_read_mode(0)
                                    title="View / Read Mode — step through frames like pages"
                                >
                                    <span class="mr-1.5">{icon::book_open_small()}</span>
                                    "Read mode"
                                </button>
                            </div>
                        </div>
                    </>
                }
                    .into_view()
            }}
        </div>
    }
}

fn frame_row(
    idx: usize,
    total: usize,
    frame: Frame,
    context: FramesContext,
) -> impl IntoView {
    let id = frame.data.id;
    let display_name = if frame.name.trim().is_empty() {
        format!("Frame {}", idx + 1)
    } else {
        frame.name.clone()
    };
    let size_badge = match frame.preset {
        FramePreset::Freehand => format!(
            "{:.1}×{:.1} cm",
            crate::model::elements::frame::px_to_cm(frame.data.width.abs()),
            crate::model::elements::frame::px_to_cm(frame.data.height.abs())
        ),
        preset => format!(
            "{} · {}",
            preset.label(),
            frame.orientation.label()
        ),
    };

    let renaming_id = context.renaming_id;
    let rename_draft = context.rename_draft;
    let selected_ids = context.selected_ids;
    let is_selected = Signal::derive(move || selected_ids.with(|ids| ids.contains(&id)));

    let ctx_select = context.clone();
    let ctx_focus = context.clone();
    let ctx_rename = context.clone();
    let ctx_commit = context.clone();
    let ctx_key = context.clone();
    let ctx_up = context.clone();
    let ctx_down = context.clone();
    let ctx_menu = context.clone();
    let frame_for_rename = frame.clone();
    let is_first = idx == 0;
    let is_last = idx + 1 >= total;

    let open_context_menu = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        ctx_menu.menu.set(Some(FrameMenu {
            id,
            index: idx,
            x: ev.client_x() as f64,
            y: ev.client_y() as f64,
        }));
    };

    view! {
        <div
            class=move || {
                if is_selected.get() {
                    "flex items-center gap-1.5 w-full rounded-md border border-accent bg-accent/10 p-1.5 text-left transition-colors"
                } else {
                    styles::CATALOGUE_ITEM
                }
            }
            on:contextmenu=open_context_menu
        >
            <span class="flex h-6 w-6 shrink-0 items-center justify-center rounded bg-surface/70 text-[11px] font-mono text-subtle tabular-nums">
                {idx + 1}
            </span>

            <div class="min-w-0 flex-1">
                {move || {
                    if renaming_id.get() == Some(id) {
                        let ctx_blur = ctx_commit.clone();
                        let ctx_keydown = ctx_key.clone();
                        view! {
                            <input
                                type="text"
                                class="w-full rounded border border-accent bg-canvas px-1.5 py-0.5 text-xs text-fg outline-none"
                                prop:value=move || rename_draft.get()
                                autofocus=true
                                on:input=move |event| rename_draft.set(event_target_value(&event))
                                on:blur=move |_| ctx_blur.commit_rename()
                                on:keydown=move |event: web_sys::KeyboardEvent| {
                                    event.stop_propagation();
                                    if event.key() == "Enter" {
                                        event.prevent_default();
                                        ctx_keydown.commit_rename();
                                    } else if event.key() == "Escape" {
                                        event.prevent_default();
                                        renaming_id.set(None);
                                    }
                                }
                            />
                        }
                            .into_view()
                    } else {
                        let name_text = display_name.clone();
                        let badge_text = size_badge.clone();
                        let ctx_click = ctx_select.clone();
                        let ctx_dbl = ctx_rename.clone();
                        let frame_dbl = frame_for_rename.clone();
                        view! {
                            <button
                                class="block w-full text-left truncate"
                                on:click=move |_| ctx_click.select_and_focus(id)
                                on:dblclick=move |event| {
                                    event.stop_propagation();
                                    ctx_dbl.begin_rename(&frame_dbl);
                                }
                                title="Click to select & focus · Double-click to rename · Right-click for options"
                            >
                                <span class="block truncate text-xs font-medium text-fg">
                                    {name_text}
                                </span>
                                <span class="block truncate text-[10px] text-subtle">
                                    {badge_text}
                                </span>
                            </button>
                        }
                            .into_view()
                    }
                }}
            </div>

            <div class="flex items-center gap-0.5 shrink-0">
                <button
                    class="flex h-6 w-6 items-center justify-center rounded text-subtle hover:text-fg hover:bg-surface/60 transition-colors"
                    title="Focus (zoom to fit)"
                    on:click=move |event| {
                        event.stop_propagation();
                        ctx_focus.focus_frame(id);
                    }
                >
                    {icon::focus_small()}
                </button>
                <button
                    class="flex h-6 w-6 items-center justify-center rounded text-subtle hover:text-fg hover:bg-surface/60 transition-colors"
                    title="Rename frame"
                    on:click={
                        let ctx = context.clone();
                        let frame = frame.clone();
                        move |event| {
                            event.stop_propagation();
                            ctx.begin_rename(&frame);
                        }
                    }
                >
                    {icon::pencil_small()}
                </button>
                <div class="flex flex-col rounded border border-border overflow-hidden">
                    <button
                        class="flex items-center justify-center px-1 py-0.5 text-subtle hover:text-fg hover:bg-accent/10 disabled:opacity-30 disabled:pointer-events-none transition-colors"
                        disabled=is_first
                        title="Move frame earlier"
                        on:click=move |event| {
                            event.stop_propagation();
                            ctx_up.move_frame(id, -1);
                        }
                    >
                        {icon::chevron_up()}
                    </button>
                    <button
                        class="flex items-center justify-center px-1 py-0.5 text-subtle hover:text-fg hover:bg-accent/10 disabled:opacity-30 disabled:pointer-events-none transition-colors"
                        disabled=is_last
                        title="Move frame later"
                        on:click=move |event| {
                            event.stop_propagation();
                            ctx_down.move_frame(id, 1);
                        }
                    >
                        {icon::chevron_down()}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Context menu opened by right-clicking a frame row in the Frames gallery (`FramesPanel`).
#[component]
pub fn FramesContextMenu(context: FramesContext) -> impl IntoView {
    let menu = context.menu;
    move || {
        let Some(item) = menu.get() else {
            return view! {}.into_view();
        };
        let size = window_size();
        let left = item.x.min(size.x - 196.0).max(8.0);
        let top = item.y.min(size.y - 220.0).max(8.0);
        let close = move || menu.set(None);

        let ctx_png = context.clone();
        let ctx_svg = context.clone();
        let ctx_pdf = context.clone();
        let ctx_focus = context.clone();
        let ctx_rename = context.clone();
        let ctx_read = context.clone();
        let ctx_delete = context.clone();

        view! {
            <>
                <div
                    class="fixed inset-0 z-[70]"
                    on:click=move |_| close()
                    on:contextmenu=move |ev| {
                        ev.prevent_default();
                        close();
                    }
                ></div>
                <div
                    class="fixed z-[71] w-48 rounded-lg bg-panel/95 backdrop-blur-sm border border-border shadow-xl py-1"
                    style=format!("left:{left}px; top:{top}px")
                    role="menu"
                >
                    <button
                        class=styles::MENU_ITEM
                        on:click=move |_| {
                            close();
                            ctx_png.export_frame(item.id, true);
                        }
                        role="menuitem"
                    >
                        <span>"Export as PNG"</span>
                        <span class="text-subtle">{icon::frame()}</span>
                    </button>
                    <button
                        class=styles::MENU_ITEM
                        on:click=move |_| {
                            close();
                            ctx_svg.export_frame(item.id, false);
                        }
                        role="menuitem"
                    >
                        <span>"Export as SVG"</span>
                        <span class="text-subtle">{icon::frame()}</span>
                    </button>
                    <button
                        class=styles::MENU_ITEM
                        on:click=move |_| {
                            close();
                            ctx_pdf.export_frame_pdf(item.id);
                        }
                        role="menuitem"
                    >
                        <span>"Export as PDF"</span>
                        <span class="text-subtle">{icon::book_open_small()}</span>
                    </button>
                    <div class="h-px bg-border my-1"></div>
                    <button
                        class=styles::MENU_ITEM
                        on:click=move |_| {
                            close();
                            ctx_focus.focus_frame(item.id);
                        }
                        role="menuitem"
                    >
                        <span>"Focus frame"</span>
                        <span class="text-subtle">{icon::focus_small()}</span>
                    </button>
                    <button
                        class=styles::MENU_ITEM
                        on:click=move |_| {
                            close();
                            let frame_opt = ctx_rename.scene.with_untracked(|s| {
                                s.frames_ordered()
                                    .into_iter()
                                    .find(|f| f.data.id == item.id)
                            });
                            if let Some(f) = frame_opt {
                                ctx_rename.begin_rename(&f);
                            }
                        }
                        role="menuitem"
                    >
                        <span>"Rename"</span>
                        <span class="text-subtle">{icon::pencil_small()}</span>
                    </button>
                    <button
                        class=styles::MENU_ITEM
                        on:click=move |_| {
                            close();
                            ctx_read.start_read_mode(item.index);
                        }
                        role="menuitem"
                    >
                        <span>"Read mode from here"</span>
                        <span class="text-subtle">{icon::book_open_small()}</span>
                    </button>
                    <div class="h-px bg-border my-1"></div>
                    <button
                        class=styles::MENU_ITEM
                        on:click=move |_| {
                            close();
                            ctx_delete.delete_frame(item.id);
                        }
                        role="menuitem"
                    >
                        <span class="text-red">"Delete"</span>
                        <span class="text-red">{icon::trash_small()}</span>
                    </button>
                </div>
            </>
        }
        .into_view()
    }
}

/// Full-screen HUD bar for stepping through frames like pages in Read / View mode.
#[component]
pub fn FrameReaderOverlay(context: FramesContext) -> impl IntoView {
    let read_index = context.read_mode_index;

    // Keyboard navigation while Read Mode is active:
    // ArrowRight / ArrowDown / PageDown / Space -> next frame
    // ArrowLeft / ArrowUp / PageUp -> previous frame
    // Escape -> exit Read Mode
    {
        let ctx = context.clone();
        let _ = window_event_listener(leptos::ev::keydown, move |event: leptos::ev::KeyboardEvent| {
            if ctx.read_mode_index.get_untracked().is_none() {
                return;
            }
            match event.key().as_str() {
                "ArrowRight" | "ArrowDown" | "PageDown" | " " => {
                    event.prevent_default();
                    event.stop_propagation();
                    ctx.next_read_frame();
                }
                "ArrowLeft" | "ArrowUp" | "PageUp" => {
                    event.prevent_default();
                    event.stop_propagation();
                    ctx.prev_read_frame();
                }
                "Escape" => {
                    event.prevent_default();
                    event.stop_propagation();
                    ctx.exit_read_mode();
                }
                _ => {}
            }
        });
    }

    // Re-fit the active frame if the window is resized while in Read Mode.
    {
        let ctx = context.clone();
        let _ = window_event_listener(leptos::ev::resize, move |_| {
            if let Some(idx) = ctx.read_mode_index.get_untracked() {
                ctx.go_to_read_index(idx);
            }
        });
    }

    move || {
        let Some(idx) = read_index.get() else {
            return view! {}.into_view();
        };
        let frames = context.scene.with(|s| s.frames_ordered());
        if frames.is_empty() {
            return view! {}.into_view();
        }
        let total = frames.len();
        let safe_idx = idx.min(total - 1);
        let is_first = safe_idx == 0;
        let is_last = safe_idx + 1 >= total;
        let current = &frames[safe_idx];
        let title = if current.name.trim().is_empty() {
            format!("Frame {}", safe_idx + 1)
        } else {
            current.name.clone()
        };
        let preset_label = match current.preset {
            FramePreset::Freehand => String::new(),
            p => format!(" ({})", p.label()),
        };

        let ctx_prev = context.clone();
        let ctx_next = context.clone();
        let ctx_fit = context.clone();
        let ctx_exit = context.clone();
        let current_id = current.data.id;

        view! {
            <div class="fixed bottom-6 inset-x-0 z-50 flex justify-center pointer-events-none select-none">
                <div class=format!(
                    "{} flex items-center gap-2 px-3 py-1.5 pointer-events-auto",
                    styles::PANEL
                )>
                    <span class="flex items-center gap-1.5 text-xs font-medium text-accent pr-1 border-r border-border">
                        {icon::book_open_small()}
                        <span>"Read Mode"</span>
                    </span>

                    <button
                        class=format!(
                            "{} disabled:opacity-30 disabled:pointer-events-none",
                            styles::BTN_GHOST
                        )
                        disabled=is_first
                        on:click=move |_| ctx_prev.prev_read_frame()
                        title="Previous frame (←)"
                    >
                        {icon::chevron_left()}
                    </button>

                    <div class="min-w-36 text-center px-2">
                        <div class="text-xs font-medium text-fg truncate">
                            {format!("{title}{preset_label}")}
                        </div>
                        <div class="text-[10px] font-mono text-subtle tabular-nums">
                            {format!("Page {} of {}", safe_idx + 1, total)}
                        </div>
                    </div>

                    <button
                        class=format!(
                            "{} disabled:opacity-30 disabled:pointer-events-none",
                            styles::BTN_GHOST
                        )
                        disabled=is_last
                        on:click=move |_| ctx_next.next_read_frame()
                        title="Next frame (→)"
                    >
                        {icon::chevron_right()}
                    </button>

                    <button
                        class=styles::BTN_GHOST
                        on:click=move |_| ctx_fit.focus_frame(current_id)
                        title="Zoom to fit current frame"
                    >
                        {icon::focus()}
                    </button>

                    <button
                        class=format!("{} ml-1", styles::BTN_TEXT)
                        on:click=move |_| ctx_exit.exit_read_mode()
                        title="Exit Read Mode (Esc)"
                    >
                        "Exit"
                    </button>
                </div>
            </div>
        }
            .into_view()
    }
}
