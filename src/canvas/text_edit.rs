use super::viewport::Viewport;
use crate::model::elements::text::{natural_line_width, FONT_FAMILY, FONT_STACK, MIN_FONT_SIZE};
use crate::model::{Element, ElementId, EmbeddedText, Point, Scene};
use leptos::*;
use std::rc::Rc;

/// Screen padding (CSS px) kept between the caret and the window edge while
/// editing. The textarea is panned to stay inside this band and soft-wraps
/// ("artificial next line") once even the band is too small.
const EDIT_PAD: f64 = 64.0;
/// Fraction of the remaining pan distance moved per animation frame, so the
/// view follows the caret smoothly instead of snapping.
const PAN_SMOOTHING: f64 = 0.3;
/// First-line vertical gap between the SVG text and the editing textarea.
///
/// The SVG renders its baseline explicitly at `world_point.y + 0.85·fs`, but
/// a textarea's first line baseline is derived from the font's own CSS ascent
/// plus half the line-height slack. For Cabin at `line-height: 1.2` in
/// WebKitGTK that lands ~0.06em lower, so opening an edit would visibly drop
/// the text. The textarea is shifted up by this fraction so glyphs land
/// exactly where they were before the edit started (seamless transition).
const EDIT_BASELINE_SHIFT: f64 = 0.06;

/// Focus the textarea and place the caret at the end of its content, once it
/// exists in the DOM. The textarea is inserted reactively after an edit opens,
/// so the first attempt happens on the next animation frame; if the element is
/// not mounted yet, it retries on subsequent frames.
pub fn focus_textarea(textarea_ref: NodeRef<leptos::html::Textarea>) {
    fn try_focus(textarea_ref: NodeRef<leptos::html::Textarea>, mut attempts: u32) {
        if let Some(ta) = textarea_ref.get() {
            let _ = ta.focus();
            let len = ta.value().len() as u32;
            let _ = ta.set_selection_start(Some(len));
            let _ = ta.set_selection_end(Some(len));
        } else if attempts < 10 {
            attempts += 1;
            leptos::leptos_dom::helpers::request_animation_frame(move || {
                try_focus(textarea_ref, attempts)
            });
        }
    }
    leptos::leptos_dom::helpers::request_animation_frame(move || try_focus(textarea_ref, 0));
}

