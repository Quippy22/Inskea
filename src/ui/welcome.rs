use super::welcome_tips;
use crate::canvas::settings::{CanvasBg, CanvasSettings};
use leptos::*;

/// One-time opening hint shown over a fresh, blank document. It is chrome,
/// not scene content, so it never appears in exports.
#[component]
pub fn WelcomeOverlay(
    welcome: RwSignal<bool>,
    generation: RwSignal<u32>,
    settings: RwSignal<CanvasSettings>,
) -> impl IntoView {
    let tip_start = welcome_tips::pick_index();
    let tip_text = move || welcome_tips::tip_for(generation.get(), tip_start);

    let word_class = move || {
        let tone = if settings.get().canvas_bg == CanvasBg::Light {
            "text-[#1a1b26]"
        } else {
            "text-fg"
        };
        format!(
            "relative z-10 select-none whitespace-nowrap text-[clamp(2.75rem,7vw,5.75rem)] font-bold \
             leading-none tracking-[0.24em] pl-[0.24em] {tone}"
        )
    };
    let tip_class = move || {
        let tone = if settings.get().canvas_bg == CanvasBg::Light {
            "text-[#565f89]"
        } else {
            "text-subtle"
        };
        format!(
            "relative z-10 mt-6 max-w-[min(92vw,44rem)] text-center font-mono text-sm \
             tracking-[0.12em] sm:text-base {tone}"
        )
    };
    let hint_class = move || {
        let tone = if settings.get().canvas_bg == CanvasBg::Light {
            "text-[#565f89]/80"
        } else {
            "text-subtle/80"
        };
        format!(
            "relative z-10 mt-5 flex flex-wrap items-center justify-center gap-x-2 gap-y-1 \
             font-mono text-[0.68rem] tracking-[0.08em] {tone}"
        )
    };
    let key_class = move || {
        if settings.get().canvas_bg == CanvasBg::Light {
            "rounded border border-[#3b4261]/30 bg-[#1a1b26]/5 px-1 py-0.5 font-mono text-[0.6rem] leading-none text-[#565f89]"
        } else {
            "rounded border border-border/80 bg-panel/60 px-1 py-0.5 font-mono text-[0.6rem] leading-none text-subtle"
        }
    };
    let accent_class = move || {
        if settings.get().canvas_bg == CanvasBg::Light {
            "text-[#3b4261]/60"
        } else {
            "text-accent/60"
        }
    };
    let subtle_class = move || {
        if settings.get().canvas_bg == CanvasBg::Light {
            "text-[#3b4261]/35"
        } else {
            "text-subtle/60"
        }
    };
    let handle_class = move || {
        if settings.get().canvas_bg == CanvasBg::Light {
            "fill-white stroke-[#3b4261]/70"
        } else {
            "fill-bg stroke-accent/80"
        }
    };
    let point_class = move || {
        if settings.get().canvas_bg == CanvasBg::Light {
            "fill-white stroke-[#3b4261]"
        } else {
            "fill-white stroke-accent"
        }
    };
    let marquee_class = move || {
        if settings.get().canvas_bg == CanvasBg::Light {
            "text-[#3b4261]/45"
        } else {
            "text-accent/55"
        }
    };

    move || {
        if !welcome.get() {
            return view! {}.into_view();
        }

        view! {
            <div class="pointer-events-none fixed inset-0 z-30 flex select-none items-center justify-center pb-[14vh]">
                <div class="flex w-full flex-col items-center px-6">
                    <div class="relative">
                        <svg
                            class="pointer-events-none absolute left-1/2 top-1/2 z-0 hidden w-[min(94vw,72rem)] -translate-x-1/2 -translate-y-1/2 md:block"
                            viewBox="0 0 1000 440"
                            fill="none"
                            xmlns="http://www.w3.org/2000/svg"
                            aria-hidden="true"
                        >
                            // Selected rectangle: outline, padded selection box,
                            // hollow corner handles, and the rotation affordance.
                            <g
                                class=subtle_class
                                stroke="currentColor"
                                stroke-width="1.5"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                transform="rotate(-6 122 121)"
                            >
                                <rect x="52" y="79" width="140" height="84" />
                            </g>
                            <g
                                class=accent_class
                                stroke="currentColor"
                                stroke-width="1.5"
                                stroke-linecap="round"
                                transform="rotate(-6 122 121)"
                            >
                                <path d="M48 65H196 M48 177H196 M38 75V167 M206 75V167" />
                                <path d="M121 30A8 8 0 1 1 110 41" />
                                <path d="M107.8 40.6L110 38L112.2 40.6" />
                            </g>
                            <g
                                class=handle_class
                                stroke-width="1.5"
                                transform="rotate(-6 122 121)"
                            >
                                <circle cx="38" cy="65" r="5" />
                                <circle cx="206" cy="65" r="5" />
                                <circle cx="38" cy="177" r="5" />
                                <circle cx="206" cy="177" r="5" />
                            </g>

                            // Crosshair, matching the canvas centre marker.
                            <g
                                class=accent_class
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                            >
                                <path d="M816 58H836 M826 48V68" />
                            </g>

                            // Curved arrow, drawn like an arrow element.
                            <g
                                class=accent_class
                                stroke="currentColor"
                                stroke-width="1.5"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <path d="M938 178C920 144 894 122 846 108" />
                                <path d="M859 98L843 108L860 117" />
                                <circle
                                    cx="938"
                                    cy="178"
                                    r="3"
                                    fill="currentColor"
                                    stroke="none"
                                />
                            </g>

                            // Freehand stroke with its path points.
                            <g transform="translate(-8 28)">
                                <g
                                    class=accent_class
                                    stroke="currentColor"
                                    stroke-width="1.8"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <path d="M54 358C82 318 112 320 138 346S190 380 220 346S268 312 298 342S336 370 366 338" />
                                    <path
                                        d="M54 358L28 386"
                                        stroke-width="1.2"
                                        stroke-dasharray="4 2"
                                        opacity="0.7"
                                    />
                                </g>
                                <g class=point_class stroke-width="1.5">
                                    <circle cx="54" cy="358" r="5" />
                                    <circle cx="220" cy="346" r="5" />
                                    <circle cx="366" cy="338" r="5" />
                                </g>
                                <g
                                    class=accent_class
                                    stroke="currentColor"
                                    stroke-width="1"
                                    opacity="0.6"
                                >
                                    <circle cx="138" cy="346" r="3.5" />
                                    <circle cx="298" cy="342" r="3.5" />
                                </g>
                            </g>

                            // Marquee selection around an ellipse.
                            <g
                                class=marquee_class
                                stroke="currentColor"
                                fill="currentColor"
                                stroke-width="1.2"
                                stroke-dasharray="4 2"
                            >
                                <rect
                                    x="806"
                                    y="322"
                                    width="138"
                                    height="84"
                                    fill-opacity="0.07"
                                    transform="rotate(4 875 364)"
                                />
                            </g>
                            <g class=subtle_class stroke="currentColor" stroke-width="1.5">
                                <ellipse
                                    cx="875"
                                    cy="364"
                                    rx="48"
                                    ry="29"
                                    transform="rotate(-14 875 364)"
                                />
                            </g>

                            <g
                                class=subtle_class
                                stroke="currentColor"
                                stroke-width="1.5"
                                stroke-linecap="round"
                            >
                                <path d="M198 407H216 M207 398V416" />
                            </g>
                        </svg>

                        <h1
                            class=word_class
                            style="font-family: Arial, 'Liberation Sans', 'Helvetica Neue', sans-serif;"
                            aria-label="Inskea"
                        >
                            "INSKE"<span>"Λ"</span>
                        </h1>
                    </div>

                    <p class=tip_class>{tip_text}</p>

                    <div class=hint_class>
                        <span>"Scroll to zoom"</span>
                        <span class="opacity-40">"·"</span>
                        <span class="flex items-center gap-1.5">
                            "Press" <kbd class=key_class>"?"</kbd> "for shortcuts"
                        </span>
                    </div>
                </div>
            </div>
        }
        .into_view()
    }
}
