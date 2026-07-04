use leptos::*;

pub fn palette() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="w-5 h-5">
            <circle cx="13.5" cy="6.5" r="0.5" fill="currentColor"/>
            <circle cx="17.5" cy="10.5" r="0.5" fill="currentColor"/>
            <circle cx="8.5" cy="7.5" r="0.5" fill="currentColor"/>
            <circle cx="6.5" cy="12.5" r="0.5" fill="currentColor"/>
            <circle cx="12" cy="12" r="10"/>
        </svg>
    }
}

pub fn pencil() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="w-5 h-5">
            <path d="M17 3a2.85 2.85 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z"/>
            <path d="m15 5 4 4"/>
        </svg>
    }
}

pub fn chevron_left() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="w-5 h-5">
            <polyline points="15 18 9 12 15 6"/>
        </svg>
    }
}

pub fn chevron_right() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="w-5 h-5">
            <polyline points="9 18 15 12 9 6"/>
        </svg>
    }
}

pub fn rect() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="w-5 h-5">
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
        </svg>
    }
}

pub fn ellipse() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="w-5 h-5">
            <ellipse cx="12" cy="12" rx="10" ry="8"/>
        </svg>
    }
}

pub fn line() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="w-5 h-5">
            <line x1="4" y1="20" x2="20" y2="4"/>
        </svg>
    }
}

pub fn arrow() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="w-5 h-5">
            <line x1="4" y1="20" x2="20" y2="4"/>
            <polyline points="14 4 20 4 20 10"/>
        </svg>
    }
}

pub fn text() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="w-5 h-5">
            <polyline points="4 7 4 4 20 4 20 7"/>
            <line x1="12" y1="4" x2="12" y2="20"/>
            <line x1="8" y1="20" x2="16" y2="20"/>
        </svg>
    }
}

pub fn freehand() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="w-5 h-5">
            <path d="M3 17c3-2 5-6 7-6s4 4 6 4 4-3 5-4"/>
        </svg>
    }
}
