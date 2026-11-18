//! What's New modal dialog displaying the changelog and release highlights
//! embedded from `updates.md` at compile time.

use crate::ui::icon;
use crate::ui::styles;
use leptos::*;

const UPDATES_MD: &str = include_str!("../../updates.md");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseUpdate {
    pub version: String,
    pub is_current: bool,
    pub features: Vec<UpdateFeature>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateFeature {
    pub title: String,
    pub bullets: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum InlineSegment {
    Plain(String),
    Bold(String),
    Code(String),
}

fn parse_inline_formatting(input: &str) -> Vec<InlineSegment> {
    let mut segments = Vec::new();
    let mut rest = input;

    while !rest.is_empty() {
        let bold_pos = rest.find("**");
        let code_pos = rest.find('`');

        match (bold_pos, code_pos) {
            (Some(b), Some(c)) if b < c => {
                if b > 0 {
                    segments.push(InlineSegment::Plain(rest[..b].to_string()));
                }
                let after_open = &rest[b + 2..];
                if let Some(end) = after_open.find("**") {
                    segments.push(InlineSegment::Bold(after_open[..end].to_string()));
                    rest = &after_open[end + 2..];
                } else {
                    segments.push(InlineSegment::Plain(rest[b..].to_string()));
                    break;
                }
            }
            (Some(_), Some(c)) => {
                if c > 0 {
                    segments.push(InlineSegment::Plain(rest[..c].to_string()));
                }
                let after_open = &rest[c + 1..];
                if let Some(end) = after_open.find('`') {
                    segments.push(InlineSegment::Code(after_open[..end].to_string()));
                    rest = &after_open[end + 1..];
                } else {
                    segments.push(InlineSegment::Plain(rest[c..].to_string()));
                    break;
                }
            }
            (Some(b), None) => {
                if b > 0 {
                    segments.push(InlineSegment::Plain(rest[..b].to_string()));
                }
                let after_open = &rest[b + 2..];
                if let Some(end) = after_open.find("**") {
                    segments.push(InlineSegment::Bold(after_open[..end].to_string()));
                    rest = &after_open[end + 2..];
                } else {
                    segments.push(InlineSegment::Plain(rest[b..].to_string()));
                    break;
                }
            }
            (None, Some(c)) => {
                if c > 0 {
                    segments.push(InlineSegment::Plain(rest[..c].to_string()));
                }
                let after_open = &rest[c + 1..];
                if let Some(end) = after_open.find('`') {
                    segments.push(InlineSegment::Code(after_open[..end].to_string()));
                    rest = &after_open[end + 1..];
                } else {
                    segments.push(InlineSegment::Plain(rest[c..].to_string()));
                    break;
                }
            }
            (None, None) => {
                segments.push(InlineSegment::Plain(rest.to_string()));
                break;
            }
        }
    }

    segments
}

fn render_formatted_line(text: &str) -> impl IntoView {
    let segments = parse_inline_formatting(text);
    view! {
        <span>
            {segments.into_iter().map(|seg| match seg {
                InlineSegment::Plain(s) => view! { <span>{s}</span> }.into_view(),
                InlineSegment::Bold(s) => view! { <strong class="font-semibold text-fg">{s}</strong> }.into_view(),
                InlineSegment::Code(s) => view! {
                    <code class="px-1 py-0.5 rounded bg-surface/70 font-mono text-[11px] text-accent">
                        {s}
                    </code>
                }.into_view(),
            }).collect::<Vec<_>>()}
        </span>
    }
}

pub fn parse_updates(content: &str, current_version: &str) -> Vec<ReleaseUpdate> {
    let mut updates = Vec::new();
    let mut current_release: Option<ReleaseUpdate> = None;
    let mut current_feature: Option<UpdateFeature> = None;

    let normalized_curr = current_version.trim_start_matches('v');

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("## ") {
            if let Some(feat) = current_feature.take() {
                if let Some(rel) = current_release.as_mut() {
                    rel.features.push(feat);
                }
            }
            if let Some(rel) = current_release.take() {
                updates.push(rel);
            }
            let version = rest.trim().to_string();
            let is_current = version.trim_start_matches('v') == normalized_curr;
            current_release = Some(ReleaseUpdate {
                version,
                is_current,
                features: Vec::new(),
            });
        } else if let Some(rest) = line.strip_prefix("### ") {
            if let Some(feat) = current_feature.take() {
                if let Some(rel) = current_release.as_mut() {
                    rel.features.push(feat);
                }
            }
            current_feature = Some(UpdateFeature {
                title: rest.trim().to_string(),
                bullets: Vec::new(),
            });
        } else if let Some(bullet) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            if let Some(feat) = current_feature.as_mut() {
                feat.bullets.push(bullet.trim().to_string());
            }
        } else if !line.starts_with('#') {
            if let Some(feat) = current_feature.as_mut() {
                feat.bullets.push(line.to_string());
            }
        }
    }

    if let Some(feat) = current_feature {
        if let Some(rel) = current_release.as_mut() {
            rel.features.push(feat);
        }
    }
    if let Some(rel) = current_release {
        updates.push(rel);
    }

    updates
}