/// Reflow the editing textarea around the typed content.
///
/// * A typed-out box ("fresh" text, width not yet set) keeps the caret within
///   an `EDIT_PAD` band on screen — the canvas pans so the text stays visible —
///   and soft-wraps ("artificial new line") once the band between both edges
///   is too small. On commit it still becomes full-size, unwrapped text.
/// * An already-sized box keeps its wrap width and soft-wraps at it.
///
/// Runs on the next animation frame: after the reactive render has patched,
/// so per-input changes are never overwritten by a re-render. The pan is
/// applied by drifting `viewport.offset_x`, so the whole SVG follows the
/// caret instead of the textarea alone.
pub fn guide_textarea(
    editing_id: RwSignal<Option<ElementId>>,
    editing_embedded: RwSignal<bool>,
    textarea_ref: NodeRef<leptos::html::Textarea>,
    scene: RwSignal<Scene>,
    viewport: RwSignal<Viewport>,
    screen_size: RwSignal<Point>,
) {
    let Some(id) = editing_id.get() else { return };
    let Some(ta) = textarea_ref.get() else { return };
    let raw: &web_sys::HtmlTextAreaElement = &ta;
    if editing_embedded.get() {
        let layout = scene.with(|s| {
            s.element_by_id(id).and_then(|element| {
                crate::model::elements::embedded_text::layout_for_element(&element)
            })
        });
        let Some(layout) = layout else { return };
        let ss = screen_size.get();
        let vp = viewport.get();
        let screen_center = vp.world_to_screen(layout.center, ss);
        let zoom = vp.zoom;
        let font_size = layout.font_size;
        let width = (layout.width * zoom).max(font_size * zoom);
        let style = raw.style();
        let _ = style.set_property("left", &format!("{}px", screen_center.x - width / 2.0));
        let _ = style.set_property("width", &format!("{width}px"));
        let _ = style.set_property("font-size", &format!("{}px", font_size * zoom));
        let _ = style.set_property("white-space", "pre-wrap");
        let _ = style.set_property("text-align", "center");
        let _ = style.set_property("overflow-wrap", "anywhere");
        let _ = style.set_property("height", "auto");
        let content_height = raw.scroll_height().max(1) as f64;
        let _ = style.set_property("height", &format!("{content_height}px"));
        let shift_up = EDIT_BASELINE_SHIFT * font_size * zoom;
        // Keep the first line anchored where the label started. Centering the
        // textarea by its growing content height moves every existing line up
        // on Enter and can push it out of the host shape.
        let initial_height = layout.height * zoom;
        let top = screen_center.y - initial_height / 2.0 - shift_up;
        let _ = style.set_property("top", &format!("{top}px"));
        let band_bottom = (ss.y - EDIT_PAD).max(EDIT_PAD);
        let overflow = top + content_height - band_bottom;
        if overflow > 0.5 {
            pan_viewport_y(top, top - overflow, &viewport);
        }
        if let Some((deg, ox, oy)) = rotation_css(
            layout.rotation,
            layout.width,
            content_height / zoom,
            zoom,
            shift_up,
        ) {
            let _ = style.set_property("transform", &format!("rotate({deg}deg)"));
            let _ = style.set_property("transform-origin", &format!("{ox}px {oy}px"));
        } else {
            let _ = style.remove_property("transform");
            let _ = style.remove_property("transform-origin");
        }
        return;
    }
    let Some(text) = scene.with(|s| s.element_by_id(id)) else {
        return;
    };
    let Element::Text(text_elem) = text else {
        return;
    };

    let ss = screen_size.get();
    let band_left = EDIT_PAD;
    let band_right = (ss.x - EDIT_PAD).max(band_left);
    let column = (band_right - band_left).max(1.0);
    let vp_world = text_elem.data.world_point;
    let zoom = viewport.get().zoom;
    let sx = (vp_world.x - viewport.get().offset_x) * zoom + ss.x / 2.0;
    let sy = (vp_world.y - viewport.get().offset_y) * zoom + ss.y / 2.0;
    let font_size = text_elem.data.style.font_size.max(MIN_FONT_SIZE);
    let fill = text_elem.data.style.stroke_color.to_hex();

    let sty = raw.style();
    let _ = sty.set_property("left", &format!("{sx}px"));
    let _ = sty.set_property(
        "top",
        &format!("{}px", sy - EDIT_BASELINE_SHIFT * font_size * zoom),
    );
    let _ = sty.set_property("font-size", &format!("{}px", font_size * zoom));
    let _ = sty.set_property("color", &fill);

    let sized = text_elem.data.width > 0.0;
    if sized {
        // Resized box: wrap at its chosen width, no panning.
        let _ = sty.set_property("white-space", "pre-wrap");
        let _ = sty.set_property("width", &format!("{}px", text_elem.data.width * zoom));
    } else {
        // Measure the real rendered width of the current line (as the browser
        // lays it out), so the follow/wrap thresholds match the pixels the
        // user actually sees. Pin the box to 1px, force a reflow by reading
        // scrollWidth, then grow it back.
        let _ = sty.set_property("white-space", "pre");
        let _ = sty.set_property("width", "1px");
        let natural = raw.scroll_width().max(1) as f64;

        if sx + natural <= band_right {
            // Fits in the band: one flowing line.
            let _ = sty.set_property("width", &format!("{natural}px"));
        } else if natural <= column {
            // Overflows the band but fits a pan: drift the view right so the
            // (end of the) caret stays at the right padding; the text origin
            // slides left towards the left padding.
            let target = (band_right - natural).max(band_left);
            pan_and_reposition(sx, target, vp_world, zoom, ss.x, &viewport, &sty);
            let _ = sty.set_property("width", &format!("{natural}px"));
        } else {
            // Too wide for the band even when snapped to the left padding:
            // artificial new line — wrap inside the band.
            pan_and_reposition(sx, band_left, vp_world, zoom, ss.x, &viewport, &sty);
            let _ = sty.set_property("white-space", "pre-wrap");
            let _ = sty.set_property("width", &format!("{column}px"));
        }
    }

    let _ = sty.set_property("height", "auto");
    let content_h = raw.scroll_height().max(1) as f64;
    let _ = sty.set_property("height", &format!("{content_h}px"));

    // Vertical follow: when the grown textarea passes the bottom padding
    // band, pan the canvas up so the caret line stays on screen.
    let top = sy - EDIT_BASELINE_SHIFT * font_size * zoom;
    let band_bottom = (ss.y - EDIT_PAD).max(EDIT_PAD);
    let overflow = top + content_h - band_bottom;
    if overflow > 0.5 {
        pan_viewport_y(top, top - overflow, &viewport);
    }

    // Keep a rotated textarea pivoting on the same centre as the SVG text.
    if let Some((deg, ox, oy)) = rotation_css(
        text_elem.data.rotation,
        text_elem.data.width,
        text_elem.data.height,
        zoom,
        EDIT_BASELINE_SHIFT * font_size * zoom,
    ) {
        let _ = sty.set_property("transform", &format!("rotate({deg}deg)"));
        let _ = sty.set_property("transform-origin", &format!("{ox}px {oy}px"));
    }
}

