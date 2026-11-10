use crate::canvas::settings::CanvasSettings;
use crate::canvas::Viewport;
use crate::catalogue as snippet_store;
use crate::catalogue::{PendingSnippet, Snippet};
use crate::model::{ElementId, Scene};
use crate::ui::components::IconButton;
use crate::ui::notify::{self, Notifications};
use crate::ui::{icon, styles};
use leptos::*;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CatalogueTab {
    Local,
    Global,
}

/// A context-menu request raised by right-clicking a catalogue row.
#[derive(Clone, Debug)]
pub struct CatalogueMenu {
    pub id: String,
    pub starred: bool,
    pub global: bool,
    pub x: f64,
    pub y: f64,
}

/// Shared state for the catalogue panel, the save-snippet dialog, and the
/// selection capture triggered from the canvas or styling panel.
#[derive(Clone)]
pub struct CatalogueContext {
    pub open: RwSignal<bool>,
    pub tab: RwSignal<CatalogueTab>,
    pub scene: RwSignal<Scene>,
    pub selected_ids: RwSignal<Vec<ElementId>>,
    pub viewport: RwSignal<Viewport>,
    pub push_snapshot: Rc<dyn Fn()>,
    pub notifications: Notifications,
    pub settings: RwSignal<CanvasSettings>,
    pub pending: RwSignal<Option<PendingSnippet>>,
    pub pending_name: RwSignal<String>,
    pub globals: RwSignal<Vec<Snippet>>,
    pub loading: RwSignal<bool>,
    pub menu: RwSignal<Option<CatalogueMenu>>,
    /// Client position of the canvas right-click menu for the selection.
    pub selection_menu: RwSignal<Option<(f64, f64)>>,
}

impl CatalogueContext {
    /// Capture the selection and show the save-as-snippet dialog.
    pub fn begin_save(&self) {
        let pending = self.scene.with_untracked(|scene| {
            snippet_store::capture_selection(scene, &self.selected_ids.get_untracked())
        });
        let Some(pending) = pending else {
            notify::error(self.notifications, "Select something to save as a snippet.");
            return;
        };
        let next = self
            .scene
            .with_untracked(|scene| scene.snippets().len() + 1);
        self.pending_name.set(format!("Snippet {next}"));
        self.pending.set(Some(pending));
    }

    pub fn cancel_save(&self) {
        self.pending.set(None);
    }

    fn take_named(&self) -> Option<(PendingSnippet, String)> {
        let pending = self.pending.get_untracked()?;
        let name = self.pending_name.get_untracked().trim().to_string();
        if name.is_empty() {
            notify::error(self.notifications, "Give the snippet a name.");
            return None;
        }
        self.pending.set(None);
        Some((pending, name))
    }

    pub fn save_local(&self) {
        let Some((pending, name)) = self.take_named() else {
            return;
        };
        let id = self.scene.with_untracked(|scene| {
            snippet_store::unique_id(&name, scene.snippets().iter().map(|s| s.id.as_str()))
        });
        let snippet = pending.into_snippet(id, name);
        self.scene.update(|scene| scene.add_snippet(snippet));
        self.tab.set(CatalogueTab::Local);
        self.open.set(true);
        notify::info(self.notifications, "Saved to this drawing's catalogue.");
    }

    pub fn save_global(&self) {
        let Some((pending, name)) = self.take_named() else {
            return;
        };
        let id = self.globals.with_untracked(|globals| {
            snippet_store::unique_id(&name, globals.iter().map(|s| s.id.as_str()))
        });
        let snippet = pending.into_snippet(id, name);
        self.tab.set(CatalogueTab::Global);
        self.open.set(true);
        let context = self.clone();
        let stored = snippet.clone();
        spawn_local(async move {
            match snippet_store::save_global(&stored).await {
                Ok(()) => {
                    context.globals.update(|globals| {
                        globals.retain(|existing| existing.id != stored.id);
                        globals.push(stored);
                    });
                    notify::info(context.notifications, "Saved to the global catalogue.");
                }
                Err(e) => {
                    notify::error(
                        context.notifications,
                        format!("Could not save the snippet: {e}"),
                    );
                    context.refresh_globals();
                }
            }
        });
    }

    pub fn refresh_globals(&self) {
        self.loading.set(true);
        let context = self.clone();
        spawn_local(async move {
            match snippet_store::load_globals().await {
                Ok(globals) => {
                    context.globals.set(globals);
                }
                Err(e) => {
                    notify::error(
                        context.notifications,
                        format!("Could not load the global catalogue: {e}"),
                    );
                }
            }
            context.loading.set(false);
        });
    }

