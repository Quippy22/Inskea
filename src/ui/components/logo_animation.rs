use leptos::*;
#[allow(unused_imports)]
use std::time::Duration;

/// Reusable Inskea animated logo component based on the technical drafting compass,
/// pencil, underline wave, and INSKEA wordmark.
#[component]
pub fn LogoAnimation(
    /// Dimension in pixels (width and height)
    #[prop(default = 220.0)]
    size: f64,
    /// Whether to render the INSKEA wordmark typography
    #[prop(default = true)]
    show_text: bool,
    /// Whether the technical blueprint compass circle rotates
    #[prop(default = true)]
    spinning: bool,
    /// Whether entrance/stroke draw and pen float animations are enabled
    #[prop(default = true)]
    animated: bool,
    /// Fast spin mode (for compact loaders or export processing)
    #[prop(default = false)]
    fast_spin: bool,
    /// Optional additional CSS classes on the outer wrapper
    #[prop(default = "")]
    class: &'static str,
) -> impl IntoView {
    let spin_class = if spinning {
        if fast_spin {
            "animate-logo-spin-fast"
        } else {
            "animate-logo-spin"
        }
    } else {
        ""
    };

    let wave_class = if animated { "animate-logo-wave" } else { "" };
    let pen_class = if animated { "animate-logo-pen" } else { "" };
    let circle_class = if animated { "animate-logo-circle" } else { "" };

    view! {
        <div
            class=format!("relative inline-flex items-center justify-center select-none text-fg {}", class)
            style=format!("width: {size}px; height: {size}px;")
        >
            <svg
                viewBox="0 0 500 500"
                class="w-full h-full overflow-visible"
                fill="none"
                xmlns="http://www.w3.org/2000/svg"
            >
                // ROTATING DRAFTING COMPASS RIG
                <g class=spin_class style="transform-origin: 250px 250px;">
                    // Extended axis lines
                    <line x1="250" y1="36" x2="250" y2="464" stroke="currentColor" stroke-width="1.2" stroke-opacity="0.45" />
                    <line x1="36" y1="250" x2="464" y2="250" stroke="currentColor" stroke-width="1.2" stroke-opacity="0.45" />

                    // Crosshair tick marks on axes
                    // Top crosshair
                    <path d="M 242 80 L 258 80 M 250 72 L 250 88" stroke="currentColor" stroke-width="1.6" stroke-opacity="0.85" />
                    // Bottom crosshair
                    <path d="M 242 420 L 258 420 M 250 412 L 250 428" stroke="currentColor" stroke-width="1.6" stroke-opacity="0.85" />
                    // Left crosshair
                    <path d="M 72 250 L 88 250 M 80 242 L 80 258" stroke="currentColor" stroke-width="1.6" stroke-opacity="0.85" />
                    // Right crosshair
                    <path d="M 412 250 L 428 250 M 420 242 L 420 258" stroke="currentColor" stroke-width="1.6" stroke-opacity="0.85" />

                    // Interior crosshairs
                    <path d="M 137 155 L 153 155 M 145 147 L 145 163" stroke="currentColor" stroke-width="1.4" stroke-opacity="0.65" />
                    <path d="M 322 355 L 338 355 M 330 347 L 330 363" stroke="currentColor" stroke-width="1.4" stroke-opacity="0.65" />

                    // Main drafting circle
                    <circle
                        cx="250"
                        cy="250"
                        r="170"
                        stroke="currentColor"
                        stroke-width="1.6"
                        stroke-opacity="0.75"
                        class=circle_class
                    />

                    // Dashed technical arc (in top-right section)
                    <circle
                        cx="250"
                        cy="250"
                        r="170"
                        stroke="currentColor"
                        stroke-width="2.2"
                        stroke-dasharray="14 10"
                        stroke-dashoffset="60"
                        stroke-opacity="0.85"
                    />

                    // Selection box with corner anchor handles (top-right quadrant)
                    <rect x="388" y="106" width="44" height="44" stroke="currentColor" stroke-width="1.4" stroke-opacity="0.75" fill="none" />
                    <rect x="385" y="103" width="6" height="6" stroke="currentColor" stroke-width="1.2" fill="var(--color-bg, #0b132b)" />
                    <rect x="429" y="103" width="6" height="6" stroke="currentColor" stroke-width="1.2" fill="var(--color-bg, #0b132b)" />
                    <rect x="385" y="147" width="6" height="6" stroke="currentColor" stroke-width="1.2" fill="var(--color-bg, #0b132b)" />
                    <rect x="429" y="147" width="6" height="6" stroke="currentColor" stroke-width="1.2" fill="var(--color-bg, #0b132b)" />

                    // Tangent circular arc & directional arrow (right side)
                    <path d="M 432 250 A 182 182 0 0 0 414 168" stroke="currentColor" stroke-width="1.4" stroke-opacity="0.75" fill="none" />
                    <polygon points="414,168 410,178 422,176" fill="currentColor" opacity="0.85" />

                    // Secondary tangent circle with node dot & projection line (bottom-left quadrant)
                    <circle cx="100" cy="355" r="34" stroke="currentColor" stroke-width="1.4" stroke-opacity="0.75" fill="none" />
                    <circle cx="105" cy="322" r="3.2" fill="currentColor" opacity="0.9" />
                    <line x1="110" y1="350" x2="175" y2="395" stroke="currentColor" stroke-width="1.2" stroke-dasharray="6 5" stroke-opacity="0.65" />
                </g>

                // WAVE RIBBON (forming from the pencil tip)
                <path
                    d="M 198 238 C 218 256, 248 256, 276 238 C 304 220, 332 222, 344 236"
                    stroke="currentColor"
                    stroke-width="7"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    fill="none"
                    class=wave_class
                />

                // DRAFTING PENCIL (in foreground, with subtle floating animation)
                <g class=pen_class style="transform-origin: 200px 240px;">
                    // Wooden cone tip
                    <polygon points="198,238 210,216 226,228" fill="#e2e8f0" />
                    // Dark graphite tip
                    <polygon points="198,238 203,229 209,234" fill="#1e293b" />
                    // Main pencil barrel - faceted highlights
                    <polygon points="210,216 258,168 268,178 220,226" fill="#f8fafc" />
                    <polygon points="220,226 268,178 274,184 226,228" fill="#cbd5e1" />
                    // Metal ferrule ring
                    <polygon points="258,168 266,160 282,176 274,184" fill="#94a3b8" />
                    // Eraser rounded cap
                    <path
                        d="M 266 160 L 273 153 C 277 149, 283 149, 287 153 L 291 157 C 295 161, 295 167, 291 171 L 282 176 Z"
                        fill="#f1f5f9"
                    />
                </g>

                // INSKEA TYPOGRAPHY
                {show_text.then(|| view! {
                    <g
                        class="select-none text-current"
                        style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;"
                    >
                        <text
                            x="250"
                            y="308"
                            text-anchor="middle"
                            font-size="34"
                            font-weight="800"
                            letter-spacing="7"
                            fill="currentColor"
                        >
                            "INSKE"
                            <tspan dy="-0.5" font-size="33">"\u{039B}"</tspan>
                        </text>
                    </g>
                })}
            </svg>
        </div>
    }
}

