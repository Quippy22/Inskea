use crate::ui::components::SegmentedControl;
use crate::ui::classes;
use crate::ui::icon;
use leptos::*;
use serde::{Deserialize, Serialize};

// ── Settings types ──────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CenterStyle {
    Crosshair,
    Dot,
    Off,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GridStyle {
    Dot,
    Line,
    Off,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GridSize {
    Px10,
    Px20,
    Px30,
}

impl GridSize {
    pub fn px(&self) -> f64 {
        match self {
            GridSize::Px10 => 10.0,
            GridSize::Px20 => 20.0,
            GridSize::Px30 => 30.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CanvasBg {
    Dark,
    Light,
}

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
pub fn SettingsPanel(
    center_style: RwSignal<CenterStyle>,
    grid_style: RwSignal<GridStyle>,
    grid_size: RwSignal<GridSize>,
    autosave: RwSignal<bool>,
    canvas_bg: RwSignal<CanvasBg>,
) -> impl IntoView {
    let open = create_rw_signal(false);

    let close = move || open.set(false);
    let toggle = move |_| open.update(|v| *v = !*v);

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

    let size_opts = &[
        (GridSize::Px10, "10px"),
        (GridSize::Px20, "20px"),
        (GridSize::Px30, "30px"),
    ];

    let toggle_opts = &[(true, "On"), (false, "Off")];

    let bg_opts = &[
        (CanvasBg::Dark, "Dark"),
        (CanvasBg::Light, "Light"),
    ];

    view! {
        <div class="fixed top-4 right-4 z-50 pointer-events-none">
            <div class="relative pointer-events-auto">
                <div class=classes::PANEL>
                    <button
                        class=classes::BTN_COLLAPSE
                        on:click=toggle
                        title="Settings"
                    >
                        {icon::gear()}
                    </button>
                </div>

                {move || {
                    if !open.get() {
                        return view! {}.into_view();
                    }

                    view! {
                        <>
                            <div class="fixed inset-0 z-40" on:click=move |_| close()></div>

                            <div class=classes::SETTINGS_WINDOW>
                                <div class="flex items-center justify-between mb-3">
                                    <span class=classes::SETTINGS_LABEL>"Center"</span>
                                    <SegmentedControl
                                        options=center_opts
                                        active=center_style
                                    />
                                </div>

                                <div class="flex items-center justify-between mb-3">
                                    <span class=classes::SETTINGS_LABEL>"Grid"</span>
                                    <SegmentedControl
                                        options=grid_opts
                                        active=grid_style
                                    />
                                </div>

                                <div class="flex items-center justify-between mb-3">
                                    <span class=classes::SETTINGS_LABEL>"Grid size"</span>
                                    <SegmentedControl
                                        options=size_opts
                                        active=grid_size
                                    />
                                </div>

                                <div class="flex items-center justify-between mb-3">
                                    <span class=classes::SETTINGS_LABEL>"Canvas bg"</span>
                                    <SegmentedControl
                                        options=bg_opts
                                        active=canvas_bg
                                    />
                                </div>

                                <div class="flex items-center justify-between">
                                    <span class=classes::SETTINGS_LABEL>"Autosave"</span>
                                    <SegmentedControl
                                        options=toggle_opts
                                        active=autosave
                                    />
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
