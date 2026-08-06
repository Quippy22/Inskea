use crate::canvas::CanvasMode;
use crate::edit_ops;
use crate::model::{Element, ElementId, Point, Scene};
use crate::ui::{icon, styles};
use leptos::*;
use std::rc::Rc;

pub fn edit_section(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    clipboard: RwSignal<Vec<Element>>,
    push_snapshot: Rc<dyn Fn()>,
    canvas_mode: RwSignal<CanvasMode>,
    cursor_world: RwSignal<Point>,
) -> impl IntoView {
    let push_dup = push_snapshot.clone();
    let push_paste = push_snapshot.clone();
    view! {
        <div class:hidden=move || canvas_mode.get() != CanvasMode::Select>
            <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                "Edit"
            </div>
            <div class="flex gap-1">
                <button
                    class=styles::BTN_LAYER
                    on:click=move |_| {
                        edit_ops::copy_selection(scene, selected_ids, clipboard);
                    }
                    title="Copy selection (Ctrl+C)"
                >
                    {icon::copy()}
                </button>
                <button
                    class=styles::BTN_LAYER
                    on:click=move |_| {
                        edit_ops::duplicate_selection(scene, selected_ids, &push_dup);
                    }
                    title="Duplicate selection (Ctrl+D)"
                >
                    {icon::duplicate()}
                </button>
                <button
                    class=styles::BTN_LAYER
                    on:click=move |_| {
                        edit_ops::paste_clipboard(
                            scene,
                            clipboard,
                            selected_ids,
                            &push_paste,
                            cursor_world,
                        );
                    }
                    title="Paste clipboard (Ctrl+V)"
                >
                    {icon::paste()}
                </button>
            </div>
        </div>
    }
}
