use crate::canvas::Viewport;
use crate::model::{Point, Scene};
use crate::ui::styles;
use crate::util::window_size;
use leptos::ev;
use leptos::*;

/// Bottom-right canvas HUD: counts for the whole document, plus a jump-to
/// action while the viewport is empty.
#[component]
pub fn CanvasStatus(scene: RwSignal<Scene>, viewport: RwSignal<Viewport>) -> impl IntoView {
    let screen_size = create_rw_signal(window_size());
    let _ = window_event_listener(ev::resize, move |_| screen_size.set(window_size()));

    let counts = Signal::derive(move || scene.with(|s| (s.element_count(), s.group_count())));

    let nothing_in_view = Signal::derive(move || {
        scene.with(|scene| {
            !scene.is_empty()
                && !scene.has_elements_in_view(viewport.get().visible_rect(screen_size.get()))
        })
    });

    // The document is never empty here, so the handler always has a target.
    let go_to_elements = move |_| {
        let center = scene.with_untracked(|scene| {
            let vp = viewport.get_untracked();
            scene.nearest_element_center(Point::new(vp.offset_x, vp.offset_y))
        });
        if let Some(center) = center {
            viewport.update(|vp| {
                vp.offset_x = center.x;
                vp.offset_y = center.y;
            });
        }
    };

    view! {
        <div class=styles::CONTAINER_HUD>
            {move || {
                nothing_in_view.get().then(|| {
                    view! {
                        <button
                            class=styles::BTN_HUD
                            on:click=go_to_elements
                            title="Center the nearest element"
                        >
                            "To elements"
                        </button>
                    }
                })
            }}
            {move || {
                let (elements, groups) = counts.get();
                if elements == 0 && groups == 0 {
                    return None;
                }
                let mut parts = Vec::new();
                if elements > 0 {
                    parts.push(format!(
                        "{elements} element{}",
                        if elements == 1 { "" } else { "s" }
                    ));
                }
                if groups > 0 {
                    parts.push(format!(
                        "{groups} group{}",
                        if groups == 1 { "" } else { "s" }
                    ));
                }
                Some(view! { <div class=styles::HUD_COUNTER>{parts.join(" · ")}</div> })
            }}
        </div>
    }
}
