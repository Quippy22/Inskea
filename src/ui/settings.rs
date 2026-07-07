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

// ── Persistence ─────────────────────────────────────────────────────────────

/// TOML-serializable snapshot of all settings.
#[derive(Serialize, Deserialize)]
struct PersistentSettings {
    center_style: CenterStyle,
    grid_style: GridStyle,
    grid_size: GridSize,
    autosave: bool,
}

/// Serialize the four settings to a TOML string.
pub fn to_toml(
    center_style: CenterStyle,
    grid_style: GridStyle,
    grid_size: GridSize,
    autosave: bool,
) -> String {
    let s = PersistentSettings {
        center_style,
        grid_style,
        grid_size,
        autosave,
    };
    toml::to_string(&s).unwrap_or_default()
}

/// Deserialize a TOML string back into the four settings.
/// Returns `None` if the content is empty or malformed.
pub fn from_toml(content: &str) -> Option<(CenterStyle, GridStyle, GridSize, bool)> {
    if content.is_empty() {
        return None;
    }
    let s: PersistentSettings = toml::from_str(content).ok()?;
    Some((s.center_style, s.grid_style, s.grid_size, s.autosave))
}

// ── Settings panel ──────────────────────────────────────────────────────────

#[component]
pub fn SettingsPanel(
    center_style: RwSignal<CenterStyle>,
    grid_style: RwSignal<GridStyle>,
    grid_size: RwSignal<GridSize>,
    autosave: RwSignal<bool>,
) -> impl IntoView {
    let open = create_rw_signal(false);

    let close = move || open.set(false);
    let toggle = move |_| open.update(|v| *v = !*v);

    // ── Options arrays (label order matches user request) ──────────────────
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
                            // backdrop
                            <div class="fixed inset-0 z-40" on:click=move |_| close()></div>

                            <div class=classes::SETTINGS_WINDOW>
                                // ── Center ─────────────────────────────────
                                <div class="flex items-center justify-between mb-3">
                                    <span class=classes::SETTINGS_LABEL>"Center"</span>
                                    <SegmentedControl
                                        options=center_opts
                                        active=center_style
                                    />
                                </div>

                                // ── Grid style ────────────────────────────
                                <div class="flex items-center justify-between mb-3">
                                    <span class=classes::SETTINGS_LABEL>"Grid"</span>
                                    <SegmentedControl
                                        options=grid_opts
                                        active=grid_style
                                    />
                                </div>

                                // ── Grid size ─────────────────────────────
                                <div class="flex items-center justify-between mb-3">
                                    <span class=classes::SETTINGS_LABEL>"Grid size"</span>
                                    <SegmentedControl
                                        options=size_opts
                                        active=grid_size
                                    />
                                </div>

                                // ── Autosave ──────────────────────────────
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

// ── Reusable segmented control ──────────────────────────────────────────────

/// Renders a horizontal segmented button group (e.g. `[Crosshair | Dot | Off]`).
///
/// The active segment is highlighted; segments are visually separated by a
/// border-right on all but the last item.
#[component]
fn SegmentedControl<T: PartialEq + Copy + 'static>(
    options: &'static [(T, &'static str)],
    active: RwSignal<T>,
) -> impl IntoView {
    let last = options.len() - 1;

    view! {
        <div class="flex rounded-md border border-border overflow-hidden">
            {options
                .iter()
                .enumerate()
                .map(|(i, (val, label))| {
                    let is_last = i == last;
                    let val = *val;
                    view! {
                        <button
                            class=move || if active.get() == val {
                                classes::SEG_BTN_ACTIVE
                            } else {
                                classes::SEG_BTN_INACTIVE
                            }
                            class:border-r=move || !is_last
                            class:border-border=move || !is_last
                            on:click=move |_| active.set(val)
                        >
                            {*label}
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}
