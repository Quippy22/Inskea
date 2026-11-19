use crate::canvas::CanvasMode;
use crate::edit_ops;
use crate::model::{
    can_align_selection, selection_item_count, Alignment, Distribution, Element, ElementId, Scene,
};
use crate::ui::{icon, styles};
use leptos::*;
use std::rc::Rc;

pub fn alignment_section(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    canvas_mode: RwSignal<CanvasMode>,
    push_snapshot: Rc<dyn Fn()>,
    notifications: crate::ui::notify::Notifications,
) -> impl IntoView {
    let in_select = move || canvas_mode.get() == CanvasMode::Select;
    let can_align = Signal::derive(move || {
        in_select() && scene.with(|s| can_align_selection(s, &selected_ids.get()))
    });
    let can_distribute = Signal::derive(move || {
        in_select() && scene.with(|s| selection_item_count(s, &selected_ids.get()) >= 3)
    });

    let is_single_frame = Signal::derive(move || {
        let ids = selected_ids.get();
        if ids.len() == 1 {
            scene.with(|s| matches!(s.element_by_id(ids[0]), Some(Element::Frame(_))))
        } else {
            false
        }
    });

    let align_left = {
        let push_snapshot = push_snapshot.clone();
        move || {
            if edit_ops::align_selection(scene, selected_ids, Alignment::Left, &push_snapshot) {
                crate::ui::notify::info(notifications, "Aligned left");
            }
        }
    };
    let align_center = {
        let push_snapshot = push_snapshot.clone();
        move || {
            if edit_ops::align_selection(
                scene,
                selected_ids,
                Alignment::HorizontalCenter,
                &push_snapshot,
            ) {
                crate::ui::notify::info(notifications, "Aligned center");
            }
        }
    };
    let align_right = {
        let push_snapshot = push_snapshot.clone();
        move || {
            if edit_ops::align_selection(scene, selected_ids, Alignment::Right, &push_snapshot) {
                crate::ui::notify::info(notifications, "Aligned right");
            }
        }
    };
    let align_top = {
        let push_snapshot = push_snapshot.clone();
        move || {
            if edit_ops::align_selection(scene, selected_ids, Alignment::Top, &push_snapshot) {
                crate::ui::notify::info(notifications, "Aligned top");
            }
        }
    };
    let align_middle = {
        let push_snapshot = push_snapshot.clone();
        move || {
            if edit_ops::align_selection(
                scene,
                selected_ids,
                Alignment::VerticalCenter,
                &push_snapshot,
            ) {
                crate::ui::notify::info(notifications, "Aligned middle");
            }
        }
    };
    let align_bottom = {
        let push_snapshot = push_snapshot.clone();
        move || {
            if edit_ops::align_selection(scene, selected_ids, Alignment::Bottom, &push_snapshot) {
                crate::ui::notify::info(notifications, "Aligned bottom");
            }
        }
    };
    let distribute_horizontal = {
        let push_snapshot = push_snapshot.clone();
        move || {
            if edit_ops::distribute_selection(
                scene,
                selected_ids,
                Distribution::Horizontal,
                &push_snapshot,
            ) {
                crate::ui::notify::info(notifications, "Distributed horizontally");
            }
        }
    };
    let distribute_vertical = move || {
        if edit_ops::distribute_selection(scene, selected_ids, Distribution::Vertical, &push_snapshot) {
            crate::ui::notify::info(notifications, "Distributed vertically");
        }
    };

    view! {
        <div class:hidden=move || !in_select()>
            <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                {move || if is_single_frame.get() { "Align frame contents" } else { "Element alignment" }}
            </div>
            <div class="grid grid-cols-3 gap-1">
                <button
                    class=styles::BTN_TEXT
                    class:opacity-40=move || !can_align.get()
                    class:cursor-not-allowed=move || !can_align.get()
                    on:click=move |_| align_left()
                    title=move || if is_single_frame.get() { "Align contents left in frame" } else { "Align left edges" }
                    aria-label=move || if is_single_frame.get() { "Align contents left in frame" } else { "Align left edges" }
                >
                    {icon::align_left()}
                </button>
                <button
                    class=styles::BTN_TEXT
                    class:opacity-40=move || !can_align.get()
                    class:cursor-not-allowed=move || !can_align.get()
                    on:click=move |_| align_center()
                    title=move || if is_single_frame.get() { "Center contents horizontally in frame" } else { "Align horizontal centers" }
                    aria-label=move || if is_single_frame.get() { "Center contents horizontally in frame" } else { "Align horizontal centers" }
                >
                    {icon::align_center_h()}
                </button>
                <button
                    class=styles::BTN_TEXT
                    class:opacity-40=move || !can_align.get()
                    class:cursor-not-allowed=move || !can_align.get()
                    on:click=move |_| align_right()
                    title=move || if is_single_frame.get() { "Align contents right in frame" } else { "Align right edges" }
                    aria-label=move || if is_single_frame.get() { "Align contents right in frame" } else { "Align right edges" }
                >
                    {icon::align_right()}
                </button>
                <button
                    class=styles::BTN_TEXT
                    class:opacity-40=move || !can_align.get()
                    class:cursor-not-allowed=move || !can_align.get()
                    on:click=move |_| align_top()
                    title=move || if is_single_frame.get() { "Align contents top in frame" } else { "Align top edges" }
                    aria-label=move || if is_single_frame.get() { "Align contents top in frame" } else { "Align top edges" }
                >
                    {icon::align_top()}
                </button>
                <button
                    class=styles::BTN_TEXT
                    class:opacity-40=move || !can_align.get()
                    class:cursor-not-allowed=move || !can_align.get()
                    on:click=move |_| align_middle()
                    title=move || if is_single_frame.get() { "Center contents vertically in frame" } else { "Align vertical centers" }
                    aria-label=move || if is_single_frame.get() { "Center contents vertically in frame" } else { "Align vertical centers" }
                >
                    {icon::align_center_v()}
                </button>
                <button
                    class=styles::BTN_TEXT
                    class:opacity-40=move || !can_align.get()
                    class:cursor-not-allowed=move || !can_align.get()
                    on:click=move |_| align_bottom()
                    title=move || if is_single_frame.get() { "Align contents bottom in frame" } else { "Align bottom edges" }
                    aria-label=move || if is_single_frame.get() { "Align contents bottom in frame" } else { "Align bottom edges" }
                >
                    {icon::align_bottom()}
                </button>
            </div>
            <div class="grid grid-cols-2 gap-1 mt-1" class:hidden=move || is_single_frame.get()>
                <button
                    class=styles::BTN_TEXT
                    class:opacity-40=move || !can_distribute.get()
                    class:cursor-not-allowed=move || !can_distribute.get()
                    on:click=move |_| distribute_horizontal()
                    title="Distribute horizontally"
                    aria-label="Distribute horizontally"
                >
                    {icon::distribute_horizontal()}
                </button>
                <button
                    class=styles::BTN_TEXT
                    class:opacity-40=move || !can_distribute.get()
                    class:cursor-not-allowed=move || !can_distribute.get()
                    on:click=move |_| distribute_vertical()
                    title="Distribute vertically"
                    aria-label="Distribute vertically"
                >
                    {icon::distribute_vertical()}
                </button>
            </div>
        </div>
    }
}
