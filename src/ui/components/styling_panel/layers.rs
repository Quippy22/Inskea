use crate::model::{ElementId, Scene};
use crate::ui::icon;
use crate::ui::styles;
use leptos::*;

pub fn layers_section(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
) -> impl IntoView {
    view! {
        <div>
            <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                "Layers"
            </div>
            <div class="flex gap-1">
                <button
                    class=styles::BTN_LAYER
                    on:click=move |_| {
                        let ids = selected_ids.get_untracked();
                        if ids.len() == 1 {
                            scene.update(|s| s.send_to_back(ids[0]));
                        }
                    }
                    title="Send to back"
                >
                    {icon::send_to_back()}
                </button>
                <button
                    class=styles::BTN_LAYER
                    on:click=move |_| {
                        let ids = selected_ids.get_untracked();
                        if ids.len() == 1 {
                            scene.update(|s| s.send_backward(ids[0]));
                        }
                    }
                    title="Send backward"
                >
                    {icon::send_backward()}
                </button>
                <button
                    class=styles::BTN_LAYER
                    on:click=move |_| {
                        let ids = selected_ids.get_untracked();
                        if ids.len() == 1 {
                            scene.update(|s| s.bring_forward(ids[0]));
                        }
                    }
                    title="Bring forward"
                >
                    {icon::bring_forward()}
                </button>
                <button
                    class=styles::BTN_LAYER
                    on:click=move |_| {
                        let ids = selected_ids.get_untracked();
                        if ids.len() == 1 {
                            scene.update(|s| s.bring_to_front(ids[0]));
                        }
                    }
                    title="Bring to front"
                >
                    {icon::bring_to_front()}
                </button>
            </div>
        </div>
    }
}
