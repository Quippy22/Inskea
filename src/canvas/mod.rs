mod viewport;
pub use viewport::Viewport;

use leptos::svg::Svg;
use leptos::*;

#[component]
pub fn Canvas() -> impl IntoView {
    let viewport = create_rw_signal(Viewport::default());

    // Screen size in pixels. The SVG is deliberately sized to 100% of a
    // `w-screen h-screen` parent (see Phase 1, step 1), so the window's
    // inner size IS the SVG's screen size — no need to measure the DOM
    // element itself, which sidesteps needing the `DomRect` web-sys
    // feature flag enabled just for this.
    fn window_size() -> (f64, f64) {
        let window = web_sys::window().expect("no global `window` exists");
        let w = window
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let h = window
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        (w, h)
    }

    let screen_size = create_rw_signal(window_size());
    let svg_ref = create_node_ref::<Svg>();

    // Kept in sync on every window resize.
    let _ = window_event_listener(ev::resize, move |_| {
        screen_size.set(window_size());
    });

    // Reactive viewBox string: recomputes whenever viewport or screen_size change.
    let view_box = move || {
        let (w, h) = screen_size.get();
        viewport.get().to_view_box(w, h)
    };

    view! {
        <svg
            _ref=svg_ref
            width="100%"
            height="100%"
            style="display: block;"
            viewBox=view_box
        >
            <defs>
                <pattern
                    id="grid"
                    width="40"
                    height="40"
                    patternUnits="userSpaceOnUse"
                >
                    <circle cx="1" cy="1" r="1" fill="#d1d5db" />
                </pattern>
            </defs>

            // Background — sized generously past any reasonable viewBox so
            // panning doesn't run off the edge of it. Revisit if this ever
            // becomes a real limit on how far the user can pan.
            <rect
                x="-100000"
                y="-100000"
                width="200000"
                height="200000"
                fill="url(#grid)"
            />

            // Temporary: proves world (0,0) really is screen-centered.
            // Remove once Phase 2's real element rendering (`<For>` loop) is in.
            <rect
                x="-60"
                y="-40"
                width="120"
                height="80"
                fill="#3b82f6"
                stroke="#1e40af"
                stroke-width="2"
            />
        </svg>
    }
}
