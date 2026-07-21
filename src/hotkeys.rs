use crate::canvas::CanvasMode;
use crate::model::{Element, ElementId, Offset, Scene};
use crate::ui::dock::Tool;
use crate::ui::file_ops;
use leptos::ev;
use leptos::*;
use std::rc::Rc;
use wasm_bindgen::JsCast;

pub(crate) const MODE_SHORTCUTS: &[(&str, &str)] = &[
    ("s", "Select mode"),
    ("a", "Pan mode"),
    ("d", "Draw mode"),
    ("e", "Erase mode"),
    ("Esc", "Deselect / cancel"),
    ("Delete", "Delete selection"),
];

pub(crate) const TOOL_SHORTCUTS: &[(&str, &str)] = &[
    ("1", "Rectangle"),
    ("2", "Ellipse"),
    ("3", "Line"),
    ("4", "Arrow"),
    ("5", "Text"),
    ("f", "Freehand"),
];

pub(crate) const FILE_SHORTCUTS: &[(&str, &str)] = &[
    ("Ctrl+Z", "Undo"),
    ("Ctrl+Shift+Z", "Redo"),
    ("Ctrl+C", "Copy"),
    ("Ctrl+V", "Paste"),
    ("Ctrl+D", "Duplicate"),
    ("Ctrl+S", "Save"),
    ("Ctrl+Shift+S", "Save As"),
    ("Ctrl+O", "Open"),
    ("Ctrl+N", "New document"),
];

pub struct HotkeysContext {
    pub canvas_mode: RwSignal<CanvasMode>,
    pub selected_tool: RwSignal<Tool>,
    pub scene: RwSignal<Scene>,
    pub selected_ids: RwSignal<Vec<ElementId>>,
    pub clipboard: RwSignal<Vec<Element>>,
    pub saved_path: RwSignal<Option<String>>,
    pub shortcuts_open: RwSignal<bool>,
    pub push_snapshot: Rc<dyn Fn()>,
    pub do_undo: Rc<dyn Fn()>,
    pub do_redo: Rc<dyn Fn()>,
}

fn is_text_input(target: &web_sys::EventTarget) -> bool {
    let node = target.clone().dyn_into::<web_sys::Node>();
    if let Ok(node) = node {
        if let Some(el) = node.dyn_ref::<web_sys::HtmlElement>() {
            let tag = el.tag_name().to_lowercase();
            if tag == "input" || tag == "textarea" {
                return true;
            }
            if el.get_attribute("contenteditable").as_deref() == Some("true") {
                return true;
            }
        }
    }
    false
}

pub fn register_hotkeys(ctx: HotkeysContext) {
    let handle_shortcut = move |key: &str, ctrl: bool, shift: bool| match (ctrl, key) {
        (true, "z") | (true, "Z") => {
            if shift {
                (ctx.do_redo)();
            } else {
                (ctx.do_undo)();
            }
        }
        (false, "s") => ctx.canvas_mode.set(CanvasMode::Select),
        (false, "a") => ctx.canvas_mode.set(CanvasMode::Pan),
        (true, "d") => {
            let ids = ctx.selected_ids.get();
            if !ids.is_empty() {
                (ctx.push_snapshot)();
                let mut new_ids = Vec::new();
                ctx.scene.update(|s| {
                    let elements = s.elements().to_vec();
                    for id in &ids {
                        if let Some(el) = elements.iter().find(|e| e.id() == *id) {
                            let mut clone = el.clone();
                            clone.offset(20.0, 20.0);
                            clone.data_mut().id = 0;
                            s.add_element(clone);
                            new_ids.push(s.next_id - 1);
                        }
                    }
                });
                ctx.selected_ids.set(new_ids);
            }
        }
        (true, "n") | (true, "N") => {
            file_ops::file_new(ctx.scene, ctx.saved_path, ctx.selected_ids)
        }
        (true, "o") | (true, "O") => {
            file_ops::file_open(ctx.scene, ctx.saved_path, ctx.selected_ids)
        }
        (true, "s") | (true, "S") => {
            if shift {
                file_ops::file_save_as(ctx.scene, ctx.saved_path);
            } else {
                file_ops::file_save(ctx.scene, ctx.saved_path);
            }
        }
        (false, "?") => ctx.shortcuts_open.update(|v| *v = !*v),
        (false, "e") => ctx.canvas_mode.set(CanvasMode::Erase),
        (false, "1") => {
            ctx.selected_tool.set(Tool::Rectangle);
            ctx.canvas_mode.set(CanvasMode::Draw);
        }
        (false, "2") => {
            ctx.selected_tool.set(Tool::Ellipse);
            ctx.canvas_mode.set(CanvasMode::Draw);
        }
        (false, "3") => {
            ctx.selected_tool.set(Tool::Line);
            ctx.canvas_mode.set(CanvasMode::Draw);
        }
        (false, "4") => {
            ctx.selected_tool.set(Tool::Arrow);
            ctx.canvas_mode.set(CanvasMode::Draw);
        }
        (false, "5") => {
            ctx.selected_tool.set(Tool::Text);
            ctx.canvas_mode.set(CanvasMode::Draw);
        }
        (false, "f") => {
            ctx.selected_tool.set(Tool::Freehand);
            ctx.canvas_mode.set(CanvasMode::Draw);
        }
        (false, "d") => ctx.canvas_mode.set(CanvasMode::Draw),
        (false, "Escape") => {}
        (true, "c") => {
            let ids = ctx.selected_ids.get();
            let els = ctx.scene.with(|s| {
                s.elements()
                    .iter()
                    .filter(|e| ids.contains(&e.id()))
                    .cloned()
                    .collect::<Vec<_>>()
            });
            ctx.clipboard.set(els);
            web_sys::console::log_1(&"Copied".into());
        }
        (true, "v") => {
            let els = ctx.clipboard.get();
            if !els.is_empty() {
                (ctx.push_snapshot)();
                let mut new_ids = Vec::new();
                ctx.scene.update(|s| {
                    for el in els {
                        let mut el = el;
                        el.data_mut().id = 0;
                        s.add_element(el);
                        new_ids.push(s.next_id - 1);
                    }
                });
                ctx.selected_ids.set(new_ids);
            }
        }
        (false, "Delete") | (false, "Backspace") => {
            let ids = ctx.selected_ids.get();
            if !ids.is_empty() {
                (ctx.push_snapshot)();
                ctx.scene.update(|s| {
                    for id in &ids {
                        s.remove_by_id(*id);
                    }
                });
                ctx.selected_ids.set(Vec::new());
            }
        }
        _ => {}
    };

    let _ = window_event_listener(ev::keydown, move |ev: ev::KeyboardEvent| {
        if let Some(target) = ev.target() {
            if is_text_input(&target) {
                return;
            }
        }
        let key = ev.key();
        let ctrl = ev.ctrl_key();
        let shift = ev.shift_key();
        handle_shortcut(&key, ctrl, shift);
    });
}

