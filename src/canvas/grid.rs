use crate::canvas::settings::{CanvasSettings, CenterStyle, GridStyle};
use leptos::*;

/// Returns a reactive closure that renders the grid pattern and centre indicator.
pub fn grid_overlay(
    settings: RwSignal<CanvasSettings>,
    hide_center: RwSignal<bool>,
) -> impl Fn() -> View {
    move || {
        let settings_state = settings.get();
        let gs = settings_state.grid_style;
        let sz = settings_state.grid_size.px();
        let theme = settings_state.theme;
        let half = sz / 2.0;
        let grid_color = theme.grid();

        let pattern = match gs {
            GridStyle::Dot => view! {
                <pattern
                    id="grid-dot"
                    width=sz.to_string()
                    height=sz.to_string()
                    patternUnits="userSpaceOnUse"
                    patternTransform=format!("translate({}, {})", -half, -half)
                >
                    <circle
                        cx=half.to_string()
                        cy=half.to_string()
                        r="1.5"
                        fill=grid_color
                        fill-opacity="0.25"
                    />
                </pattern>
            }
            .into_view(),
            GridStyle::Line => view! {
                <pattern
                    id="grid-line"
                    width=sz.to_string()
                    height=sz.to_string()
                    patternUnits="userSpaceOnUse"
                >
                    <path
                        d=format!("M {} 0 L 0 0 0 {}", sz, sz)
                        fill="none"
                        stroke=grid_color
                        stroke-opacity="0.25"
                        stroke-width="1"
                    />
                </pattern>
            }
            .into_view(),
            GridStyle::Off => view! {}.into_view(),
        };

        let rect = match gs {
            GridStyle::Off => view! {}.into_view(),
            _ => {
                let fill_id = match gs {
                    GridStyle::Dot => "url(#grid-dot)",
                    GridStyle::Line => "url(#grid-line)",
                    _ => "",
                };
                view! { <rect x="-100000" y="-100000" width="200000" height="200000" fill=fill_id /> }
                .into_view()
            }
        };

        // The centre marker is hidden with a class rather than removed: a
        // pointerdown that hides the welcome overlay must not replace DOM
        // nodes, or Leptos's delegated handler loses the event target and the
        // canvas never sees the click.
        let hidden = move || hide_center.get();
        let center = match settings_state.center_style {
            CenterStyle::Crosshair => view! {
                <path
                    class:hidden=hidden
                    d="M-12,0 L12,0 M0,-12 L0,12"
                    stroke=theme.accent()
                    stroke-width="2"
                />
            }
            .into_view(),
            CenterStyle::Dot => {
                view! { <circle class:hidden=hidden cx="0" cy="0" r="3" fill=theme.accent() /> }
                    .into_view()
            }
            CenterStyle::Off => view! {}.into_view(),
        };

        view! {
            <defs>{pattern}</defs>
            {rect}
            {center}
        }
        .into_view()
    }
}
