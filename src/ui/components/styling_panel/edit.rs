use crate::canvas::CanvasMode;
use crate::edit_ops::{self, Clipboard};
use crate::model::{ElementId, Point, Scene};
use crate::ui::{icon, styles};
use leptos::*;
use std::rc::Rc;

pub fn edit_section(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    clipboard: RwSignal<Clipboard>,
    push_snapshot: Rc<dyn Fn()>,
    canvas_mode: RwSignal<CanvasMode>,
    cursor_world: RwSignal<Point>,
    notifications: crate::ui::notify::Notifications,
) -> impl IntoView {
    let push_dup = push_snapshot.clone();
    let push_paste = push_snapshot.clone();
    view! {
        <div class="shrink-0" class:hidden=move || canvas_mode.get() != CanvasMode::Select>
            <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                "Edit"
            </div>
            <div class="flex gap-1">
                <button
                    class=styles::BTN_LAYER
                    on:click=move |_| {
                        if !selected_ids.get().is_empty() {
                            edit_ops::copy_selection(scene, selected_ids, clipboard);
                            crate::ui::notify::info(notifications, "Copied");
                        }
                    }
                    title="Copy selection (Ctrl+C)"
                >
                    {icon::copy()}
                </button>
                <button
                    class=styles::BTN_LAYER
                    on:click=move |_| {
                        if edit_ops::duplicate_selection(scene, selected_ids, &push_dup) {
                            crate::ui::notify::info(notifications, "Duplicated elements");
                        }
                    }
                    title="Duplicate selection (Ctrl+D)"
                >
                    {icon::duplicate()}
                </button>
                <button
                    class=styles::BTN_LAYER
                    on:click=move |_| {
                        let count_before = scene.with_untracked(|s| s.element_count());
                        edit_ops::paste_clipboard(
                            scene,
                            clipboard,
                            selected_ids,
                            &push_paste,
                            cursor_world,
                        );
                        if scene.with_untracked(|s| s.element_count()) > count_before {
                            crate::ui::notify::info(notifications, "Pasted");
                        }
                    }
                    title="Paste clipboard (Ctrl+V)"
                >
                    {icon::paste()}
                </button>
            </div>
        </div>
    }
}