    /// Place a snippet in the middle of the visible canvas. Rows are clicked
    /// over the panel, so the last canvas cursor position would be stale.
    pub fn apply(&self, snippet: &Snippet) {
        let center = self
            .viewport
            .get_untracked()
            .visible_rect(crate::util::window_size())
            .center();
        snippet_store::apply(
            snippet,
            self.scene,
            self.selected_ids,
            &self.push_snapshot,
            center,
        );
        self.open.set(false);
    }

    pub fn toggle_star(&self, global: bool, id: &str) {
        if !global {
            self.scene.update(|scene| scene.toggle_snippet_star(id));
            return;
        }
        let Some(mut snippet) = self
            .globals
            .with_untracked(|globals| globals.iter().find(|s| s.id == id).cloned())
        else {
            return;
        };
        snippet.starred = !snippet.starred;
        self.globals.update(|globals| {
            if let Some(existing) = globals.iter_mut().find(|s| s.id == id) {
                existing.starred = snippet.starred;
            }
        });
        let context = self.clone();
        let stored = snippet;
        spawn_local(async move {
            if let Err(e) = snippet_store::save_global(&stored).await {
                notify::error(
                    context.notifications,
                    format!("Could not update the snippet: {e}"),
                );
                context.refresh_globals();
            }
        });
    }

    pub fn delete(&self, global: bool, id: &str) {
        if !global {
            self.scene.update(|scene| scene.remove_snippet(id));
            notify::info(self.notifications, "Deleted the local snippet.");
            return;
        }
        self.globals.update(|globals| {
            globals.retain(|snippet| snippet.id != id);
        });
        let context = self.clone();
        let id = id.to_string();
        spawn_local(async move {
            match snippet_store::delete_global(&id).await {
                Ok(()) => notify::info(context.notifications, "Deleted the global snippet."),
                Err(e) => {
                    notify::error(
                        context.notifications,
                        format!("Could not delete the snippet: {e}"),
                    );
                    context.refresh_globals();
                }
            }
        });
    }
}

/// The catalogue button and its dropdown panel. Rendered inside the top-right
/// tools island next to the settings gear.
#[component]
pub fn Catalogue(context: CatalogueContext) -> impl IntoView {
    let open = context.open;
    let tab = context.tab;
    let close = move || open.set(false);
    let toggle = move || open.update(|value| *value = !*value);

    view! {
        <div class="relative">
            <IconButton on_click=toggle title="Catalogue" class=styles::BTN_COLLAPSE>
                {icon::catalogue()}
            </IconButton>

            {move || {
                if !open.get() {
                    return view! {}.into_view();
                }
                let local_context = context.clone();
                let global_context = context.clone();
                let context = context.clone();
                view! {
                    <>
                        <div class="fixed inset-0 z-40" on:click=move |_| close()></div>
                        <div class=styles::CATALOGUE_WINDOW>
                            <div class="flex items-center justify-between">
                                <span class="text-sm font-medium text-fg">"Catalogue"</span>
                                <button
                                    class="text-subtle hover:text-fg transition-colors"
                                    on:click=move |_| close()
                                    title="Close"
                                >
                                    {icon::close()}
                                </button>
                            </div>

                            <div class="mt-2 flex gap-1 rounded-md bg-surface/40 p-0.5">
                                <button
                                    class=move || {
                                        if tab.get() == CatalogueTab::Local {
                                            styles::CATALOGUE_TAB_ACTIVE
                                        } else {
                                            styles::CATALOGUE_TAB_INACTIVE
                                        }
                                    }
                                    on:click=move |_| tab.set(CatalogueTab::Local)
                                >
                                    "Local"
                                </button>
                                <button
                                    class=move || {
                                        if tab.get() == CatalogueTab::Global {
                                            styles::CATALOGUE_TAB_ACTIVE
                                        } else {
                                            styles::CATALOGUE_TAB_INACTIVE
                                        }
                                    }
                                    on:click=move |_| tab.set(CatalogueTab::Global)
                                >
                                    "Global"
                                </button>
                            </div>

                            <div class="mt-2 max-h-[50vh] overflow-y-auto pr-0.5">
                                {move || local_list(local_context.clone())}
                                {move || global_list(global_context.clone())}
                            </div>

                            <button
                                class=format!("{} w-full mt-2", styles::BTN_TEXT)
                                on:click={
                                    let context = context.clone();
                                    move |_| context.begin_save()
                                }
                            >
                                <span class="mr-1.5">{icon::save_snippet()}</span>
                                "Save selection"
                            </button>
                            <p class="mt-1.5 text-[10px] leading-snug text-subtle">
                                {move || {
                                    if tab.get() == CatalogueTab::Local {
                                        "Local snippets travel with this drawing."
                                    } else {
                                        "Global snippets live in the Inskea catalogue folder."
                                    }
                                }}
                            </p>
                        </div>
                    </>
                }
                .into_view()
            }}
        </div>
    }
}

