//! SVG icon components used throughout the UI.
//!
/// The source files live in `src/ui/icons/` and come from Lucide 1.59.0
/// (ISC, see `src/ui/icons/LICENSE`). They are inlined at compile time and
/// rendered as outline-only SVGs that inherit the parent text colour.
use leptos::*;

const DEFAULT_CLASS: &str = "w-5 h-5";

/// Extract the markup inside the downloaded `<svg>` wrapper. The files carry
/// a leading license comment, so the opening tag must be located first.
fn inner(markup: &'static str) -> &'static str {
    let Some(svg) = markup.find("<svg") else {
        return "";
    };
    let Some(tag_end) = markup[svg..].find('>') else {
        return "";
    };
    let start = svg + tag_end + 1;
    let end = markup.rfind("</svg>").unwrap_or(markup.len());
    markup.get(start..end).unwrap_or("")
}

fn svg(markup: &'static str, class_name: &'static str) -> impl IntoView {
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class=class_name
            aria-hidden="true"
            focusable="false"
            inner_html=inner(markup)
        ></svg>
    }
}

macro_rules! lucide_icons {
    ($($name:ident => $file:literal),* $(,)?) => {
        $(
            pub fn $name() -> impl IntoView {
                svg(include_str!(concat!("icons/", $file)), DEFAULT_CLASS)
            }
        )*
    };
}

lucide_icons! {
    // Tools and canvas modes.
    pencil => "pencil.svg",
    cursor => "mouse-pointer-2.svg",
    hand => "hand.svg",
    eraser => "eraser.svg",
    rect => "square.svg",
    ellipse => "ellipse.svg",
    line => "slash.svg",
    arrow => "arrow-up-right.svg",
    text => "type.svg",
    freehand => "line-squiggle.svg",

    // Toolbar and menu.
    home => "house.svg",
    undo => "undo-2.svg",
    redo => "redo-2.svg",
    gear => "settings.svg",
    menu => "menu.svg",
    close => "x.svg",
    check => "check.svg",
    chevron_right => "chevron-right.svg",

    // Edit, clipboard, and layers.
    copy => "copy.svg",
    duplicate => "copy-plus.svg",
    paste => "clipboard.svg",
    send_to_back => "send-to-back.svg",
    send_backward => "arrow-down.svg",
    bring_forward => "arrow-up.svg",
    bring_to_front => "bring-to-front.svg",

    // Element alignment.
    align_left => "align-horizontal-justify-start.svg",
    align_center_h => "align-horizontal-justify-center.svg",
    align_right => "align-horizontal-justify-end.svg",
    align_top => "align-vertical-justify-start.svg",
    align_center_v => "align-vertical-justify-center.svg",
    align_bottom => "align-vertical-justify-end.svg",
    distribute_horizontal => "align-horizontal-distribute-center.svg",
    distribute_vertical => "align-vertical-distribute-center.svg",

    // Text alignment.
    text_align_left => "align-left.svg",
    text_align_center => "align-center.svg",
    text_align_right => "align-right.svg",

    // Grouping, line curvature, and edge styles.
    group => "group.svg",
    ungroup => "ungroup.svg",
    line_straight => "slash.svg",
    line_curved => "spline.svg",
    edge_sharp => "square.svg",
    edge_rounded => "squircle.svg",
}

/// Connector routing previews: a node at each end plus the route between them.
pub fn connector_straight() -> impl IntoView {
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class=DEFAULT_CLASS
            aria-hidden="true"
            focusable="false"
        >
            <rect x="3" y="9" width="4" height="6" rx="1" fill="currentColor" stroke="none" />
            <rect x="17" y="9" width="4" height="6" rx="1" fill="currentColor" stroke="none" />
            <path d="M7 12 H17" />
        </svg>
    }
}

pub fn connector_curved() -> impl IntoView {
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class=DEFAULT_CLASS
            aria-hidden="true"
            focusable="false"
        >
            <rect x="3" y="9" width="4" height="6" rx="1" fill="currentColor" stroke="none" />
            <rect x="17" y="9" width="4" height="6" rx="1" fill="currentColor" stroke="none" />
            <path d="M7 12 C10 7 14 17 17 12" />
        </svg>
    }
}

pub fn connector_direct() -> impl IntoView {
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class=DEFAULT_CLASS
            aria-hidden="true"
            focusable="false"
        >
            <rect x="3" y="9" width="4" height="6" rx="1" fill="currentColor" stroke="none" />
            <rect x="17" y="9" width="4" height="6" rx="1" fill="currentColor" stroke="none" />
            <path d="M7 12 H12 V7 H17" />
        </svg>
    }
}

/// Small chevrons for the number-slider stepper.
pub fn chevron_up() -> impl IntoView {
    svg(include_str!("icons/chevron-up.svg"), "w-3 h-3")
}

pub fn chevron_down() -> impl IntoView {
    svg(include_str!("icons/chevron-down.svg"), "w-3 h-3")
}

/// Palette mark used by the theme picker.
pub fn palette() -> impl IntoView {
    svg(include_str!("icons/palette.svg"), "w-4 h-4")
}

#[cfg(test)]
mod tests {
    use super::inner;

    #[test]
    fn extracts_paths_without_the_svg_wrapper() {
        let inner = inner(include_str!("icons/pencil.svg"));
        assert!(inner.contains("<path"));
        assert!(!inner.contains("<svg"));
        assert!(!inner.contains("</svg>"));
    }
}
