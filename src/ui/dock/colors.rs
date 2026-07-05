use crate::model::ShapeColor;
use crate::ui::classes;
use leptos::*;

/// Vertical panel of colour swatches representing the ShapeColor palette.
///
/// The selected colour gets a ring-accent border. Each swatch is rendered
/// as a small coloured square button.
#[component]
pub fn ColorsPanel(selected_color: RwSignal<ShapeColor>) -> impl IntoView {
    let colors = [
        ShapeColor::Red,
        ShapeColor::Orange,
        ShapeColor::Yellow,
        ShapeColor::Green,
        ShapeColor::Cyan,
        ShapeColor::Blue,
        ShapeColor::Purple,
        ShapeColor::White,
    ];

    view! {
        <div class="flex flex-col gap-1 p-2">
            {colors
                .into_iter()
                .map(|c| {
                    let hex = c.to_hex();
                    let label = format!("{c:?}");
                    let c_for_click = c.clone();
                    let c_for_sel = c;
                    view! {
                        <button
                            class=move || {
                                if selected_color.get() == c_for_sel {
                                    classes::BTN_SWATCH_SEL
                                } else {
                                    classes::BTN_SWATCH_OFF
                                }
                            }
                            style=format!("background-color: {hex}")
                            title=label
                            on:click=move |_| selected_color.set(c_for_click.clone())
                        />
                    }
                })
                .collect_view()}
        </div>
    }
}
