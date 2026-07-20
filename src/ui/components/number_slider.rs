use std::rc::Rc;
use crate::ui::classes;
use leptos::*;
use leptos::ev;
use web_sys::MouseEvent;

#[component]
pub fn NumberSlider(
    value: RwSignal<f64>,
    min: f64,
    max: f64,
    increment: f64,
    label: &'static str,
    #[prop(optional)]
    on_change: Option<Rc<dyn Fn(f64)>>,
) -> impl IntoView {
    let snap = move |v: f64| -> f64 {
        let snapped = (v / increment).round() * increment;
        snapped.clamp(min, max)
    };

    let local = create_rw_signal(value.get());

    let commit: Rc<dyn Fn()> = Rc::new(move || {
        let v = local.get();
        value.set(v);
        if let Some(ref f) = on_change {
            f(v);
        }
    });

    let inc = {
        let commit = Rc::clone(&commit);
        move |_| {
            local.update(|v| *v = snap(*v + increment));
            commit();
        }
    };

    let dec = {
        let commit = Rc::clone(&commit);
        move |_| {
            local.update(|v| *v = snap(*v - increment));
            commit();
        }
    };

    create_effect(move |_| {
        let v = value.get();
        leptos::untrack(move || local.set(v));
    });

    let dragging = create_rw_signal(false);
    let track_ref = create_node_ref::<html::Div>();

    let start_drag = move |ev: MouseEvent| {
        dragging.set(true);
        update_from_event(&local, &snap, min, max, &track_ref, &ev);
    };

    let _ = window_event_listener(ev::mousemove, {
        let local = local;
        move |ev| {
            if dragging.get() {
                update_from_event(&local, &snap, min, max, &track_ref, &ev);
            }
        }
    });

    {
        let commit = Rc::clone(&commit);
        let _ = window_event_listener(ev::mouseup, move |_| {
            if dragging.get() {
                dragging.set(false);
                commit();
            }
        });
    }

    let pct = move || {
        let v = local.get();
        if (max - min).abs() < f64::EPSILON {
            50.0
        } else {
            ((v - min) / (max - min)) * 100.0
        }
    };

    view! {
        <div class="flex flex-col gap-1">
            <span class=classes::SETTINGS_LABEL>{label}</span>
            <div class=classes::SLIDER_ROW>
                <div
                    ref=track_ref
                    class=classes::SLIDER_TRACK
                    on:mousedown=start_drag
                >
                    <div
                        class=classes::SLIDER_FILL
                        style:width=move || format!("{}%", pct())
                    />
                    <div
                        class=classes::SLIDER_THUMB
                        style:left=move || format!("calc({}% - 0.125rem)", pct())
                        class:scale-125=move || dragging.get()
                    />
                </div>

                <span class=classes::SLIDER_READOUT>
                    {move || format!("{}", local.get().round() as i64)}
                </span>

                <div class=classes::SLIDER_STEPPER>
                    <button
                        class=classes::SLIDER_STEP_BTN
                        on:click=inc
                        title="Increase"
                    >
                        <svg width="8" height="5" viewBox="0 0 8 5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M1 4 L4 1 L7 4"/>
                        </svg>
                    </button>
                    <div class="border-t border-border" />
                    <button
                        class=classes::SLIDER_STEP_BTN
                        on:click=dec
                        title="Decrease"
                    >
                        <svg width="8" height="5" viewBox="0 0 8 5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M1 1 L4 4 L7 1"/>
                        </svg>
                    </button>
                </div>
            </div>
        </div>
    }
}

fn update_from_event(
    value: &RwSignal<f64>,
    snap: &dyn Fn(f64) -> f64,
    min: f64,
    max: f64,
    track_ref: &NodeRef<html::Div>,
    ev: &MouseEvent,
) {
    if let Some(el) = track_ref.get() {
        let rect = el.get_bounding_client_rect();
        let x = ev.client_x() as f64 - rect.left();
        let w = rect.width();
        if w > 0.0 {
            let fraction = (x / w).clamp(0.0, 1.0);
            let raw = min + fraction * (max - min);
            value.set(snap(raw));
        }
    }
}
