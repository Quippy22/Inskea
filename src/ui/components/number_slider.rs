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
) -> impl IntoView {
    let snap = move |v: f64| -> f64 {
        let snapped = (v / increment).round() * increment;
        snapped.clamp(min, max)
    };

    let inc = move |_| value.update(|v| *v = snap(*v + increment));
    let dec = move |_| value.update(|v| *v = snap(*v - increment));

    // ── Drag state ─────────────────────────────────────────────────────────
    let dragging = create_rw_signal(false);
    let track_ref = create_node_ref::<html::Div>();

    let start_drag = move |ev: MouseEvent| {
        dragging.set(true);
        update_from_event(&value, &snap, min, max, &track_ref, &ev);
    };

    // Attach global listeners when dragging starts
    let _ = window_event_listener(ev::mousemove, move |ev| {
        if dragging.get() {
            update_from_event(&value, &snap, min, max, &track_ref, &ev);
        }
    });

    let _ = window_event_listener(ev::mouseup, move |_| {
        dragging.set(false);
    });

    let pct = move || {
        let v = value.get();
        if (max - min).abs() < f64::EPSILON {
            50.0
        } else {
            ((v - min) / (max - min)) * 100.0
        }
    };

    view! {
        <div class=classes::SLIDER_ROW>
            <span class=classes::SETTINGS_LABEL>{label}</span>

            // ── Drag track ──────────────────────────────────────────────────
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

            // ── Numeric readout ─────────────────────────────────────────────
            <span class=classes::SLIDER_READOUT>
                {move || format!("{}", value.get().round() as i64)}
            </span>

            // ── Arrow stepper pill ──────────────────────────────────────────
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
