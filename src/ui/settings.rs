use crate::canvas::settings::{CanvasBg, CanvasSettings, CenterStyle, GridSize, GridStyle};
use crate::ui::components::{IconButton, NumberSlider, SegmentedControl};
use crate::ui::icon;
use crate::ui::styles;
use leptos::*;
use serde::{Deserialize, Serialize};

// ── Persistence ─────────────────────────────────────────────────────────────

/// TOML-serializable snapshot of all settings.
#[derive(Serialize, Deserialize)]
struct PersistentSettings {
    center_style: CenterStyle,
    grid_style: GridStyle,
    grid_size: GridSize,
    autosave: bool,
    canvas_bg: Option<CanvasBg>,
}

/// Serialize settings to a TOML string.
pub fn to_toml(
    center_style: CenterStyle,
    grid_style: GridStyle,
    grid_size: GridSize,
    autosave: bool,
    canvas_bg: CanvasBg,
) -> String {
    let s = PersistentSettings {
        center_style,
        grid_style,
        grid_size,
        autosave,
        canvas_bg: Some(canvas_bg),
    };
    toml::to_string(&s).unwrap_or_default()
}

/// Deserialize a TOML string back into settings.
/// Returns `None` if the content is empty or malformed.
pub fn from_toml(content: &str) -> Option<(CenterStyle, GridStyle, GridSize, bool, CanvasBg)> {
    if content.is_empty() {
        return None;
    }
    let s: PersistentSettings = toml::from_str(content).ok()?;
    Some((
        s.center_style,
        s.grid_style,
        s.grid_size,
        s.autosave,
        s.canvas_bg.unwrap_or(CanvasBg::Dark),
    ))
}

// ── Settings panel ──────────────────────────────────────────────────────────

#[component]
pub fn SettingsPanel(settings: RwSignal<CanvasSettings>) -> impl IntoView {
    let open = create_rw_signal(false);

    let center_style = create_rw_signal(settings.get().center_style);
    let grid_style = create_rw_signal(settings.get().grid_style);
    let grid_size = create_rw_signal(settings.get().grid_size.px());
    let autosave = create_rw_signal(settings.get().autosave);
    let canvas_bg = create_rw_signal(settings.get().canvas_bg);

    create_effect(move |_| {
        let s = settings.get();
        if center_style.get_untracked() != s.center_style {
            center_style.set(s.center_style);
        }
        if grid_style.get_untracked() != s.grid_style {
            grid_style.set(s.grid_style);
        }
        if grid_size.get_untracked() != s.grid_size.px() {
            grid_size.set(s.grid_size.px());
        }
        if autosave.get_untracked() != s.autosave {
            autosave.set(s.autosave);
        }
        if canvas_bg.get_untracked() != s.canvas_bg {
            canvas_bg.set(s.canvas_bg);
        }
    });

    create_effect(move |_| {
        settings.update(|s| s.center_style = center_style.get());
    });
    create_effect(move |_| {
        settings.update(|s| s.grid_style = grid_style.get());
    });
    create_effect(move |_| {
        settings.update(|s| s.grid_size = GridSize::new(grid_size.get()));
    });
    create_effect(move |_| {
        settings.update(|s| s.autosave = autosave.get());
    });
    create_effect(move |_| {
        settings.update(|s| s.canvas_bg = canvas_bg.get());
    });

    let close = move || open.set(false);
    let toggle = move || open.update(|v| *v = !*v);

    // ── Options arrays ────────────────────────────────────
    let center_opts = &[
        (CenterStyle::Crosshair, "Crosshair"),
        (CenterStyle::Dot, "Dot"),
        (CenterStyle::Off, "Off"),
    ];

    let grid_opts = &[
        (GridStyle::Dot, "Dot grid"),
        (GridStyle::Line, "Line"),
        (GridStyle::Off, "Off"),
    ];

    let toggle_opts = &[(true, "On"), (false, "Off")];

    let bg_opts = &[(CanvasBg::Dark, "Dark"), (CanvasBg::Light, "Light")];

    view! {
        <div class="fixed top-4 right-4 z-50 pointer-events-none">
            <div class="relative pointer-events-auto">
                <div class=styles::PANEL>
                    <IconButton on_click=toggle title="Settings" class=styles::BTN_COLLAPSE>
                        {icon::gear()}
                    </IconButton>
                </div>

                {move || {
                    if !open.get() {
                        return view! {}.into_view();
                    }

                    view! {
                        <>
                            <div class="fixed inset-0 z-40" on:click=move |_| close()></div>

                            <div class=styles::SETTINGS_WINDOW>
                                <div class="flex items-center justify-between mb-3">
                                    <span class=styles::SETTINGS_LABEL>"Center"</span>
                                    <SegmentedControl options=center_opts active=center_style />
                                </div>

                                <div class="flex items-center justify-between mb-3">
                                    <span class=styles::SETTINGS_LABEL>"Grid"</span>
                                    <SegmentedControl options=grid_opts active=grid_style />
                                </div>

                                <div class="mb-3">
                                    <NumberSlider
                                        value=grid_size
                                        min=5.0
                                        max=100.0
                                        increment=5.0
                                        label="Grid size"
                                    />
                                </div>

                                <div class="flex items-center justify-between mb-3">
                                    <span class=styles::SETTINGS_LABEL>"Canvas bg"</span>
                                    <SegmentedControl options=bg_opts active=canvas_bg />
                                </div>

                                <div class="flex items-center justify-between">
                                    <span class=styles::SETTINGS_LABEL>"Autosave"</span>
                                    <SegmentedControl options=toggle_opts active=autosave />
                                </div>
                            </div>
                        </>
                    }
                        .into_view()
                }}
            </div>
        </div>
    }
}
