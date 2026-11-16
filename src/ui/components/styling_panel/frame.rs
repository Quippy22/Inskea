use crate::model::{FrameOrientation, FramePreset, FrameStyle};
use crate::ui::{icon, styles};
use leptos::*;
use std::rc::Rc;

#[allow(clippy::too_many_arguments)]
pub fn frame_section(
    is_frame: Signal<bool>,
    has_single_selected_frame: Signal<bool>,
    local_frame_style: RwSignal<FrameStyle>,
    frame_dimensions_label: Signal<String>,
    set_frame_preset: Rc<dyn Fn(FramePreset)>,
    set_frame_orientation: Rc<dyn Fn(FrameOrientation)>,
    on_focus_frame: Rc<dyn Fn()>,
) -> impl IntoView {
    view! {
        <div class="flex flex-col gap-3" class:hidden=move || !is_frame.get()>
            <div>
                <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                    "Frame size"
                </div>
                <div class="grid grid-cols-4 gap-1">
                    {FramePreset::ALL
                        .iter()
                        .map(|&preset| {
                            let label = preset.label();
                            let set_preset = set_frame_preset.clone();
                            view! {
                                <button
                                    class=move || {
                                        if local_frame_style.get().preset == preset {
                                            "px-1.5 py-1 rounded border border-accent bg-accent/10 text-[11px] font-medium text-accent transition-colors text-center"
                                        } else {
                                            "px-1.5 py-1 rounded border border-border text-[11px] text-subtle hover:text-fg hover:bg-surface/50 transition-colors text-center"
                                        }
                                    }
                                    title=format!("{label} frame size")
                                    on:click=move |_| set_preset(preset)
                                >
                                    {label}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </div>

            <Show when=move || local_frame_style.get().preset != FramePreset::Freehand>
                <div>
                    <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                        "Orientation"
                    </div>
                    <div class="grid grid-cols-2 gap-1">
                        {[FrameOrientation::Portrait, FrameOrientation::Landscape]
                            .into_iter()
                            .map(|orient| {
                                let label = orient.label();
                                let set_orient = set_frame_orientation.clone();
                                view! {
                                    <button
                                        class=move || {
                                            if local_frame_style.get().orientation == orient {
                                                styles::BTN_TEXT_ACTIVE
                                            } else {
                                                styles::BTN_TEXT
                                            }
                                        }
                                        on:click=move |_| set_orient(orient)
                                    >
                                        {label}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                </div>
            </Show>

            <Show when=move || !frame_dimensions_label.get().is_empty()>
                <div class="text-[10px] leading-snug text-subtle px-0.5">
                    {move || frame_dimensions_label.get()}
                </div>
            </Show>

            <Show when=move || has_single_selected_frame.get()>
                <button
                    class=format!("{} w-full", styles::BTN_TEXT)
                    on:click={
                        let f = on_focus_frame.clone();
                        move |_| f()
                    }
                    title="Zoom and pan viewport to fit this frame"
                >
                    <span class="mr-1.5">{icon::focus_small()}</span>
                    "Focus frame"
                </button>
            </Show>
        </div>
    }
}
