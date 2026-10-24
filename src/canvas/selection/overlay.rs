use super::utils::{selection_bounds, ROTATE_HANDLE_OFFSET, SELECT_PADDING};
use super::DASH_PREVIEW;
use crate::canvas::settings::CanvasSettings;
use crate::model::elements::path::segment_midpoint;
use crate::model::elements::{Bounds, PathPoints};
use crate::model::interaction::layout_bounds;
use crate::model::{Element, ElementId, Point, Rect, Scene};
use leptos::*;

pub fn selection_preview_overlay(
    select_anchor: RwSignal<Option<Point>>,
    cursor_world: RwSignal<Point>,
    settings: RwSignal<CanvasSettings>,
) -> impl Fn() -> Option<View> {
    move || {
        let anchor = select_anchor.get()?;
        let world = cursor_world.get();
        let x = anchor.x.min(world.x);
        let y = anchor.y.min(world.y);
        let w = (world.x - anchor.x).abs();
        let h = (world.y - anchor.y).abs();
        if w < 1.0 || h < 1.0 {
            return None;
        }
        let hex = settings.get().theme.accent();
        Some(
            view! {
                <rect
                    x=x
                    y=y
                    width=w
                    height=h
                    fill=format!("{}33", hex)
                    stroke=hex
                    stroke-width="1"
                    stroke-dasharray=DASH_PREVIEW
                    pointer-events="none"
                />
            }
            .into_view(),
        )
    }
}

/// Path-point dots, segment-midpoint (ghost) handles, and move cross icon
/// for a single-selected path element (Line, Arrow, Freehand).
/// Returns `None` if the element has no path points.
#[inline]
fn render_path_element_overlay(
    el: &Element,
    hex: &'static str,
    fill: &'static str,
) -> Option<View> {
    let points = el.path_points()?;
    let n = points.len();
    let b = el.bounds();
    let mx = b.x + b.w / 2.0;
    let my = b.y - 25.0;

    let path_handles: Vec<_> = points
        .iter()
        .map(|p| {
            view! {
                <circle
                    cx=p.x
                    cy=p.y
                    r="5"
                    fill=fill
                    stroke=hex
                    stroke-width="1.5"
                    pointer-events="none"
                />
            }
            .into_view()
        })
        .collect();

    let cm = el.curve_mode();
    let ghost_handles: Vec<_> = (0..n.saturating_sub(1))
        .filter_map(|i| {
            let (gx, gy) = segment_midpoint(points, cm, i)?;
            Some(
                view! {
                    <circle
                        cx=gx
                        cy=gy
                        r="3.5"
                        fill="none"
                        stroke=hex
                        stroke-width="1"
                        pointer-events="none"
                    />
                }
                .into_view(),
            )
        })
        .collect();

    let move_icon = view! {
        <g
            stroke=hex
            stroke-width="1.5"
            fill="none"
            transform=format!("translate({} {}) scale(0.9375)", mx - 11.25, my - 11.25)
            pointer-events="none"
        >
            <path d="M12 3 L12 21 M3 12 L21 12" />
            <path d="M9 6 L12 3 L15 6" />
            <path d="M9 18 L12 21 L15 18" />
            <path d="M6 9 L3 12 L6 15" />
            <path d="M18 9 L21 12 L18 15" />
        </g>
    };

    Some(
        view! {
            {path_handles}
            {ghost_handles}
            {move_icon}
        }
        .into_view(),
    )
}

/// Compute the padded bounding box for selection handles.
/// Uses frozen bounds during a rotation drag, otherwise recomputes from current elements.
#[inline]
fn compute_padded_bounds(
    ids: &[ElementId],
    els: &[Element],
    overlay_freeze: RwSignal<Option<Rect>>,
) -> Rect {
    let mut b = if let Some(fb) = overlay_freeze.get() {
        fb
    } else {
        selection_bounds(ids, els).unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0))
    };
    b = b.padded(SELECT_PADDING);
    b
}

/// Centre the handle assembly rotates about. While a rotation drag freezes the
/// frame, the pivot stays where the drag started; otherwise a single selection
/// spins around its element's layout centre, the point the element itself
/// rotates around. (A Text's tight glyph box is not centred on that layout
/// box, so using the drawn box centre would swing the handles off the glyphs.)
#[inline]
fn selection_pivot(ids: &[ElementId], els: &[Element], b: Rect, frozen: Option<Rect>) -> Point {
    if let Some(fb) = frozen {
        return fb.center();
    }
    if ids.len() == 1 {
        if let Some(el) = els.iter().find(|el| el.id() == ids[0]) {
            return layout_bounds(el).center();
        }
    }
    b.center()
}

