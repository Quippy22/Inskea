use crate::canvas::CanvasMode;
use crate::edit_ops;
use crate::model::{ElementId, Scene};
use crate::ui::styles;
use leptos::*;
use std::rc::Rc;

pub fn group_section(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    canvas_mode: RwSignal<CanvasMode>,
    push_snapshot: Rc<dyn Fn()>,
) -> impl IntoView {
    let in_select = move || canvas_mode.get() == CanvasMode::Select;
    let can_group = Signal::derive(move || in_select() && selected_ids.get().len() >= 2);
    let is_group = Signal::derive(move || {
        if !in_select() {
            return false;
        }
        let ids = selected_ids.get();
        scene.with(|s| s.selection_is_group(&ids))
    });
    let member_count = Signal::derive(move || {
        let ids = selected_ids.get();
        scene.with(|s| {
            s.group_containing(*ids.first().unwrap_or(&0))
                .map_or(0, |g| g.members.len())
        })
    });
    let push_group = push_snapshot.clone();
    let push_ungroup = push_snapshot.clone();
    view! {
        <div class:hidden=move || !in_select()>
            <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                "Group"
            </div>
            <div class="flex items-center gap-1">
                <button
                    class=move || {
                        let base = if is_group.get() {
                            format!(
                                "{} border-accent text-accent bg-accent/10",
                                styles::BTN_TEXT
                            )
                        } else {
                            styles::BTN_TEXT.to_string()
                        };
                        base
                    }
                    class:opacity-40=move || !can_group.get()
                    class:cursor-not-allowed=move || !can_group.get()
                    on:click=move |_| {
                        edit_ops::group_selection(scene, selected_ids, &push_group);
                    }
                    title="Group selection (Ctrl+G)"
                >
                    "Group"
                </button>
                <button
                    class=styles::BTN_TEXT
                    class:opacity-40=move || !is_group.get()
                    class:cursor-not-allowed=move || !is_group.get()
                    on:click=move |_| {
                        edit_ops::ungroup_selection(scene, selected_ids, &push_ungroup);
                    }
                    title="Ungroup selection (Ctrl+Shift+G)"
                >
                    "Ungroup"
                </button>
                <span class=move || {
                    if is_group.get() {
                        let n = member_count.get();
                        format!("{n} elements")
                    } else {
                        String::new()
                    }
                }>
                </span>
            </div>
        </div>
    }
}