/// Rotation angle and CSS `transform-origin` (px) for the editing textarea.
/// Mirrors the SVG pivot: the element's box centre, offset by the baseline
/// shift between the textarea's top edge and the SVG text's top edge.
fn rotation_css(
    rotation: f64,
    width: f64,
    height: f64,
    zoom: f64,
    shift_up: f64,
) -> Option<(f64, f64, f64)> {
    (rotation != 0.0).then(|| {
        (
            rotation.to_degrees(),
            width * zoom / 2.0,
            height * zoom / 2.0 + shift_up,
        )
    })
}

/// Vertical counterpart of [`pan_and_reposition`]: ease `viewport.offset_y`
/// so the editing textarea's top drifts toward `target_top` on screen.
fn pan_viewport_y(sy: f64, target_top: f64, viewport: &RwSignal<Viewport>) {
    let dy = sy - target_top;
    if dy.abs() < 0.5 {
        return;
    }
    let step = dy * PAN_SMOOTHING;
    viewport.update(|vp| vp.offset_y += step / vp.zoom);
}

/// Pan `viewport.offset_x` so the text origin eases toward screen `target`,
/// then immediately re-position the textarea to the new world-to-screen spot.
///
/// Only a fraction of the remaining distance moves each call. `guide_textarea`
/// re-runs on the next animation frame whenever the viewport is still moving,
/// so this interpolates over several frames until it settles — no snap. When
/// it has settled it stops panning, the viewport stops changing, and the
/// re-run loop ends by itself.
fn pan_and_reposition(
    sx: f64,
    target_sx: f64,
    vp_world: Point,
    zoom: f64,
    screen_w: f64,
    viewport: &RwSignal<Viewport>,
    sty: &web_sys::CssStyleDeclaration,
) {
    let dx = sx - target_sx;
    if dx.abs() < 0.5 {
        return;
    }
    let step = dx * PAN_SMOOTHING;
    viewport.update(|vp| vp.offset_x += step / vp.zoom);
    let new_sx = (vp_world.x - viewport.get().offset_x) * zoom + screen_w / 2.0;
    let _ = sty.set_property("left", &format!("{new_sx}px"));
}

