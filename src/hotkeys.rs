use crate::canvas::CanvasMode;
use crate::document::DocContext;
use crate::edit_ops::{self, Clipboard};
use crate::model::{Alignment, Distribution, Point};
use crate::ui::dock::Tool;
use crate::ui::{file_ops, icon};
use leptos::ev;
use leptos::*;
use std::rc::Rc;
use wasm_bindgen::JsCast;

pub(crate) const MODE_SHORTCUTS: &[(&str, &str)] = &[
    ("s", "Select mode"),
    ("a", "Pan mode"),
    ("d", "Draw mode"),
    ("e", "Erase mode"),
    ("Esc", "Cancel / pan mode"),
    ("Delete", "Delete selection"),
    ("Shift + drag", "Smart guides"),
    ("Alt + drag", "Snap to grid"),
];

pub(crate) const TOOL_SHORTCUTS: &[(&str, &str)] = &[
    ("1", "Rectangle"),
    ("2", "Ellipse"),
    ("3", "Line"),
    ("4", "Arrow"),
    ("5", "Text"),
    ("f", "Freehand"),
];

pub(crate) const EDIT_SHORTCUTS: &[(&str, &str)] = &[
    ("Ctrl+C", "Copy"),
    ("Ctrl+V", "Paste"),
    ("Ctrl+D", "Duplicate"),
    ("Ctrl+G", "Group"),
    ("Ctrl+Shift+G", "Ungroup"),
    ("Ctrl+Alt+←/→/↑/↓", "Align selection"),
    ("Ctrl+Alt+Shift+←/↑", "Distribute selection"),
];

pub(crate) const FILE_SHORTCUTS: &[(&str, &str)] = &[
    ("Ctrl+Z", "Undo"),
    ("Ctrl+Shift+Z", "Redo"),
    ("Ctrl+S", "Save"),
    ("Ctrl+Shift+S", "Save As"),
    ("Ctrl+O", "Open"),
    ("Ctrl+N", "New document"),
];

pub struct HotkeysContext {
    pub canvas_mode: RwSignal<CanvasMode>,
    pub selected_tool: RwSignal<Tool>,
    pub eraser_active: RwSignal<bool>,
    pub doc: DocContext,
    pub clipboard: RwSignal<Clipboard>,
    pub cursor_world: RwSignal<Point>,
    pub shortcuts_open: RwSignal<bool>,
    pub push_snapshot: Rc<dyn Fn()>,
    pub do_undo: Rc<dyn Fn()>,
    pub do_redo: Rc<dyn Fn()>,
}

const TAB_TOOLS: [Tool; 5] = [
    Tool::Rectangle,
    Tool::Ellipse,
    Tool::Line,
    Tool::Arrow,
    Tool::Text,
];