fn local_list(context: CatalogueContext) -> View {
    if context.tab.get() != CatalogueTab::Local {
        return view! {}.into_view();
    }
    let snippets = context.scene.with(|scene| scene.snippets().to_vec());
    if snippets.is_empty() {
        return empty_state("Save a selection to reuse it in this drawing.").into_view();
    }
    view! {
        <div class="flex flex-col gap-1">
            <For each=move || snippets.clone() key=|snippet| snippet.id.clone() let:snippet>
                {snippet_row(snippet, false, context.clone())}
            </For>
        </div>
    }
    .into_view()
}

fn global_list(context: CatalogueContext) -> View {
    if context.tab.get() != CatalogueTab::Global {
        return view! {}.into_view();
    }
    if context.loading.get() {
        return empty_state("Loading the global catalogue…").into_view();
    }
    let snippets = context.globals.get();
    if snippets.is_empty() {
        return empty_state("Save a selection to the global catalogue to use it across drawings.")
            .into_view();
    }
    view! {
        <div class="flex flex-col gap-1">
            <For each=move || snippets.clone() key=|snippet| snippet.id.clone() let:snippet>
                {snippet_row(snippet, true, context.clone())}
            </For>
        </div>
    }
    .into_view()
}

fn snippet_row(snippet: Snippet, global: bool, context: CatalogueContext) -> impl IntoView {
    let id = snippet.id.clone();
    let name = snippet.name.clone();
    let starred = snippet.starred;
    let count = snippet.elements.len();
    let preview = snippet_preview(&snippet, context.settings);
    let title = name.clone();

    let apply = {
        let context = context.clone();
        let snippet = snippet.clone();
        move |_| context.apply(&snippet)
    };
    let open_menu = {
        let context = context.clone();
        move |ev: web_sys::MouseEvent| {
            ev.prevent_default();
            ev.stop_propagation();
            context.menu.set(Some(CatalogueMenu {
                id: id.clone(),
                starred,
                global,
                x: ev.client_x() as f64,
                y: ev.client_y() as f64,
            }));
        }
    };

    view! {
        <div
            class=styles::CATALOGUE_ITEM
            on:contextmenu=open_menu
            title=move || format!("{} — click to place", title)
        >
            <button
                class="flex min-w-0 flex-1 items-center gap-2 text-left"
                on:click=apply
                title="Place snippet at the cursor"
            >
                <span class="h-10 w-10 shrink-0 overflow-hidden rounded border border-border bg-canvas">
                    {preview}
                </span>
                <span class="min-w-0 flex-1">
                    <span class="block truncate text-xs text-fg">{name.clone()}</span>
                    <span class="block text-[10px] text-subtle">
                        {count}
                        {if count == 1 { " element" } else { " elements" }}
                    </span>
                </span>
            </button>
            <Show when=move || starred>
                <span class="shrink-0 text-accent" title="Starred">
                    {icon::star_small()}
                </span>
            </Show>
        </div>
    }
}

fn snippet_preview(snippet: &Snippet, settings: RwSignal<CanvasSettings>) -> View {
    use crate::model::Render;

    let Some(bounds) = snippet_store::bounds(snippet) else {
        return view! {}.into_view();
    };
    let pad = 6.0;
    let x = bounds.x - pad;
    let y = bounds.y - pad;
    let w = (bounds.w + pad * 2.0).max(1.0);
    let h = (bounds.h + pad * 2.0).max(1.0);
    let body = snippet
        .elements
        .iter()
        .map(|element| element.render(1.0))
        .collect_view();

    view! {
        <svg
            class="h-10 w-10"
            viewBox=format!("{x} {y} {w} {h}")
            preserveAspectRatio="xMidYMid meet"
            aria-hidden="true"
        >
            <rect
                x=x
                y=y
                width=w
                height=h
                fill=move || settings.get().theme.canvas()
            />
            {body}
        </svg>
    }
    .into_view()
}