/// Opening splash screen that displays the animated logo while the canvas and settings initialize.
/// Once `loaded` is true (and a minimum duration has passed to prevent flickering), it smoothly dissolves away.
#[component]
pub fn OpeningScreen(
    /// Signal indicating that canvas settings, fonts, and state have loaded
    loaded: Signal<bool>,
    /// Minimum duration to show the splash screen before dissolving (ms)
    #[prop(default = 700)]
    #[allow(unused_variables)]
    min_duration_ms: u64,
) -> impl IntoView {
    let visible = create_rw_signal(true);
    let dissolving = create_rw_signal(false);

    // Ensure splash is displayed for at least min_duration_ms for smooth visual rhythm
    let min_time_elapsed = create_rw_signal(false);
    #[cfg(target_arch = "wasm32")]
    set_timeout(
        move || {
            min_time_elapsed.set(true);
        },
        Duration::from_millis(min_duration_ms),
    );
    #[cfg(not(target_arch = "wasm32"))]
    min_time_elapsed.set(true);

    // Watch for both loaded and min_time_elapsed
    create_effect(move |_| {
        if loaded.get() && min_time_elapsed.get() && !dissolving.get_untracked() {
            dissolving.set(true);
            #[cfg(target_arch = "wasm32")]
            set_timeout(
                move || {
                    visible.set(false);
                },
                Duration::from_millis(450),
            );
            #[cfg(not(target_arch = "wasm32"))]
            visible.set(false);
        }
    });

    move || {
        if !visible.get() {
            return view! {}.into_view();
        }

        let dissolve_class = if dissolving.get() {
            "animate-splash-fade-out pointer-events-none"
        } else {
            ""
        };

        view! {
            <div
                class=format!(
                    "fixed inset-0 z-[9999] flex flex-col items-center justify-center bg-[var(--color-bg,#0f172a)] select-none overflow-hidden transition-all duration-400 {}",
                    dissolve_class
                )
                aria-label="Loading Inskea"
            >
                // Subtle radial glow centered behind logo
                <div class="pointer-events-none absolute w-[480px] h-[480px] rounded-full bg-[var(--accent-glow,rgba(122,162,247,0.25))] blur-3xl opacity-40 animate-logo-pulse-glow" />

                // Animated Logo
                <div class="relative z-10 flex flex-col items-center">
                    <LogoAnimation size=240.0 spinning=true animated=true show_text=true />

                    // Subtle loading status
                    <div class="mt-4 flex items-center gap-2.5 font-mono text-[0.7rem] uppercase tracking-[0.2em] text-subtle/80">
                        <span class="inline-block w-1.5 h-1.5 rounded-full bg-accent animate-pulse" />
                        <span>"INITIALIZING CANVAS"</span>
                    </div>
                </div>
            </div>
        }.into_view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logo_animation_creates_view() {
        let runtime = leptos::create_runtime();
        let view = view! {
            <LogoAnimation size=200.0 show_text=true spinning=true />
        };
        let html = view.into_view().render_to_string();
        assert!(html.contains("svg"));
        assert!(html.contains("INSKE"));
        assert!(html.contains("animate-logo-spin"));
        runtime.dispose();
    }

    #[test]
    fn test_opening_screen_creates_view() {
        let runtime = leptos::create_runtime();
        let loaded = create_rw_signal(false);
        let view = view! {
            <OpeningScreen loaded=loaded.into() min_duration_ms=100 />
        };
        let html = view.into_view().render_to_string();
        assert!(html.contains("INITIALIZING CANVAS"));
        runtime.dispose();
    }
}
