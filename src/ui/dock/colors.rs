use crate::model::ShapeColor;
use leptos::*;

#[component]
pub fn ColorsPanel() -> impl IntoView {
    let selected = create_rw_signal(ShapeColor::White);

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
            {colors.into_iter().map(|c| {
                let hex = c.to_hex();
                let label = format!("{c:?}");
                let c_for_click = c.clone();
                let c_for_sel = c;
                view! {
                    <button
                        class=move || {
                            let base = "w-7 h-7 rounded-md border transition-transform hover:scale-110";
                            if selected.get() == c_for_sel {
                                format!("{base} border-accent ring-2 ring-accent/50")
                            } else {
                                format!("{base} border-border")
                            }
                        }
                        style=format!("background-color: {hex}")
                        title=label
                        on:click=move |_| selected.set(c_for_click.clone())
                    />
                }
            }).collect_view()}
        </div>
    }
}
