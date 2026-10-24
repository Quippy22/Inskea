use crate::canvas::settings::{CanvasSettings, CenterStyle, GridSize, GridStyle, Theme};
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
    #[serde(default)]
    theme: Option<Theme>,
    /// Pre-theme settings stored a two-value canvas background. Read it only
    /// so old files keep their look after the upgrade.
    #[serde(default, skip_serializing)]
    canvas_bg: Option<LegacyCanvasBg>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
enum LegacyCanvasBg {
    Dark,
    Light,
}

/// Serialize settings to a TOML string.
pub fn to_toml(
    center_style: CenterStyle,
    grid_style: GridStyle,
    grid_size: GridSize,
    autosave: bool,
    theme: Theme,
) -> String {
    let s = PersistentSettings {
        center_style,
        grid_style,
        grid_size,
        autosave,
        theme: Some(theme),
        canvas_bg: None,
    };
    toml::to_string(&s).unwrap_or_default()
}

/// Deserialize a TOML string back into settings.
/// Returns `None` if the content is empty or malformed.
pub fn from_toml(content: &str) -> Option<(CenterStyle, GridStyle, GridSize, bool, Theme)> {
    if content.is_empty() {
        return None;
    }
    let s: PersistentSettings = toml::from_str(content).ok()?;
    let theme = s
        .theme
        .or_else(|| {
            s.canvas_bg.map(|bg| match bg {
                LegacyCanvasBg::Dark => Theme::Storm,
                LegacyCanvasBg::Light => Theme::Paper,
            })
        })
        .unwrap_or(Theme::Storm);
    Some((s.center_style, s.grid_style, s.grid_size, s.autosave, theme))
}

// ── Settings panel ──────────────────────────────────────────────────────────

#[component]
pub fn SettingsPanel(
    settings: RwSignal<CanvasSettings>,
    theme_open: RwSignal<bool>,
) -> impl IntoView {
    let open = create_rw_signal(false);

    let center_style = create_rw_signal(settings.get().center_style);
    let grid_style = create_rw_signal(settings.get().grid_style);
    let grid_size = create_rw_signal(settings.get().grid_size.px());
    let autosave = create_rw_signal(settings.get().autosave);

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
                                    <span class=styles::SETTINGS_LABEL>"Theme"</span>
                                    <button
                                        class=styles::SETTINGS_THEME_BTN
                                        on:click=move |_| {
                                            close();
                                            theme_open.set(true);
                                        }
                                        title="Change theme"
                                    >
                                        <span
                                            class="h-3 w-3 rounded-full border border-black/20 shrink-0"
                                            style=move || {
                                                format!("background:{}", settings.get().theme.accent())
                                            }
                                        ></span>
                                        <span>{move || settings.get().theme.label()}</span>
                                    </button>
                                </div>

                                <div class="h-px bg-border/70 mb-3"></div>

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_through_toml() {
        let content = to_toml(
            CenterStyle::Dot,
            GridStyle::Line,
            GridSize::new(37.5),
            true,
            Theme::Paper,
        );
        assert!(
            from_toml(&content)
                == Some((
                    CenterStyle::Dot,
                    GridStyle::Line,
                    GridSize::new(37.5),
                    true,
                    Theme::Paper,
                ))
        );
    }

    #[test]
    fn empty_settings_are_not_loaded() {
        assert!(from_toml("").is_none());
    }

    #[test]
    fn legacy_canvas_background_maps_to_a_theme() {
        let dark = r#"
center_style = "Crosshair"
grid_style = "Dot"
grid_size = 30.0
autosave = false
canvas_bg = "Dark"
"#;
        let light = dark.replace("Dark", "Light");
        assert_eq!(from_toml(dark).unwrap().4, Theme::Storm);
        assert_eq!(from_toml(&light).unwrap().4, Theme::Paper);
    }
}
