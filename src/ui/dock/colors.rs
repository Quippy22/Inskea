use crate::model::Color;
use crate::ui::classes;
use leptos::*;

const COLOR_LIST: &[&str] = &[
    Color::RED,
    Color::ORANGE,
    Color::YELLOW,
    Color::GREEN,
    Color::CYAN,
    Color::BLUE,
    Color::PURPLE,
    Color::WHITE,
];

/// Vertical panel of colour swatches.
#[component]
pub fn ColorsPanel(selected_color: RwSignal<Color>) -> impl IntoView {
    view! {
        <div class="flex flex-col gap-1 p-2">
            {COLOR_LIST
                .iter()
                .map(|&hex| {
                    let label = hex.to_string();
                    let hex_for_style = hex.to_string();
                    let hex_for_sel = hex.to_string();
                    let c_for_click = Color::new(hex);
                    view! {
                        <button
                            class=move || {
                                if selected_color.get() == Color::new(&hex_for_sel) {
                                    classes::BTN_SWATCH_SEL
                                } else {
                                    classes::BTN_SWATCH_OFF
                                }
                            }
                            style=format!("background-color: {hex_for_style}")
                            title=label
                            on:click=move |_| selected_color.set(c_for_click.clone())
                        />
                    }
                })
                .collect_view()}
        </div>
    }
}