/// Determine the rotation angle to apply to the selection handle assembly.
/// Single-select uses the element's own rotation (plus drag delta during rotate);
/// multi-select only uses the drag delta.
#[inline]
fn compute_rotation(ids: &[ElementId], els: &[Element], rotation_delta: RwSignal<f64>) -> f64 {
    if ids.len() == 1 {
        let el_rot = els
            .iter()
            .find(|el| el.id() == ids[0])
            .map(|el| el.data().rotation)
            .unwrap_or(0.0);
        let delta = rotation_delta.get();
        if el_rot == 0.0 && delta != 0.0 {
            delta
        } else {
            el_rot
        }
    } else {
        rotation_delta.get()
    }
}

/// Thin dashed per-element rectangles for multi-select mode.
/// Excludes path-based elements (Line, Arrow, Freehand).
/// Each box is individually rotated to match its element's orientation.
#[inline]
fn render_multi_select_boxes(ids: &[ElementId], els: &[Element], hex: &'static str) -> Vec<View> {
    if ids.len() <= 1 {
        return vec![];
    }
    els.iter()
        .filter(|el| ids.contains(&el.id()) && el.path_points().is_none())
        .map(|el| {
            let b = if let Element::Text(t) = el {
                t.visual_bounds()
            } else {
                el.bounds()
            };
            let rot = el.data().rotation;
            if rot != 0.0 {
                // Spin around the element's own render pivot: a Text's tight
                // glyph box is not centred on the layout box its glyphs turn
                // around, so the drawn box centre would drift off the glyphs.
                let pivot = layout_bounds(el).center();
                let cx = pivot.x;
                let cy = pivot.y;
                let deg = rot.to_degrees();
                view! {
                    <g transform=format!("rotate({} {} {})", deg, cx, cy)>
                        <rect
                            x=b.x - SELECT_PADDING
                            y=b.y - SELECT_PADDING
                            width=b.w + SELECT_PADDING * 2.0
                            height=b.h + SELECT_PADDING * 2.0
                            fill="none"
                            stroke=hex
                            stroke-width="0.5"
                            stroke-dasharray="3 2"
                            pointer-events="none"
                        />
                    </g>
                }
                .into_view()
            } else {
                view! {
                    <rect
                        x=b.x - SELECT_PADDING
                        y=b.y - SELECT_PADDING
                        width=b.w + SELECT_PADDING * 2.0
                        height=b.h + SELECT_PADDING * 2.0
                        fill="none"
                        stroke=hex
                        stroke-width="0.5"
                        stroke-dasharray="3 2"
                        pointer-events="none"
                    />
                }
                .into_view()
            }
        })
        .collect()
}

/// Solid bounding-box rectangle with 8 hollow resize handle circles. The
/// box is drawn as 8 segments that stop short of every circle, so the lines
/// never cross through a handle.
#[inline]
fn render_bbox_overlay(bx: f64, by: f64, bw: f64, bh: f64, hr: f64, hex: &'static str) -> View {
    let corners = [
        (bx, by),
        (bx + bw / 2.0, by),
        (bx + bw, by),
        (bx, by + bh / 2.0),
        (bx + bw, by + bh / 2.0),
        (bx, by + bh),
        (bx + bw / 2.0, by + bh),
        (bx + bw, by + bh),
    ];

    // Distance from a handle centre where the box line must stop: the handle
    // radius plus clearance for the two strokes and a small visible gap.
    let clear = hr + 2.0;
    let xs = [bx, bx + bw / 2.0, bx + bw];
    let ys = [by, by + bh / 2.0, by + bh];

    let mut segments: Vec<(f64, f64, f64, f64)> = Vec::new();
    for y in [by, by + bh] {
        for (start, end) in [(xs[0], xs[1]), (xs[1], xs[2])] {
            if end - start > clear * 2.0 {
                segments.push((start + clear, y, end - clear, y));
            }
        }
    }
    for x in [bx, bx + bw] {
        for (start, end) in [(ys[0], ys[1]), (ys[1], ys[2])] {
            if end - start > clear * 2.0 {
                segments.push((x, start + clear, x, end - clear));
            }
        }
    }

    view! {
        {segments
            .iter()
            .map(|&(x1, y1, x2, y2)| {
                view! {
                    <line
                        x1=x1
                        y1=y1
                        x2=x2
                        y2=y2
                        stroke=hex
                        stroke-width="1"
                        pointer-events="none"
                    />
                }
                    .into_view()
            })
            .collect_view()}
        {corners
            .iter()
            .map(|&(hx, hy)| {
                view! {
                    <circle
                        cx=hx
                        cy=hy
                        r=hr
                        fill="none"
                        stroke=hex
                        stroke-width="1"
                        pointer-events="none"
                    />
                }
                    .into_view()
            })
            .collect_view()}
    }
    .into_view()
}

