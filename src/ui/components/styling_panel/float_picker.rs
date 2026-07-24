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
) -> impl IntoView {
    view! {
        <Show when=move || active_slot.get().is_some()>
            <div class="absolute left-full top-1/2 -translate-y-1/2 ml-4" style="z-index: 9999;">
                <div node_ref=picker_ref>
                    {
                        let lpc_theme = Theme::custom(
                            LpcColor::from_html("#24283b").unwrap(),
                            LpcColor::from_html("#1e1f2e").unwrap(),
                            LpcColor::from_html("#a9b1d6").unwrap(),
                            LpcColor::from_html("#3b4261").unwrap(),
                            "8px".to_string(),
                            "0 4px 12px rgba(0,0,0,0.3)".to_string(),
                            "280px".to_string(),
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
                </div>
            </div>
        </Show>
    }
}