use crate::document::PendingAction;
use crate::ui::styles;
use leptos::*;

/// Confirmation shown before an action would discard unsaved edits.
#[component]
pub fn UnsavedChangesDialog(
    pending: RwSignal<Option<PendingAction>>,
    on_save: Callback<()>,
    on_discard: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let label = move || {
        pending
            .get()
            .map(|action| action.label)
            .unwrap_or("continue")
    };

    view! {
        <Show when=move || pending.get().is_some()>
            <div class="fixed inset-0 z-[80] bg-black/40"></div>
            <div class="fixed inset-0 z-[81] grid place-items-center">
                <div
                    class=format!("{} w-80 p-5", styles::PANEL)
                    role="dialog"
                    aria-modal="true"
                >
                    <h2 class="text-sm font-medium text-fg">"Unsaved changes"</h2>
                    <p class="mt-2 text-xs text-subtle">
                        "Save your drawing before you " {label} "?"
                    </p>
                    <div class="mt-5 flex justify-end gap-2">
                        <button class=styles::BTN_TEXT on:click=move |_| on_cancel.call(())>
                            "Cancel"
                        </button>
                        <button class=styles::BTN_TEXT on:click=move |_| on_discard.call(())>
                            "Discard"
                        </button>
                        <button class=styles::BTN_TEXT_ACTIVE on:click=move |_| on_save.call(())>
                            "Save"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