#[component]
pub fn WhatsNewModal(open: RwSignal<bool>) -> impl IntoView {
    let current_version = env!("CARGO_PKG_VERSION");
    let updates = parse_updates(UPDATES_MD, current_version);
    let close = move || open.set(false);

    move || {
        if open.get() {
            let close = close;
            view! {
                <>
                    <div
                        class="fixed inset-0 z-40 bg-black/50 backdrop-blur-sm"
                        on:click=move |_| close()
                    ></div>
                    <div class="fixed inset-0 z-50 grid place-items-center pointer-events-none p-4">
                        <div
                            class="relative pointer-events-auto rounded-xl bg-panel/95 backdrop-blur-md border border-border shadow-2xl max-w-2xl w-full max-h-[85vh] flex flex-col overflow-hidden select-none"
                            role="dialog"
                            aria-modal="true"
                            aria-label="What's New in Inskea"
                        >
                            // Header
                            <div class="px-6 pt-5 pb-4 border-b border-border flex items-center justify-between">
                                <div class="flex items-center gap-3">
                                    <div class="flex items-center justify-center w-8 h-8 rounded-lg bg-accent/15 text-accent">
                                        {icon::book_open()}
                                    </div>
                                    <div>
                                        <h2 class="text-base font-semibold text-fg">"What's New in Inskea"</h2>
                                        <div class="flex items-center gap-2 mt-0.5">
                                            <span class="text-xs text-subtle font-mono">
                                                {format!("v{}", current_version)}
                                            </span>
                                            <span class="px-1.5 py-0.2 rounded text-[10px] font-medium bg-accent/20 text-accent">
                                                "Current"
                                            </span>
                                        </div>
                                    </div>
                                </div>
                                <button
                                    class="text-subtle hover:text-fg p-1 rounded-md hover:bg-surface/50 transition-colors"
                                    on:click=move |_| close()
                                    title="Close"
                                >
                                    {icon::close()}
                                </button>
                            </div>

                            // Scrollable updates list
                            <div class="p-6 overflow-y-auto space-y-6 flex-1 text-sm select-text">
                                {updates.iter().map(|rel| {
                                    let is_current = rel.is_current;
                                    let ver = rel.version.clone();
                                    view! {
                                        <div class="space-y-4">
                                            <div class="flex items-center gap-2 pb-1 border-b border-border/60">
                                                <span class="font-mono font-semibold text-sm text-fg">
                                                    {ver}
                                                </span>
                                                {if is_current {
                                                    view! {
                                                        <span class="px-1.5 py-0.5 rounded text-[10px] font-semibold bg-accent/20 text-accent uppercase tracking-wider">
                                                            "Latest"
                                                        </span>
                                                    }.into_view()
                                                } else {
                                                    view! {}.into_view()
                                                }}
                                            </div>

                                            <div class="space-y-4 pl-1">
                                                {rel.features.iter().map(|feat| {
                                                    let title = feat.title.clone();
                                                    let bullets = feat.bullets.clone();
                                                    view! {
                                                        <div class="space-y-1.5">
                                                            <h4 class="text-xs font-semibold text-fg flex items-center gap-2">
                                                                <span class="w-1.5 h-1.5 rounded-full bg-accent shrink-0"></span>
                                                                <span>{render_formatted_line(&title)}</span>
                                                            </h4>
                                                            <div class="space-y-1 pl-3.5">
                                                                {bullets.iter().map(|b| {
                                                                    let bullet_text = b.clone();
                                                                    view! {
                                                                        <p class="text-xs text-subtle leading-relaxed">
                                                                            {render_formatted_line(&bullet_text)}
                                                                        </p>
                                                                    }
                                                                }).collect::<Vec<_>>()}
                                                            </div>
                                                        </div>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>

                            // Footer
                            <div class="px-6 py-3 border-t border-border bg-surface/20 flex items-center justify-between">
                                <a
                                    href="https://github.com/Quippy22/Inskea/blob/main/updates.md"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    class="text-xs text-subtle hover:text-accent transition-colors flex items-center gap-1"
                                >
                                    <span>"Changelog on GitHub"</span>
                                </a>
                                <button
                                    class=styles::BTN_TEXT_ACTIVE
                                    on:click=move |_| close()
                                >
                                    "Got it"
                                </button>
                            </div>
                        </div>
                    </div>
                </>
            }.into_view()
        } else {
            view! {}.into_view()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_updates_markdown_correctly() {
        let md = r#"
# What's New

## v3.1.1
### Fast Image Rendering
- Blob URL caching: In-memory **Blob URLs** prevent lag.
- Downscaled `raster` decodes smoothly.

### Export Scopes
- Fine-grained export options.

## v3.1.0
### Snippet Catalogue
- Reusable snippet storage.
"#;
        let updates = parse_updates(md, "3.1.1");
        assert_eq!(updates.len(), 2);

        let u0 = &updates[0];
        assert_eq!(u0.version, "v3.1.1");
        assert!(u0.is_current);
        assert_eq!(u0.features.len(), 2);
        assert_eq!(u0.features[0].title, "Fast Image Rendering");
        assert_eq!(u0.features[0].bullets.len(), 2);
        assert_eq!(u0.features[1].title, "Export Scopes");

        let u1 = &updates[1];
        assert_eq!(u1.version, "v3.1.0");
        assert!(!u1.is_current);
        assert_eq!(u1.features.len(), 1);
        assert_eq!(u1.features[0].title, "Snippet Catalogue");
    }

    #[test]
    fn parses_inline_formatting() {
        let segs = parse_inline_formatting("Hello **world** and `code` test");
        assert_eq!(
            segs,
            vec![
                InlineSegment::Plain("Hello ".into()),
                InlineSegment::Bold("world".into()),
                InlineSegment::Plain(" and ".into()),
                InlineSegment::Code("code".into()),
                InlineSegment::Plain(" test".into()),
            ]
        );
    }

    #[test]
    fn embedded_updates_md_is_valid() {
        let current_version = env!("CARGO_PKG_VERSION");
        let updates = parse_updates(UPDATES_MD, current_version);
        assert!(!updates.is_empty(), "Embedded updates.md must contain releases");
        assert!(
            updates.iter().any(|u| u.is_current),
            "Embedded updates.md must contain an entry for current version {current_version}"
        );
    }
}
