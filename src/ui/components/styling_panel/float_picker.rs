use crate::canvas::settings::CanvasSettings;
use crate::ui::components::styling_panel::color::ColorSlot;
use leptos::*;
use leptos_color::components::color_picker::ColorPicker as LpcCp;
use leptos_color::theme::Theme;
use leptos_color::Color as LpcColor;

pub fn float_picker(
    active_slot: RwSignal<Option<ColorSlot>>,
    picker_color: Signal<LpcColor>,
    picker_on_change: Callback<LpcColor>,
    picker_ref: NodeRef<html::Div>,
    settings: RwSignal<CanvasSettings>,
) -> impl IntoView {
    view! {
        <Show when=move || active_slot.get().is_some()>
            <div class="absolute left-full top-1/2 -translate-y-1/2 ml-4" style="z-index: 9999;">
                <div node_ref=picker_ref>
                    {
                        move || {
                            let p = settings.get().theme.palette();
                            let lpc_theme = Theme::custom(
                                LpcColor::from_html(p.panel).unwrap(),
                                LpcColor::from_html(p.surface).unwrap(),
                                LpcColor::from_html(p.fg).unwrap(),
                                LpcColor::from_html(p.border).unwrap(),
                                "8px".to_string(),
                                "0 4px 12px rgba(0,0,0,0.3)".to_string(),
                                // rem so the picker tracks the responsive UI scale.
                                "17.5rem".to_string(),
                            );
                            view! {
                                <LpcCp
                                    color=picker_color
                                    on_change=picker_on_change
                                    hide_alpha=true
                                    theme=lpc_theme
                                />
                            }
                        }
                    }
                </div>
            </div>
        </Show>
    }
}
