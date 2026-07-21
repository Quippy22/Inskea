use leptos::*;

/// A button that renders whatever children it wraps (typically an SVG icon).
/// Replaces the repetitive `button/icon/on:click` pattern used in settings
/// collapse toggles, dock headers, etc.
#[component]
pub fn IconButton<F>(
    /// Click handler.
    on_click: F,
    /// ARIA tooltip.
    title: &'static str,
    /// Extra Tailwind classes (e.g. `styles::BTN_COLLAPSE`).
    class: &'static str,
    /// The icon (or any content) rendered inside the button.
    children: Children,
) -> impl IntoView
where
    F: Fn() + 'static,
{
    view! {
        <button class=class on:click=move |_| on_click() title=title>
            {children()}
        </button>
    }
}