pub fn selection_handle_overlay(
    selected_ids: RwSignal<Vec<ElementId>>,
    scene: RwSignal<Scene>,
    overlay_freeze: RwSignal<Option<Rect>>,
    rotation_delta: RwSignal<f64>,
    settings: RwSignal<CanvasSettings>,
) -> impl Fn() -> Option<View> {
    move || {
        let ids = selected_ids.get();
        if ids.is_empty() {
            return None;
        }
        let theme = settings.get().theme;
        // `with` instead of `get`: the overlay re-runs on every scene change
        // (including each drag step) and must not clone the document.
        scene.with(|s| {
            let els = s.elements_for_ids(&ids);
            let hex = theme.accent();
            let fill = theme.canvas();

            if ids.len() == 1 {
                if let Some(el) = els.iter().find(|el| el.id() == ids[0]) {
                    if let Some(view) = render_path_element_overlay(el, hex, fill) {
                        return Some(view);
                    }
                }
            }

            let b = compute_padded_bounds(&ids, &els, overlay_freeze);
            let hr = 3.5;
            let pivot = selection_pivot(&ids, &els, b, overlay_freeze.get());
            let cx = pivot.x;
            let cy = pivot.y;
            let rx = cx;
            let ry = b.y - ROTATE_HANDLE_OFFSET;

            let rot = compute_rotation(&ids, &els, rotation_delta);
            let individual_boxes = render_multi_select_boxes(&ids, &els, hex);
            let inner = render_bbox_overlay(b.x, b.y, b.w, b.h, hr, hex);
            let icons = render_move_rotate_icons(hex, cx, cy, rx, ry);

            let content = if rot != 0.0 {
                let deg = rot.to_degrees();
                view! {
                    {individual_boxes}
                    <g transform=format!("rotate({} {} {})", deg, cx, cy)>{inner}{icons}</g>
                }
                .into_view()
            } else {
                view! {
                    {individual_boxes}
                    {inner}
                    {icons}
                }
                .into_view()
            };

            Some(content)
        })
    }
}

/// Move cross (centred on bbox) and rotation-arrow (above top edge) icons.
/// Not interactive — hit-testing is analytic in `handle_selection_handles`.
#[inline]
fn render_move_rotate_icons(hex: &'static str, cx: f64, cy: f64, rx: f64, ry: f64) -> leptos::View {
    let move_icon = view! {
        <g
            stroke=hex
            stroke-width="1.5"
            fill="none"
            transform=format!("translate({} {}) scale(0.9375)", cx - 11.25, cy - 11.25)
            pointer-events="none"
        >
            <path d="M12 3 L12 21 M3 12 L21 12" />
            <path d="M9 6 L12 3 L15 6" />
            <path d="M9 18 L12 21 L15 18" />
            <path d="M6 9 L3 12 L6 15" />
            <path d="M18 9 L21 12 L18 15" />
        </g>
    };
    let rotate_icon = view! {
        <g
            stroke=hex
            stroke-width="1.5"
            fill="none"
            transform=format!("translate({} {}) scale(0.75)", rx - 9.0, ry - 9.0)
            pointer-events="none"
        >
            <path d="M12 4 A8 8 0 1 1 4 12" />
            <path d="M1.8 11.6 L4 9 L6.2 11.6" />
        </g>
    };
    view! {
        {move_icon}
        {rotate_icon}
    }
    .into_view()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::elements::text::WrappedText;
    use crate::model::{ElementData, Text, TextAlign};

    #[test]
    fn text_handles_pivot_on_the_layout_centre() {
        let mut text = Text {
            data: ElementData::new(1),
            wrapped: WrappedText::new("hi"),
            align: TextAlign::Left,
        };
        text.data.width = 200.0;
        text.data.height = 24.0;
        let els = vec![Element::Text(text)];
        let box_bounds = Rect::new(0.0, 6.0, 40.0, 12.0);

        // The drawn box is the tight glyph box, not centred on the layout box
        // the glyphs actually rotate around.
        assert_eq!(
            selection_pivot(&[1], &els, box_bounds, None),
            Point::new(100.0, 12.0)
        );
        // A frozen frame (rotation drag in progress) keeps its own centre.
        let frozen = Rect::new(0.0, 0.0, 50.0, 50.0);
        assert_eq!(
            selection_pivot(&[1], &els, box_bounds, Some(frozen)),
            Point::new(25.0, 25.0)
        );
    }
}
