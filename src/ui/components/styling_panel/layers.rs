use crate::edit_ops;
use crate::model::{ElementId, Scene};
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
                        let ids = selected_ids.get_untracked();
                        if ids.len() == 1 {
                            edit_ops::mutate_scene(scene, &push_back, |s| s.send_to_back(ids[0]));
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
                            edit_ops::mutate_scene(
                                scene,
                                &push_backward,
                                |s| s.send_backward(ids[0]),
                            );
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
                            edit_ops::mutate_scene(
                                scene,
                                &push_forward,
                                |s| s.bring_forward(ids[0]),
                            );
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
                            edit_ops::mutate_scene(
                                scene,
                                &push_front,
                                |s| s.bring_to_front(ids[0]),
                            );
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