fn next_tab_tool(current: Tool) -> Option<Tool> {
    let index = TAB_TOOLS.iter().position(|tool| *tool == current)?;
    Some(TAB_TOOLS[(index + 1) % TAB_TOOLS.len()])
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
    let handle_shortcut = move |key: &str, ctrl: bool, shift: bool, alt: bool| match (ctrl, key) {
        (true, "z") | (true, "Z") => {
            if shift {
                (ctx.do_redo)();
            } else {
                (ctx.do_undo)();
            }
        }
        (false, "s") => ctx.canvas_mode.set(CanvasMode::Select),
        (false, "a") => ctx.canvas_mode.set(CanvasMode::Pan),
        (true, "d") | (true, "D") => {
            edit_ops::duplicate_selection(ctx.doc.scene, ctx.doc.selected_ids, &ctx.push_snapshot)
        }
        (true, "g") | (true, "G") => {
            if shift {
                edit_ops::ungroup_selection(
                    ctx.doc.scene,
                    ctx.doc.selected_ids,
                    &ctx.push_snapshot,
                );
            } else {
                edit_ops::group_selection(ctx.doc.scene, ctx.doc.selected_ids, &ctx.push_snapshot);
            }
        }
        (true, "ArrowLeft") if alt && shift => edit_ops::distribute_selection(
            ctx.doc.scene,
            ctx.doc.selected_ids,
            Distribution::Horizontal,
            &ctx.push_snapshot,
        ),
        (true, "ArrowUp") if alt && shift => edit_ops::distribute_selection(
            ctx.doc.scene,
            ctx.doc.selected_ids,
            Distribution::Vertical,
            &ctx.push_snapshot,
        ),
        (true, "ArrowLeft") if alt => edit_ops::align_selection(
            ctx.doc.scene,
            ctx.doc.selected_ids,
            Alignment::Left,
            &ctx.push_snapshot,
        ),
        (true, "ArrowRight") if alt => edit_ops::align_selection(
            ctx.doc.scene,
            ctx.doc.selected_ids,
            Alignment::Right,
            &ctx.push_snapshot,
        ),
        (true, "ArrowUp") if alt => edit_ops::align_selection(
            ctx.doc.scene,
            ctx.doc.selected_ids,
            Alignment::Top,
            &ctx.push_snapshot,
        ),
        (true, "ArrowDown") if alt => edit_ops::align_selection(
            ctx.doc.scene,
            ctx.doc.selected_ids,
            Alignment::Bottom,
            &ctx.push_snapshot,
        ),
        (true, "n") | (true, "N") => file_ops::file_new(ctx.doc.clone()),
        (true, "o") | (true, "O") => file_ops::file_open(ctx.doc.clone()),
        (true, "s") | (true, "S") => {
            if shift {
                file_ops::file_save_as(ctx.doc.clone());
            } else {
                file_ops::file_save(ctx.doc.clone());
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
        // Escape is handled by the canvas, which owns the interaction state.
        (false, "Escape") => {}
        (true, "c") | (true, "C") => {
            edit_ops::copy_selection(ctx.doc.scene, ctx.doc.selected_ids, ctx.clipboard);
            web_sys::console::log_1(&"Copied".into());
        }
        (true, "v") | (true, "V") => edit_ops::paste_clipboard(
            ctx.doc.scene,
            ctx.clipboard,
            ctx.doc.selected_ids,
            &ctx.push_snapshot,
            ctx.cursor_world,
        ),
        (false, "Delete") | (false, "Backspace") => {
            let ids = ctx.doc.selected_ids.get();
            if !ids.is_empty() {
                (ctx.push_snapshot)();
                ctx.doc.scene.update(|s| {
                    for id in &ids {
                        s.remove_by_id(*id);
                    }
                });
                ctx.doc.selected_ids.set(Vec::new());
            }
        }
        _ => {}
    };

    let _ = window_event_listener(ev::keydown, move |ev: ev::KeyboardEvent| {
        if ev.key() == "Tab" {
            ev.prevent_default();
            if let Some(target) = ev.target() {
                if is_text_input(&target) {
                    return;
                }
            }
            if ctx.canvas_mode.get() == CanvasMode::Draw && !ctx.eraser_active.get() {
                if let Some(tool) = next_tab_tool(ctx.selected_tool.get()) {
                    ctx.selected_tool.set(tool);
                }
            }
            return;
        }
        if let Some(target) = ev.target() {
            if is_text_input(&target) {
                return;
            }
        }
        let key = ev.key();
        let ctrl = ev.ctrl_key();
        let shift = ev.shift_key();
        handle_shortcut(&key, ctrl, shift, ev.alt_key());
    });
}

#[cfg(test)]
mod tests {
    use super::{next_tab_tool, Tool};

    #[test]
    fn tab_cycles_only_through_standard_drawing_tools() {
        assert!(matches!(
            next_tab_tool(Tool::Rectangle),
            Some(Tool::Ellipse)
        ));
        assert!(matches!(next_tab_tool(Tool::Text), Some(Tool::Rectangle)));
        assert!(next_tab_tool(Tool::Freehand).is_none());
    }
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
                        <div class="pointer-events-auto rounded-lg bg-panel/95 backdrop-blur-sm border border-border shadow-xl py-5 px-7 max-w-4xl w-full mx-4 max-h-[85vh] overflow-y-auto">
                            <button
                                class="absolute top-3 right-3 text-subtle hover:text-fg transition-colors"
                                on:click=move |_| shortcuts_open.set(false)
                                title="Close"
                            >
                                {icon::close()}
                            </button>
                            <h2 class="text-lg font-semibold mb-4">"Keyboard Shortcuts"</h2>
                            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-x-8 gap-y-6">
                                <div class="min-w-0">
                                    <h3 class="text-xs font-semibold text-subtle uppercase tracking-wider mb-2">
                                        "Shapes"
                                    </h3>
                                    <table class="text-sm w-full table-fixed">
                                        <tbody>
                                            {TOOL_SHORTCUTS
                                                .iter()
                                                .map(|(key, desc)| {
                                                    view! {
                                                        <tr>
                                                            <td class="py-1 pr-5 font-mono text-accent whitespace-normal break-words align-top">
                                                                {*key}
                                                            </td>
                                                            <td class="py-1 text-fg whitespace-normal break-words align-top">{*desc}</td>
                                                        </tr>
                                                    }
                                                })
                                                .collect::<Vec<_>>()}
                                        </tbody>
                                    </table>
                                </div>
                                <div class="min-w-0">
                                    <h3 class="text-xs font-semibold text-subtle uppercase tracking-wider mb-2">
                                        "Modes"
                                    </h3>
                                    <table class="text-sm w-full table-fixed">
                                        <tbody>
                                            {MODE_SHORTCUTS
                                                .iter()
                                                .map(|(key, desc)| {
                                                    view! {
                                                        <tr>
                                                            <td class="py-1 pr-5 font-mono text-accent whitespace-normal break-words align-top">
                                                                {*key}
                                                            </td>
                                                            <td class="py-1 text-fg whitespace-normal break-words align-top">{*desc}</td>
                                                        </tr>
                                                    }
                                                })
                                                .collect::<Vec<_>>()}
                                        </tbody>
                                    </table>
                                </div>
                                <div class="min-w-0">
                                    <h3 class="text-xs font-semibold text-subtle uppercase tracking-wider mb-2">
                                        "File"
                                    </h3>
                                    <table class="text-sm w-full table-fixed">
                                        <tbody>
                                            {FILE_SHORTCUTS
                                                .iter()
                                                .map(|(key, desc)| {
                                                    view! {
                                                        <tr>
                                                            <td class="py-1 pr-5 font-mono text-accent whitespace-normal break-words align-top">
                                                                {*key}
                                                            </td>
                                                            <td class="py-1 text-fg whitespace-normal break-words align-top">{*desc}</td>
                                                        </tr>
                                                    }
                                                })
                                                .collect::<Vec<_>>()}
                                        </tbody>
                                    </table>
                                </div>
                                <div class="min-w-0">
                                    <h3 class="text-xs font-semibold text-subtle uppercase tracking-wider mb-2">
                                        "Edit"
                                    </h3>
                                    <table class="text-sm w-full table-fixed">
                                        <tbody>
                                            {EDIT_SHORTCUTS
                                                .iter()
                                                .map(|(key, desc)| {
                                                    view! {
                                                        <tr>
                                                            <td class="py-1 pr-5 font-mono text-accent whitespace-normal break-words align-top">
                                                                {*key}
                                                            </td>
                                                            <td class="py-1 text-fg whitespace-normal break-words align-top">{*desc}</td>
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