/// Create the commit function that saves textarea content back to the element.
pub fn make_commit_edit(
    editing_id: RwSignal<Option<ElementId>>,
    editing_embedded: RwSignal<bool>,
    editing_new: RwSignal<bool>,
    edit_text: RwSignal<String>,
    scene: RwSignal<Scene>,
    push_snapshot: Rc<dyn Fn()>,
    discard_snapshot: Rc<dyn Fn()>,
) -> Rc<dyn Fn()> {
    Rc::new(move || {
        if let Some(id) = editing_id.get() {
            let text = edit_text.get();
            let embedded = editing_embedded.get_untracked();
            let is_new = editing_new.get_untracked();
            // Editing existing content is an undoable action; the journal
            // entry for a freshly created text was already opened when it was
            // made.
            let edited = scene.with_untracked(|s| match s.element_by_id(id) {
                Some(element) if embedded => {
                    element.data().embedded_text.as_ref().is_some_and(|label| {
                        !label.wrapped.raw.is_empty() && label.wrapped.raw.as_str() != text
                    })
                }
                Some(Element::Text(t)) => {
                    !t.wrapped.raw.is_empty() && t.wrapped.raw.as_str() != text
                }
                _ => false,
            });
            if edited || (embedded && is_new && !text.is_empty()) {
                push_snapshot();
            }
            scene.update(|s| {
                if embedded {
                    if let Some(mut element) = s.element_by_id_mut(id) {
                        if text.is_empty() {
                            element.data_mut().embedded_text = None;
                        } else if let Some(label) = element.data_mut().embedded_text.as_mut() {
                            label.set_content(&text);
                        } else {
                            let Some((width, height)) =
                                crate::model::elements::embedded_text::reference_size_for_element(
                                    &element,
                                )
                            else {
                                return;
                            };
                            let (color, font_size) = {
                                let data = element.data();
                                (data.style.stroke_color, data.style.font_size)
                            };
                            let mut label = EmbeddedText::new(color, font_size, width, height);
                            label.set_content(&text);
                            element.data_mut().embedded_text = Some(label);
                        }
                    }
                } else if text.is_empty() {
                    s.remove_by_id(id);
                } else if let Some(mut element) = s.element_by_id_mut(id) {
                    let Element::Text(text_elem) = &mut *element else {
                        return;
                    };
                    // Keep the wrap width of an already-sized box. A fresh box
                    // is committed at its natural, unwrapped width — measured
                    // from the model, not the clipped textarea — so the SVG
                    // shows it full-size as a single flowing line.
                    let fs = text_elem.data.style.font_size.max(MIN_FONT_SIZE);
                    let wrap_w = text_elem.data.width;
                    let new_w = if wrap_w > 0.0 {
                        wrap_w
                    } else {
                        natural_line_width(&text, fs, FONT_FAMILY)
                    };
                    text_elem.set_content(&text);
                    text_elem.data.width = new_w;
                }
            });
            if text.is_empty() && is_new {
                // Dismissing a newly-created text box or label collapses its
                // creation journal entry to nothing.
                discard_snapshot();
            }
            editing_id.set(None);
            editing_embedded.set(false);
            editing_new.set(false);
            edit_text.set(String::new());
        }
    })
}

