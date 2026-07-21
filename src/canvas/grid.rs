use crate::canvas::settings::{CanvasSettings, CenterStyle, GridStyle};
use leptos::*;

/// Returns a reactive closure that renders the grid pattern and centre indicator.
pub fn grid_overlay(settings: RwSignal<CanvasSettings>) -> impl Fn() -> View {
    move || {
        let gs = settings.get().grid_style;
        let sz = settings.get().grid_size.px();
        let half = sz / 2.0;

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
                        fill="#d1d5db"
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
                        stroke="#d1d5db"
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

        let center = match settings.get().center_style {
            CenterStyle::Crosshair => {
                view! { <path d="M-12,0 L12,0 M0,-12 L0,12" stroke="#7aa2f7" stroke-width="2" /> }
                    .into_view()
            }
            CenterStyle::Dot => view! { <circle cx="0" cy="0" r="3" fill="#7aa2f7" /> }.into_view(),
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
