use std::rc::Rc;

use crate::canvas::settings::{CanvasSettings, CenterStyle, GridSize, GridStyle, Theme};
use crate::document::{PendingAction, RequestAction};
use crate::tauri_bridge::{self, UpdateCheck, UpdateOutcome};
use crate::ui::catalogue::{Catalogue, CatalogueContext};
use crate::ui::components::{IconButton, NumberSlider, SegmentedControl};
use crate::ui::icon;
use crate::ui::notify;
use crate::ui::styles;
use leptos::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Debug, PartialEq, Eq)]
enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    Available(UpdateCheck),
    Installing(UpdateCheck),
    ReadyToRelaunch(String),
    Failed,
}

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
    catalogue: CatalogueContext,
    request_action: RequestAction,
) -> impl IntoView {
    let is_tauri = tauri_bridge::is_tauri();
    let open = create_rw_signal(false);
    let update_state = create_rw_signal(UpdateState::Idle);
    let manual_checked = create_rw_signal(false);
    let relaunch_prompt = create_rw_signal::<Option<String>>(None);
    let notifications = catalogue.notifications;

    let run_update_check = move |user_initiated: bool| {
        if !is_tauri {
            return;
        }
        if matches!(
            update_state.get_untracked(),
            UpdateState::Checking | UpdateState::Installing(_) | UpdateState::ReadyToRelaunch(_)
        ) {
            return;
        }
        if user_initiated {
            manual_checked.set(true);
        }
        update_state.set(UpdateState::Checking);
        spawn_local(async move {
            match tauri_bridge::check_for_update().await {
                Ok(info) => {
                    if info.update_available {
                        let version = info.latest_version.clone();
                        update_state.set(UpdateState::Available(info));
                        if user_initiated {
                            notify::info(notifications, format!("Inskea v{version} is available."));
                        } else {
                            notify::info(
                                notifications,
                                format!("Inskea v{version} is available in Settings."),
                            );
                        }
                    } else {
                        let current = info.current_version.clone();
                        update_state.set(UpdateState::UpToDate);
                        if user_initiated {
                            notify::info(
                                notifications,
                                format!("Inskea v{current} is up to date."),
                            );
                        }
                    }
                }
                Err(e) => {
                    update_state.set(UpdateState::Failed);
                    if user_initiated {
                        notify::error(notifications, format!("Could not check for updates: {e}"));
                    }
                }
            }
        });
    };

    let run_update_install = move |info: UpdateCheck| {
        if matches!(
            update_state.get_untracked(),
            UpdateState::Checking | UpdateState::Installing(_) | UpdateState::ReadyToRelaunch(_)
        ) {
            return;
        }
        update_state.set(UpdateState::Installing(info.clone()));
        spawn_local(async move {
            let version = info.latest_version.clone();
            match tauri_bridge::install_update(&info).await {
                Ok(UpdateOutcome::ReadyToRelaunch) => {
                    open.set(false);
                    update_state.set(UpdateState::ReadyToRelaunch(version.clone()));
                    relaunch_prompt.set(Some(version));
                }
                Ok(UpdateOutcome::OpenedPage) => {
                    update_state.set(UpdateState::Available(info));
                    notify::info(
                        notifications,
                        format!("Opened the Inskea v{version} release page."),
                    );
                }
                Err(e) => {
                    update_state.set(UpdateState::Available(info));
                    notify::error(notifications, format!("Could not update Inskea: {e}"));
                }
            }
        });
    };

    let trigger_relaunch: Rc<dyn Fn()> = {
        let request_action = request_action.clone();
        Rc::new(move || {
            open.set(false);
            relaunch_prompt.set(None);
            request_action(PendingAction {
                label: "relaunch Inskea",
                run: Rc::new(tauri_bridge::relaunch_app),
            });
        })
    };

    // Check for updates automatically every time the desktop application opens.
    if is_tauri {
        run_update_check(false);
    }

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
                <div class=format!("{} flex items-center", styles::PANEL)>
                    <Catalogue context=catalogue />
                    <div class="relative">
                        <IconButton on_click=toggle title="Settings" class=styles::BTN_COLLAPSE>
                            {icon::gear()}
                        </IconButton>
                        <Show when=move || {
                            matches!(
                                update_state.get(),
                                UpdateState::Available(_)
                                    | UpdateState::Installing(_)
                                    | UpdateState::ReadyToRelaunch(_)
                            )
                        }>
                            <span
                                class="pointer-events-none absolute top-1.5 right-1.5 h-2 w-2 rounded-full bg-accent"
                                title="Update available"
                            ></span>
                        </Show>
                    </div>
                </div>

                {
                    let trigger_relaunch = trigger_relaunch.clone();
                    move || {
                        if !open.get() {
                            return view! {}.into_view();
                        }
                        let trigger_relaunch = trigger_relaunch.clone();

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

                                    <div class="h-px bg-border/70 my-3"></div>

                                    <div class="flex items-center justify-between gap-2">
                                        <span class=styles::SETTINGS_LABEL>"Version"</span>
                                        <div class="flex items-center gap-2">
                                            <span class="text-xs text-subtle font-mono">
                                                {concat!("v", env!("CARGO_PKG_VERSION"))}
                                            </span>
                                            {move || {
                                                if !is_tauri {
                                                    return view! {}.into_view();
                                                }
                                                let trigger_relaunch = trigger_relaunch.clone();
                                                match update_state.get() {
                                                    UpdateState::Installing(_) => {
                                                        view! {
                                                            <button
                                                                class=styles::SETTINGS_UPDATE_BTN_ACTIVE
                                                                disabled=true
                                                                title="Installing update"
                                                            >
                                                                "Installing…"
                                                            </button>
                                                        }
                                                            .into_view()
                                                    }
                                                    UpdateState::ReadyToRelaunch(version) => {
                                                        let title =
                                                            format!("Relaunch into Inskea v{version}");
                                                        view! {
                                                            <button
                                                                class=styles::SETTINGS_UPDATE_BTN_ACTIVE
                                                                title=title
                                                                on:click=move |_| trigger_relaunch()
                                                            >
                                                                "Relaunch"
                                                            </button>
                                                        }
                                                            .into_view()
                                                    }
                                                    UpdateState::Available(info) => {
                                                        let label = if info.can_install {
                                                            format!("Install v{}", info.latest_version)
                                                        } else {
                                                            format!("Get v{}", info.latest_version)
                                                        };
                                                        let title = if info.can_install {
                                                            format!(
                                                                "Download and install Inskea v{}",
                                                                info.latest_version
                                                            )
                                                        } else {
                                                            format!(
                                                                "Open the Inskea v{} release page",
                                                                info.latest_version
                                                            )
                                                        };
                                                        view! {
                                                            <button
                                                                class=styles::SETTINGS_UPDATE_BTN_ACTIVE
                                                                title=title
                                                                on:click=move |_| run_update_install(info.clone())
                                                            >
                                                                {label}
                                                            </button>
                                                        }
                                                            .into_view()
                                                    }
                                                    UpdateState::Idle
                                                    | UpdateState::Checking
                                                    | UpdateState::UpToDate
                                                    | UpdateState::Failed => {
                                                        if manual_checked.get() {
                                                            view! {}.into_view()
                                                        } else {
                                                            view! {
                                                                <button
                                                                    class=styles::SETTINGS_UPDATE_BTN
                                                                    title="Check for updates"
                                                                    on:click=move |_| run_update_check(true)
                                                                >
                                                                    "Check for updates"
                                                                </button>
                                                            }
                                                                .into_view()
                                                        }
                                                    }
                                                }
                                            }}
                                        </div>
                                    </div>
                                </div>
                            </>
                        }
                            .into_view()
                    }
                }
            </div>
        </div>

        <Show when=move || relaunch_prompt.get().is_some()>
            {
                let trigger_relaunch = trigger_relaunch.clone();
                let version = move || relaunch_prompt.get().unwrap_or_default();
                view! {
                    <div class="fixed inset-0 z-[60] bg-black/40"></div>
                    <div class="fixed inset-0 z-[61] grid place-items-center">
                        <div class=format!("{} w-80 p-5", styles::PANEL) role="dialog" aria-modal="true">
                            <h2 class="text-sm font-medium text-fg">"Update ready"</h2>
                            <p class="mt-2 text-xs text-subtle">
                                "Inskea v" {version} " has been installed. Relaunch now to switch to the updated version?"
                            </p>
                            <div class="mt-5 flex justify-end gap-2">
                                <button
                                    class=styles::BTN_TEXT
                                    on:click=move |_| relaunch_prompt.set(None)
                                >
                                    "Later"
                                </button>
                                <button
                                    class=styles::BTN_TEXT_ACTIVE
                                    autofocus=true
                                    on:click=move |_| trigger_relaunch()
                                >
                                    "Relaunch"
                                </button>
                            </div>
                        </div>
                    </div>
                }
            }
        </Show>
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
