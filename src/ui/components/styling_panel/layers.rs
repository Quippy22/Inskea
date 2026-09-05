use crate::edit_ops;
use crate::model::{ElementId, LayerOrder, Scene};
use crate::ui::icon;
use crate::ui::styles;
use leptos::*;
use std::rc::Rc;

pub fn layers_section(
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    push_snapshot: Rc<dyn Fn()>,
) -> impl IntoView {
    let push_back = push_snapshot.clone();
    let push_backward = push_snapshot.clone();
    let push_forward = push_snapshot.clone();
    let push_front = push_snapshot;
    view! {
        <div>
            <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                "Layers"
            </div>
            <div class="flex gap-1">
                <button
                    class=styles::BTN_LAYER
                    on:click=move |_| {
                        edit_ops::reorder_selection(scene, selected_ids, LayerOrder::Back, &push_back);
                    }
                    title="Send to back"
                >
                    {icon::send_to_back()}
                </button>
                <button
                    class=styles::BTN_LAYER
                    on:click=move |_| {
                        edit_ops::reorder_selection(
                            scene,
                            selected_ids,
                            LayerOrder::Backward,
                            &push_backward,
                        );
                    }
                    title="Send backward"
                >
                    {icon::send_backward()}
                </button>
                <button
                    class=styles::BTN_LAYER
                    on:click=move |_| {
                        edit_ops::reorder_selection(
                            scene,
                            selected_ids,
                            LayerOrder::Forward,
                            &push_forward,
                        );
                    }
                    title="Bring forward"
                >
                    {icon::bring_forward()}
                </button>
                <button
                    class=styles::BTN_LAYER
                    on:click=move |_| {
                        edit_ops::reorder_selection(scene, selected_ids, LayerOrder::Front, &push_front);
                    }
                    title="Bring to front"
                >
                    {icon::bring_to_front()}
                </button>
            </div>
        </div>
    }
}
