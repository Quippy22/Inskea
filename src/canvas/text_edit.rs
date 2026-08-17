use super::viewport::Viewport;
use crate::model::elements::text::FONT_FAMILY;
use crate::model::{Element, ElementId, Point, Scene};
use leptos::*;
use std::rc::Rc;

/// Create the commit function that saves textarea content back to the element.
pub fn make_commit_edit(
    editing_id: RwSignal<Option<ElementId>>,
    edit_text: RwSignal<String>,
    scene: RwSignal<Scene>,
    textarea_ref: NodeRef<leptos::html::Textarea>,
    viewport: RwSignal<Viewport>,
) -> Rc<dyn Fn()> {
    Rc::new(move || {
        if let Some(id) = editing_id.get() {
            let text = edit_text.get();
            scene.update(|s| {
                if text.is_empty() {
                    s.remove_by_id(id);
                } else if let Some(Element::Text(text_elem)) = s.element_by_id_mut(id) {
                    let zoom = viewport.get().zoom;
                    let (w, h) = textarea_ref
                        .get()
                        .map(|ta| {
                            (
                                ta.client_width() as f64 / zoom,
                                ta.scroll_height() as f64 / zoom,
                            )
                        })
                        .unwrap_or((text_elem.data.width, text_elem.data.height));
                    text_elem.set_content(&text);
                    text_elem.data.width = w;
                    text_elem.data.height = h;
                }
            });
            editing_id.set(None);
            edit_text.set(String::new());
        }
    })
}

/// Returns a closure that renders the invisible text-editing textarea overlay.
/// No border or background — only the caret and the typed text are visible.
pub fn text_edit_overlay(
    editing_id: RwSignal<Option<ElementId>>,
    edit_text: RwSignal<String>,
    textarea_ref: NodeRef<leptos::html::Textarea>,
    scene: RwSignal<Scene>,
    viewport: RwSignal<Viewport>,
    screen_size: RwSignal<Point>,
    commit: Rc<dyn Fn()>,
) -> impl Fn() -> Option<View> {
    move || {
        let id = editing_id.get()?;
        let text_elem = scene.with(|s| {
            s.element_by_id(id).and_then(|e| {
                if let Element::Text(t) = e {
                    Some(t.clone())
                } else {
                    None
                }
            })
        })?;
        let zoom = viewport.get().zoom;
        let font_size = text_elem.data.style.font_size.max(12.0);
        let ss = screen_size.get();
        let screen_pos = viewport.get().world_to_screen(
            Point::new(text_elem.data.world_point.x, text_elem.data.world_point.y),
            ss,
        );
        let (sx, sy) = (screen_pos.x, screen_pos.y);
        let fill = text_elem
            .data
            .style
            .fill_color
            .as_ref()
            .map(|c| c.to_hex())
            .unwrap_or_else(|| text_elem.data.style.stroke_color.to_hex());

        let ta_w = if text_elem.data.width > 0.0 {
            text_elem.data.width * zoom
        } else {
            font_size * zoom
        };
        let ta_h = if text_elem.data.height > 0.0 {
            text_elem.data.height * zoom
        } else {
            font_size * zoom * 1.2
        };

        let ce_blur = commit.clone();
        let ce_esc = commit.clone();

        Some(
            view! {
                <textarea
                    autofocus="true"
                    _ref=textarea_ref
                    on:focus=move |ev| {
                        let ta = event_target::<web_sys::HtmlTextAreaElement>(&ev);
                        let ta2 = ta.clone();
                        leptos::leptos_dom::helpers::set_timeout(
                            move || {
                                let len = ta2.value().len() as u32;
                                let _ = ta2.set_selection_start(Some(len));
                                let _ = ta2.set_selection_end(Some(len));
                            },
                            std::time::Duration::ZERO,
                        );
                    }
                    style=format!(
                        "position:fixed;left:{}px;top:{}px;\
                         width:{}px;height:{}px;\
                         font-size:{}px;font-family:{};\
                         color:{};\
                         background:transparent;\
                         border:none;\
                         outline:none;resize:none;\
                         overflow:hidden;\
                         white-space:pre;overflow-wrap:normal;\
                         padding:0;margin:0;z-index:100;\
                         line-height:1.2;",
                        sx,
                        sy,
                        ta_w,
                        ta_h,
                        font_size * zoom,
                        FONT_FAMILY,
                        fill,
                    )
                    prop:value=move || edit_text.get()
                    on:input=move |ev| {
                        edit_text.set(event_target_value(&ev));
                        let ta = event_target::<web_sys::HtmlTextAreaElement>(&ev);
                        let _ = ta
                            .style()
                            .set_property("width", &format!("{}px", ta.scroll_width().max(1)));
                    }
                    on:blur=move |_| ce_blur()
                    on:keydown=move |ev: ev::KeyboardEvent| {
                        if ev.key() == "Escape" {
                            ce_esc();
                        }
                        if ev.key() == "Tab" {
                            ev.prevent_default();
                            let target = event_target::<web_sys::HtmlTextAreaElement>(&ev);
                            let cursor = target.selection_start().ok().flatten().unwrap_or(0)
                                as usize;
                            let mut val = edit_text.get();
                            val.insert(cursor, '\t');
                            edit_text.set(val);
                            let pos = (cursor + 1) as u32;
                            let _ = target.set_selection_start(Some(pos));
                            let _ = target.set_selection_end(Some(pos));
                        }
                    }
                ></textarea>
            }
            .into_view(),
        )
    }
}
