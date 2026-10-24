use leptos::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, PartialEq)]
pub enum NoticeKind {
    Info,
    Error,
}

#[derive(Clone, PartialEq)]
pub struct Notice {
    pub id: u64,
    pub kind: NoticeKind,
    pub message: String,
}

pub type Notifications = RwSignal<Vec<Notice>>;

/// Show a transient message in the bottom-centre notice stack.
pub fn notify(notifications: Notifications, kind: NoticeKind, message: impl Into<String>) {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    notifications.update(|list| {
        list.push(Notice {
            id,
            kind,
            message: message.into(),
        });
        if list.len() > 4 {
            list.remove(0);
        }
    });
    set_timeout(
        move || notifications.update(|list| list.retain(|notice| notice.id != id)),
        Duration::from_secs(5),
    );
}

pub fn info(notifications: Notifications, message: impl Into<String>) {
    notify(notifications, NoticeKind::Info, message);
}

pub fn error(notifications: Notifications, message: impl Into<String>) {
    notify(notifications, NoticeKind::Error, message);
}

#[component]
pub fn Toasts(notifications: Notifications) -> impl IntoView {
    view! {
        <div class="fixed bottom-4 left-1/2 -translate-x-1/2 z-[70] flex flex-col items-center gap-2 pointer-events-none">
            <For each=move || notifications.get() key=|notice| notice.id let:notice>
                {
                    let class = match notice.kind {
                        NoticeKind::Info => "rounded-lg border border-border bg-panel/95 px-3 py-2 text-xs text-fg shadow-lg backdrop-blur-sm",
                        NoticeKind::Error => "rounded-lg border border-red/60 bg-panel/95 px-3 py-2 text-xs text-red shadow-lg backdrop-blur-sm",
                    };
                    view! { <div class=class>{notice.message}</div> }
                }
            </For>
        </div>
    }
}