#[component]
pub fn ShortcutsModal(shortcuts_open: RwSignal<bool>) -> impl IntoView {
    move || {
        if shortcuts_open.get() {
            view! {
                <>
                    <div
                        class="fixed inset-0 z-40"
                        on:click=move |_| shortcuts_open.set(false)
                    ></div>
                    <div class="fixed inset-0 z-50 grid place-items-center pointer-events-none">
                        <div class="pointer-events-auto rounded-lg bg-panel/95 backdrop-blur-sm border border-border shadow-xl py-5 px-7 max-w-2xl w-full mx-4 max-h-[85vh] overflow-y-auto">
                            <button
                                class="absolute top-3 right-3 text-subtle hover:text-fg transition-colors"
                                on:click=move |_| shortcuts_open.set(false)
                                title="Close"
                            >
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    class="w-5 h-5"
                                >
                                    <line x1="18" y1="6" x2="6" y2="18" />
                                    <line x1="6" y1="6" x2="18" y2="18" />
                                </svg>
                            </button>
                            <h2 class="text-lg font-semibold mb-4">"Keyboard Shortcuts"</h2>
                            <div class="flex gap-10">
                                <div>
                                    <h3 class="text-xs font-semibold text-subtle uppercase tracking-wider mb-2">
                                        "Shapes"
                                    </h3>
                                    <table class="text-sm">
                                        <tbody>
                                            {TOOL_SHORTCUTS
                                                .iter()
                                                .map(|(key, desc)| {
                                                    view! {
                                                        <tr>
                                                            <td class="py-1 pr-5 font-mono text-accent whitespace-nowrap">
                                                                {*key}
                                                            </td>
                                                            <td class="py-1 text-fg whitespace-nowrap">{*desc}</td>
                                                        </tr>
                                                    }
                                                })
                                                .collect::<Vec<_>>()}
                                        </tbody>
                                    </table>
                                </div>
                                <div>
                                    <h3 class="text-xs font-semibold text-subtle uppercase tracking-wider mb-2">
                                        "Modes"
                                    </h3>
                                    <table class="text-sm">
                                        <tbody>
                                            {MODE_SHORTCUTS
                                                .iter()
                                                .map(|(key, desc)| {
                                                    view! {
                                                        <tr>
                                                            <td class="py-1 pr-5 font-mono text-accent whitespace-nowrap">
                                                                {*key}
                                                            </td>
                                                            <td class="py-1 text-fg whitespace-nowrap">{*desc}</td>
                                                        </tr>
                                                    }
                                                })
                                                .collect::<Vec<_>>()}
                                        </tbody>
                                    </table>
                                </div>
                                <div>
                                    <h3 class="text-xs font-semibold text-subtle uppercase tracking-wider mb-2">
                                        "File & Edit"
                                    </h3>
                                    <table class="text-sm">
                                        <tbody>
                                            {FILE_SHORTCUTS
                                                .iter()
                                                .map(|(key, desc)| {
                                                    view! {
                                                        <tr>
                                                            <td class="py-1 pr-5 font-mono text-accent whitespace-nowrap">
                                                                {*key}
                                                            </td>
                                                            <td class="py-1 text-fg whitespace-nowrap">{*desc}</td>
                                                        </tr>
                                                    }
                                                })
                                                .collect::<Vec<_>>()}
                                        </tbody>
                                    </table>
                                </div>
                            </div>
                        </div>
                    </div>
                </>
            }.into_view()
        } else {
            view! {}.into_view()
        }
    }
}
