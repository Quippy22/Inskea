use crate::ui::classes;
use leptos::*;

#[component]
pub fn SegmentedControl<T: PartialEq + Copy + 'static>(
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