/// Returns a closure that renders the invisible text-editing textarea overlay.
/// No border or background — only the caret and the typed text are visible.
#[allow(clippy::too_many_arguments)]
pub fn text_edit_overlay(
    editing_id: RwSignal<Option<ElementId>>,
    editing_embedded: RwSignal<bool>,
    edit_text: RwSignal<String>,
    textarea_ref: NodeRef<leptos::html::Textarea>,
    scene: RwSignal<Scene>,
    viewport: RwSignal<Viewport>,
    screen_size: RwSignal<Point>,
    commit: Rc<dyn Fn()>,
) -> impl Fn() -> Option<View> {
    move || {
        // The textarea must NOT re-render while an edit is in progress: the
        // overlay is a `DynChild`, whose reactive re-run destroys and recreates
        // the `<textarea>` node — that drops focus, firing `blur` and
        // committing the edit out from under the user. So only `editing_id`
        // (show/hide) is tracked here; every sizing/positioning/style read is
        // untracked and re-applied imperatively by `guide_textarea` each frame.
        let id = editing_id.get()?;
        let embedded = editing_embedded.get_untracked();
        let (world_point, width, height, rotation, font_size, fill, align, keep_width) = scene
            .with_untracked(|s| {
                let element = s.element_by_id(id)?;
                if embedded {
                    let label = element.data().embedded_text.clone().or_else(|| {
                        let (width, height) =
                            crate::model::elements::embedded_text::reference_size_for_element(
                                &element,
                            )?;
                        Some(EmbeddedText::new(
                            element.data().style.stroke_color,
                            element.data().style.font_size,
                            width,
                            height,
                        ))
                    })?;
                    let mut layout_element = element.clone();
                    layout_element.data_mut().embedded_text = Some(label.clone());
                    let layout =
                        crate::model::elements::embedded_text::layout_for_element(&layout_element)?;
                    Some((
                        Point::new(
                            layout.center.x - layout.width / 2.0,
                            layout.center.y - layout.height / 2.0,
                        ),
                        layout.width,
                        layout.height,
                        layout.rotation,
                        layout.font_size,
                        label.color.to_hex(),
                        crate::model::TextAlign::Center,
                        true,
                    ))
                } else if let Element::Text(text) = element {
                    Some((
                        text.data.world_point,
                        text.data.width,
                        text.data.height,
                        text.data.rotation,
                        text.data.style.font_size.max(MIN_FONT_SIZE),
                        text.data.style.stroke_color.to_hex(),
                        text.align,
                        text.data.width > 0.0,
                    ))
                } else {
                    None
                }
            })?;
        let vp = viewport.get_untracked();
        let ss = screen_size.get_untracked();
        let screen_pos = vp.world_to_screen(world_point, ss);
        let (sx, sy) = (screen_pos.x, screen_pos.y);
        let shift_up = EDIT_BASELINE_SHIFT * font_size * vp.zoom;
        let rotation_style = rotation_css(rotation, width, height, vp.zoom, shift_up)
            .map(|(deg, ox, oy)| {
                format!("transform:rotate({deg}deg);transform-origin:{ox}px {oy}px;")
            })
            .unwrap_or_default();

        let ta_w = if width > 0.0 {
            width * vp.zoom
        } else {
            font_size * vp.zoom
        };
        let ta_h = if height > 0.0 {
            height * vp.zoom
        } else {
            font_size * vp.zoom * 1.2
        };

        let ce_blur = commit.clone();
        let ce_esc = commit.clone();
        let ce_finish = commit.clone();
        let blur_id = id;
        // Sized (~resized) boxes keep their wrap width while editing and
        // soft-wrap words at it. Fresh text grows with its content and
        // must not wrap until it has a width — that only happens on commit.
        let white_space = if keep_width { "pre-wrap" } else { "pre" };

        Some(
            view! {
                <textarea
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
                         white-space:{};overflow-wrap:normal;\
                         padding:0;margin:0;z-index:100;\
                         line-height:1.2;text-align:{};{}",
                        sx,
                        sy - shift_up,
                        ta_w,
                        ta_h,
                        font_size * vp.zoom,
                        FONT_STACK,
                        fill,
                        white_space,
                        align.css(),
                        rotation_style,
                    )
                    prop:value=move || edit_text.get()
                    on:input=move |ev| {
                        edit_text.set(event_target_value(&ev));
                    }
                    on:blur=move |_| {
                        if editing_id.get() == Some(blur_id) {
                            ce_blur();
                        }
                    }
                    on:keydown=move |ev: ev::KeyboardEvent| {
                        if ev.key() == "Escape" {
                            // The canvas Escape handler must not also treat
                            // this as a deselect/cancel: the edit owns it.
                            ev.stop_propagation();
                            ce_esc();
                        }
                        if ev.key() == "Enter" && (ev.ctrl_key() || ev.meta_key()) {
                            ev.prevent_default();
                            ev.stop_propagation();
                            ce_finish();
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
