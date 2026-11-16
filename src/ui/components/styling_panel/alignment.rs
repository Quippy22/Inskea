use crate::canvas::CanvasMode;
use crate::edit_ops;
use crate::model::{
    can_align_selection, find_enclosing_frame, selection_item_count, AlignTarget, Alignment,
    Distribution, ElementId, Scene,
};
use crate::ui::{icon, styles};
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
        in_select() && scene.with(|s| can_align_selection(s, &selected_ids.get()))
    });
    let can_distribute = Signal::derive(move || {
        in_select() && scene.with(|s| selection_item_count(s, &selected_ids.get()) >= 3)
    });

    let enclosing_frame_name = Signal::derive(move || {
        scene.with(|s| {
            find_enclosing_frame(s, &selected_ids.get()).map(|f| f.header_label())
        })
    });

    let has_multiple_items = Signal::derive(move || {
        scene.with(|s| selection_item_count(s, &selected_ids.get()) >= 2)
    });

    let align_to_frame = create_rw_signal(true);
    let align_as_group = create_rw_signal(true);

    let get_target = move || {
        if enclosing_frame_name.get().is_some()
            && (align_to_frame.get() || !has_multiple_items.get())
        {
            if align_as_group.get() {
                AlignTarget::Frame
            } else {
                AlignTarget::FrameEach
            }
        } else {
            AlignTarget::Selection
        }
    };

    let align_left = {
        let push_snapshot = push_snapshot.clone();
        move || {
            edit_ops::align_selection_to(
                scene,
                selected_ids,
                Alignment::Left,
                get_target(),
                &push_snapshot,
            )
        }
    };
    let align_center = {
        let push_snapshot = push_snapshot.clone();
        move || {
            edit_ops::align_selection_to(
                scene,
                selected_ids,
                Alignment::HorizontalCenter,
                get_target(),
                &push_snapshot,
            )
        }
    };
    let align_right = {
        let push_snapshot = push_snapshot.clone();
        move || {
            edit_ops::align_selection_to(
                scene,
                selected_ids,
                Alignment::Right,
                get_target(),
                &push_snapshot,
            )
        }
    };
    let align_top = {
        let push_snapshot = push_snapshot.clone();
        move || {
            edit_ops::align_selection_to(
                scene,
                selected_ids,
                Alignment::Top,
                get_target(),
                &push_snapshot,
            )
        }
    };
    let align_middle = {
        let push_snapshot = push_snapshot.clone();
        move || {
            edit_ops::align_selection_to(
                scene,
                selected_ids,
                Alignment::VerticalCenter,
                get_target(),
                &push_snapshot,
            )
        }
    };
    let align_bottom = {
        let push_snapshot = push_snapshot.clone();
        move || {
            edit_ops::align_selection_to(
                scene,
                selected_ids,
                Alignment::Bottom,
                get_target(),
                &push_snapshot,
            )
        }
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

    let is_frame_mode = move || {
        enclosing_frame_name.get().is_some()
            && (align_to_frame.get() || !has_multiple_items.get())
    };

    view! {
        <div class:hidden=move || !in_select()>
            <div class="flex items-center justify-between mb-1.5">
                <span class=styles::SETTINGS_LABEL>"Element alignment"</span>
                <Show when=move || enclosing_frame_name.get().is_some()>
                    <div class="flex items-center gap-1 text-[11px]">
                        <button
                            class=move || {
                                if is_frame_mode() {
                                    styles::SEG_BTN_ACTIVE
                                } else {
                                    styles::SEG_BTN_INACTIVE
                                }
                            }
                            class="px-2 py-0.5 rounded border border-border transition-colors truncate max-w-[110px]"
                            on:click=move |_| align_to_frame.set(true)
                            title="Align relative to frame"
                        >
                            {move || enclosing_frame_name.get().unwrap_or_else(|| "Frame".to_string())}
                        </button>
                        <Show when=move || has_multiple_items.get()>
                            <button
                                class=move || {
                                    if !is_frame_mode() {
                                        styles::SEG_BTN_ACTIVE
                                    } else {
                                        styles::SEG_BTN_INACTIVE
                                    }
                                }
                                class="px-2 py-0.5 rounded border border-border transition-colors"
                                on:click=move |_| align_to_frame.set(false)
                                title="Align items relative to each other"
                            >
                                "Selection"
                            </button>
                        </Show>
                    </div>
                </Show>
            </div>
            <Show when=move || is_frame_mode() && has_multiple_items.get()>
                <div class="flex items-center justify-between mb-1.5 text-[11px] text-subtle px-0.5">
                    <span>"Relative mode"</span>
                    <button
                        class="px-1.5 py-0.5 rounded text-[11px] border border-border transition-colors hover:text-fg"
                        class:bg-accent=move || align_as_group.get()
                        class:text-white=move || align_as_group.get()
                        class:bg-surface=move || !align_as_group.get()
                        on:click=move |_| align_as_group.update(|v| *v = !*v)
                        title="Toggle between aligning selection as a single block vs aligning each item individually"
                    >
                        {move || if align_as_group.get() { "As Block" } else { "Each Item" }}
                    </button>
                </div>
            </Show>
            <div class="grid grid-cols-3 gap-1">
                <button
                    class=styles::BTN_TEXT
                    class:opacity-40=move || !can_align.get()
                    class:cursor-not-allowed=move || !can_align.get()
                    on:click=move |_| align_left()
                    title=move || if is_frame_mode() { "Align left inside frame" } else { "Align left edges" }
                    aria-label=move || if is_frame_mode() { "Align left inside frame" } else { "Align left edges" }
                >
                    {icon::align_left()}
                </button>
                <button
                    class=styles::BTN_TEXT
                    class:opacity-40=move || !can_align.get()
                    class:cursor-not-allowed=move || !can_align.get()
                    on:click=move |_| align_center()
                    title=move || if is_frame_mode() { "Align horizontal center in frame" } else { "Align horizontal centers" }
                    aria-label=move || if is_frame_mode() { "Align horizontal center in frame" } else { "Align horizontal centers" }
                >
                    {icon::align_center_h()}
                </button>
                <button
                    class=styles::BTN_TEXT
                    class:opacity-40=move || !can_align.get()
                    class:cursor-not-allowed=move || !can_align.get()
                    on:click=move |_| align_right()
                    title=move || if is_frame_mode() { "Align right inside frame" } else { "Align right edges" }
                    aria-label=move || if is_frame_mode() { "Align right inside frame" } else { "Align right edges" }
                >
                    {icon::align_right()}
                </button>
                <button
                    class=styles::BTN_TEXT
                    class:opacity-40=move || !can_align.get()
                    class:cursor-not-allowed=move || !can_align.get()
                    on:click=move |_| align_top()
                    title=move || if is_frame_mode() { "Align top inside frame" } else { "Align top edges" }
                    aria-label=move || if is_frame_mode() { "Align top inside frame" } else { "Align top edges" }
                >
                    {icon::align_top()}
                </button>
                <button
                    class=styles::BTN_TEXT
                    class:opacity-40=move || !can_align.get()
                    class:cursor-not-allowed=move || !can_align.get()
                    on:click=move |_| align_middle()
                    title=move || if is_frame_mode() { "Align vertical center in frame" } else { "Align vertical centers" }
                    aria-label=move || if is_frame_mode() { "Align vertical center in frame" } else { "Align vertical centers" }
                >
                    {icon::align_center_v()}
                </button>
                <button
                    class=styles::BTN_TEXT
                    class:opacity-40=move || !can_align.get()
                    class:cursor-not-allowed=move || !can_align.get()
                    on:click=move |_| align_bottom()
                    title=move || if is_frame_mode() { "Align bottom inside frame" } else { "Align bottom edges" }
                    aria-label=move || if is_frame_mode() { "Align bottom inside frame" } else { "Align bottom edges" }
                >
                    {icon::align_bottom()}
                </button>
            </div>
            <div class="grid grid-cols-2 gap-1 mt-1">
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
