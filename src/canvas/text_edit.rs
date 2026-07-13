use super::viewport::Viewport;
use crate::model::elements::text::CHAR_WIDTH_RATIO;
use crate::model::{Element, ElementId, Scene};
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
                    let wrap_width = if text_elem.data.width > 0.0 {
                        text_elem.data.width
                    } else {
                        textarea_ref
                            .get()
                            .map(|ta| ta.client_width() as f64 / viewport.get().zoom)
                            .unwrap_or(200.0)
                    };
                    text_elem.set_content(&text, wrap_width);
                }
            });
            editing_id.set(None);
            edit_text.set(String::new());
        }
    })
}

/// Returns a closure that renders the text-editing textarea overlay.
pub fn text_edit_overlay(
    editing_id: RwSignal<Option<ElementId>>,
    edit_text: RwSignal<String>,
    textarea_ref: NodeRef<leptos::html::Textarea>,
    scene: RwSignal<Scene>,
    viewport: RwSignal<Viewport>,
    screen_size: RwSignal<(f64, f64)>,
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
        let font_size = text_elem.data.font_size.max(12.0);
        let (sw, sh) = screen_size.get();
        let (sx, sy) = viewport
            .get()
            .world_to_screen((text_elem.data.world_point.x, text_elem.data.world_point.y), (sw, sh));
        let fill = text_elem
            .data
            .fill_color
            .map(|c| c.to_hex())
            .unwrap_or_else(|| text_elem.data.stroke_color.to_hex());

        let default_ta_w = (20.0_f64 * font_size * CHAR_WIDTH_RATIO).max(120.0);
        let ta_w = if text_elem.data.width > 0.0 {
            (text_elem.data.width * zoom).max(default_ta_w)
        } else {
            default_ta_w
        };
        let ta_h = if text_elem.data.height > 0.0 {
            (text_elem.data.height * zoom).max(font_size * zoom * 1.2)
        } else {
            (font_size * 1.2).max(50.0)
        };

        let ce_blur = commit.clone();
        let ce_esc = commit.clone();

        Some(
            view! {
                <textarea
                    autofocus="true"
                    _ref=textarea_ref
                    style={format!(
                        "position:fixed;left:{}px;top:{}px;\
                         width:{}px;height:{}px;\
                         font-size:{}px;font-family:sans-serif;\
                         color:{};\
                         background:rgba(122,162,247,0.05);\
                         border:1px dashed {};\
                         outline:none;resize:none;\
                         overflow:hidden;\
                         white-space:pre-wrap;overflow-wrap:break-word;\
                         padding:0;margin:0;z-index:100;\
                         line-height:1.2;",
                        sx, sy, ta_w, ta_h, font_size * zoom, fill, fill
                    )}
                    prop:value=move || edit_text.get()
                    on:input=move |ev| {
                        edit_text.set(event_target_value(&ev));
                        let ta = event_target::<web_sys::HtmlTextAreaElement>(&ev);
                        ta.style().set_property("height", "auto").ok();
                        let _ = ta.style().set_property("height", &format!("{}px", ta.scroll_height()));
                    }
                    on:blur=move |_| ce_blur()
                    on:keydown=move |ev: ev::KeyboardEvent| {
                        if ev.key() == "Escape" { ce_esc(); }
                        if ev.key() == "Tab" {
                            ev.prevent_default();
                            let target = event_target::<web_sys::HtmlTextAreaElement>(&ev);
                            let cursor = target.selection_start().ok().flatten().unwrap_or(0) as usize;
                            let mut val = edit_text.get();
                            val.insert(cursor, '\t');
                            edit_text.set(val);
                            let pos = (cursor + 1) as u32;
                            let _ = target.set_selection_start(Some(pos));
                            let _ = target.set_selection_end(Some(pos));
                        }
                    }
                ></textarea>
            }.into_view(),
        )
    }
}
