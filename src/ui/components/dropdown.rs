use crate::ui::classes;
use leptos::*;
use std::rc::Rc;

/// A single item inside a [`Dropdown`].
#[derive(Clone)]
pub enum DropdownItem {
    /// Clickable row with a label.
    Action {
        label: &'static str,
        on_click: Rc<dyn Fn()>,
    },
    /// Non-clickable header label (e.g. "PNG", "SVG").
    Header { label: &'static str },
    /// Horizontal separator line.
    Separator,
}

/// A submenu panel rendered at `left: 100%; top: -4px` relative to the
/// trigger row.  The parent is responsible for showing/hiding via the
/// `show` signal and for feeding back the mouse-enter/leave state so the
/// panel stays open while hovered.
#[component]
pub fn Dropdown(
    /// When `true` the panel is rendered.
    show: Signal<bool>,
    /// Called when the pointer enters the panel.
    on_mouseenter: Rc<dyn Fn()>,
    /// Called when the pointer leaves the panel.
    on_mouseleave: Rc<dyn Fn()>,
    /// Items to render inside the panel.
    items: Vec<DropdownItem>,
    /// Optional extra CSS classes beyond `classes::MENU_DROPDOWN`.
    #[prop(optional)]
    _extra_class: &'static str,
) -> impl IntoView {
    move || {
        if !show.get() {
            return view! {}.into_view();
        }

        let me = on_mouseenter.clone();
        let ml = on_mouseleave.clone();

        view! {
            <div
                class=classes::MENU_DROPDOWN
                style="left: 100%; top: -4px;"
                on:mouseenter=move |_| me()
                on:mouseleave=move |_| ml()
            >
                {items.iter().map(|item| {
                    match item {
                        DropdownItem::Action { label, on_click } => {
                            let cb = on_click.clone();
                            view! {
                                <button class=classes::MENU_ITEM on:click=move |_| (cb)()>
                                    {*label}
                                </button>
                            }.into_view()
                        }
                        DropdownItem::Header { label } => {
                            view! {
                                <span class="block px-4 py-1 text-xs text-subtle">
                                    {*label}
                                </span>
                            }.into_view()
                        }
                        DropdownItem::Separator => {
                            view! {
                                <div class="h-px bg-border my-1"></div>
                            }.into_view()
                        }
                    }
                }).collect_view()}
            </div>
        }
        .into_view()
    }
}