fn empty_state(message: &'static str) -> View {
    view! {
        <div class="px-2 py-6 text-center text-xs text-subtle">{message}</div>
    }
    .into_view()
}

/// The small menu opened by right-clicking a catalogue row.
#[component]
pub fn CatalogueContextMenu(context: CatalogueContext) -> impl IntoView {
    let menu = context.menu;
    move || {
        let Some(item) = menu.get() else {
            return view! {}.into_view();
        };
        let size = crate::util::window_size();
        let left = item.x.min(size.x - 168.0).max(8.0);
        let top = item.y.min(size.y - 88.0).max(8.0);
        let star_label = if item.starred { "Unstar" } else { "Star" };
        let close = move || menu.set(None);

        let star = {
            let context = context.clone();
            let id = item.id.clone();
            let global = item.global;
            move |_| {
                context.toggle_star(global, &id);
                close();
            }
        };
        let delete = {
            let context = context.clone();
            let id = item.id.clone();
            let global = item.global;
            move |_| {
                context.delete(global, &id);
                close();
            }
        };

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
                    class="fixed z-[71] w-40 rounded-lg bg-panel/95 backdrop-blur-sm border border-border shadow-xl py-1"
                    style=format!("left:{left}px; top:{top}px")
                >
                    <button class=styles::MENU_ITEM on:click=star>
                        <span>{star_label}</span>
                        <span class="text-accent">{icon::star_small()}</span>
                    </button>
                    <div class="h-px bg-border my-1"></div>
                    <button class=styles::MENU_ITEM on:click=delete>
                        <span class="text-red">"Delete"</span>
                        <span class="text-red">{icon::trash_small()}</span>
                    </button>
                </div>
            </>
        }
        .into_view()
    }
}

/// The menu opened by right-clicking a shape on the canvas.
#[component]
pub fn SelectionContextMenu(context: CatalogueContext) -> impl IntoView {
    let menu = context.selection_menu;
    move || {
        let Some((x, y)) = menu.get() else {
            return view! {}.into_view();
        };
        let size = crate::util::window_size();
        let left = x.min(size.x - 184.0).max(8.0);
        let top = y.min(size.y - 48.0).max(8.0);
        let close = move || menu.set(None);
        let save = {
            let context = context.clone();
            move |_| {
                close();
                context.begin_save();
            }
        };

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
                    class="fixed z-[71] w-44 rounded-lg bg-panel/95 backdrop-blur-sm border border-border shadow-xl py-1"
                    style=format!("left:{left}px; top:{top}px")
                    role="menu"
                >
                    <button class=styles::MENU_ITEM on:click=save role="menuitem">
                        <span>"Save as snippet"</span>
                        <span class="text-subtle">{icon::save_snippet()}</span>
                    </button>
                </div>
            </>
        }
        .into_view()
    }
}

/// The popup that asks whether a captured selection becomes a local or global
/// snippet.
#[component]
pub fn SaveSnippetDialog(
    pending: RwSignal<Option<PendingSnippet>>,
    name: RwSignal<String>,
    on_local: Callback<()>,
    on_global: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    view! {
        <Show when=move || pending.get().is_some()>
            <div class="fixed inset-0 z-[60] bg-black/40"></div>
            <div class="fixed inset-0 z-[61] grid place-items-center">
                <div
                    class=format!("{} w-80 p-5", styles::PANEL)
                    role="dialog"
                    aria-modal="true"
                    aria-label="Save snippet"
                >
                    <h2 class="text-sm font-medium text-fg">"Save as snippet"</h2>
                    <p class="mt-1 text-xs text-subtle">
                        "Local snippets stay with this drawing. Global snippets are available in every drawing."
                    </p>
                    <input
                        type="text"
                        class="mt-4 w-full rounded-md border border-border bg-surface/40 px-2.5 py-1.5 text-sm text-fg outline-none focus:border-accent"
                        prop:value=move || name.get()
                        on:input=move |ev| name.set(event_target_value(&ev))
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" {
                                ev.prevent_default();
                                on_local.call(());
                            }
                        }
                        autofocus=true
                    />
                    <div class="mt-5 flex justify-end gap-2">
                        <button class=styles::BTN_TEXT on:click=move |_| on_cancel.call(())>
                            "Cancel"
                        </button>
                        <button class=styles::BTN_TEXT on:click=move |_| on_global.call(())>
                            "Global"
                        </button>
                        <button
                            class=styles::BTN_TEXT_ACTIVE
                            on:click=move |_| on_local.call(())
                        >
                            "Local"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
