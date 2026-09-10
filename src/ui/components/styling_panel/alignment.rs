use crate::canvas::CanvasMode;
use crate::edit_ops;
use crate::model::{selection_item_count, Alignment, Distribution, ElementId, Scene};
use crate::ui::styles;
use leptos::*;
use std::rc::Rc;

pub fn alignment_section(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    canvas_mode: RwSignal<CanvasMode>,
    push_snapshot: Rc<dyn Fn()>,
) -> impl IntoView {
    let in_select = move || canvas_mode.get() == CanvasMode::Select;
    let can_align = Signal::derive(move || {
        in_select() && scene.with(|s| selection_item_count(s, &selected_ids.get()) >= 2)
    });
    let can_distribute = Signal::derive(move || {
        in_select() && scene.with(|s| selection_item_count(s, &selected_ids.get()) >= 3)
    });

    let align_left = {
        let push_snapshot = push_snapshot.clone();
        move || edit_ops::align_selection(scene, selected_ids, Alignment::Left, &push_snapshot)
    };
    let align_center = {
        let push_snapshot = push_snapshot.clone();
        move || {
            edit_ops::align_selection(
                scene,
                selected_ids,
                Alignment::HorizontalCenter,
                &push_snapshot,
            )
        }
    };
    let align_right = {
        let push_snapshot = push_snapshot.clone();
        move || edit_ops::align_selection(scene, selected_ids, Alignment::Right, &push_snapshot)
    };
    let align_top = {
        let push_snapshot = push_snapshot.clone();
        move || edit_ops::align_selection(scene, selected_ids, Alignment::Top, &push_snapshot)
    };
    let align_middle = {
        let push_snapshot = push_snapshot.clone();
        move || {
            edit_ops::align_selection(
                scene,
                selected_ids,
                Alignment::VerticalCenter,
                &push_snapshot,
            )
        }
    };
    let align_bottom = {
        let push_snapshot = push_snapshot.clone();
        move || edit_ops::align_selection(scene, selected_ids, Alignment::Bottom, &push_snapshot)
    };
    let distribute_horizontal = {
        let push_snapshot = push_snapshot.clone();
        move || {
            edit_ops::distribute_selection(
                scene,
                selected_ids,
                Distribution::Horizontal,
                &push_snapshot,
            )
        }
    };
    let distribute_vertical = move || {
        edit_ops::distribute_selection(scene, selected_ids, Distribution::Vertical, &push_snapshot)
    };

    view! {
        <div class:hidden=move || !in_select()>
            <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                "Element alignment"
            </div>
            <div class="grid grid-cols-3 gap-1">
                <button class=styles::BTN_TEXT class:opacity-40=move || !can_align.get() class:cursor-not-allowed=move || !can_align.get() on:click=move |_| align_left() title="Align left edges">"Left"</button>
                <button class=styles::BTN_TEXT class:opacity-40=move || !can_align.get() class:cursor-not-allowed=move || !can_align.get() on:click=move |_| align_center() title="Align horizontal centers">"Center"</button>
                <button class=styles::BTN_TEXT class:opacity-40=move || !can_align.get() class:cursor-not-allowed=move || !can_align.get() on:click=move |_| align_right() title="Align right edges">"Right"</button>
                <button class=styles::BTN_TEXT class:opacity-40=move || !can_align.get() class:cursor-not-allowed=move || !can_align.get() on:click=move |_| align_top() title="Align top edges">"Top"</button>
                <button class=styles::BTN_TEXT class:opacity-40=move || !can_align.get() class:cursor-not-allowed=move || !can_align.get() on:click=move |_| align_middle() title="Align vertical centers">"Middle"</button>
                <button class=styles::BTN_TEXT class:opacity-40=move || !can_align.get() class:cursor-not-allowed=move || !can_align.get() on:click=move |_| align_bottom() title="Align bottom edges">"Bottom"</button>
            </div>
            <div class="flex gap-1 mt-1">
                <button class=styles::BTN_TEXT class:opacity-40=move || !can_distribute.get() class:cursor-not-allowed=move || !can_distribute.get() on:click=move |_| distribute_horizontal() title="Distribute horizontally">"Distribute H"</button>
                <button class=styles::BTN_TEXT class:opacity-40=move || !can_distribute.get() class:cursor-not-allowed=move || !can_distribute.get() on:click=move |_| distribute_vertical() title="Distribute vertically">"Distribute V"</button>
            </div>
        </div>
    }
}
