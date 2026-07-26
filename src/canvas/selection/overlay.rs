use super::utils::{selection_bounds, ROTATE_HANDLE_OFFSET, SELECT_PADDING};
use super::DASH_PREVIEW;
use crate::model::elements::path::segment_midpoint;
use crate::model::elements::{Bounds, PathPoints};
use crate::model::{Color, Element, ElementId, Point, Rect, Scene};
use leptos::*;

pub fn selection_preview_overlay(
    select_anchor: RwSignal<Option<Point>>,
    cursor_world: RwSignal<Point>,
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
        let hex = Color::BLUE;
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
fn render_path_element_overlay(el: &Element, hex: &'static str) -> Option<View> {
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
                    fill="white"
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
            transform=format!(
                "translate({} {}) scale(0.9375)",
                mx - 11.25,
                my - 11.25,
            )
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
            let d = el.data();
            let rot = d.rotation;
            if rot != 0.0 {
                let cx = d.world_point.x + d.width / 2.0;
                let cy = d.world_point.y + d.height / 2.0;
                let deg = rot.to_degrees();
                view! {
                    <g transform=format!("rotate({} {} {})", deg, cx, cy)>
                        <rect
                            x=d.world_point.x - SELECT_PADDING
                            y=d.world_point.y - SELECT_PADDING
                            width=d.width + SELECT_PADDING * 2.0
                            height=d.height + SELECT_PADDING * 2.0
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
                let b = el.bounds();
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

/// Solid bounding-box rectangle with 8 hollow resize handle circles.
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
    view! {
        <rect
            x=bx
            y=by
            width=bw
            height=bh
            fill="none"
            stroke=hex
            stroke-width="1"
            pointer-events="none"
        />
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
                        stroke-width="1.5"
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
) -> impl Fn() -> Option<View> {
    move || {
        let ids = selected_ids.get();
        if ids.is_empty() {
            return None;
        }
        let els = scene.get().elements().to_vec();
        let hex = Color::BLUE;

        if ids.len() == 1 {
            if let Some(el) = els.iter().find(|el| el.id() == ids[0]) {
                if let Some(view) = render_path_element_overlay(el, hex) {
                    return Some(view);
                }
            }
        }

        let b = compute_padded_bounds(&ids, &els, overlay_freeze);
        let hr = 3.5;
        let cx = b.center().x;
        let cy = b.center().y;
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
