//! Leptos view for the guides, drawn in world coordinates under the canvas
//! transform.

use super::{
    GuideAxis, GuideKind, SmartGuideState, GUIDE_ALIGNMENT_COLOR, GUIDE_LABEL_CLEARANCE_PX,
    GUIDE_LABEL_FONT_PX, GUIDE_MIN_LABEL_SPAN_PX, GUIDE_SPACING_BAND_PX, GUIDE_SPACING_CAP_PX,
    GUIDE_SPACING_CLEARANCE_PX, GUIDE_SPACING_COLOR,
};
use crate::canvas::viewport::Viewport;
use crate::model::Point;
use leptos::*;

/// One guide line, drawn with a stroke that keeps its screen width at any
/// zoom.
fn guide_line(start: Point, end: Point, color: &'static str, width: f64, opacity: f64) -> View {
    let (x1, y1) = (start.x, start.y);
    let (x2, y2) = (end.x, end.y);
    view! {
        <line
            x1=x1
            y1=y1
            x2=x2
            y2=y2
            stroke=color
            stroke-width=width
            stroke-linecap="round"
            vector-effect="non-scaling-stroke"
            opacity=opacity
        />
    }
    .into_view()
}

pub fn guide_overlay(
    guide_state: RwSignal<SmartGuideState>,
    viewport: RwSignal<Viewport>,
) -> impl Fn() -> Option<View> {
    move || {
        let state = guide_state.get();
        if state.guides.is_empty() {
            return None;
        }

        let zoom = viewport.get().zoom;
        let zoom = if zoom.is_finite() && zoom > 0.0 {
            zoom
        } else {
            1.0
        };
        let band = GUIDE_SPACING_BAND_PX / 2.0 / zoom;
        let cap = GUIDE_SPACING_CAP_PX / 2.0 / zoom;
        let clearance = GUIDE_SPACING_CLEARANCE_PX / zoom;

        // Equal gaps read as soft bands with solid end ticks, drawn outside
        // the boxes so they never sit on an alignment line.
        let markers = state
            .guides
            .iter()
            .filter(|guide| guide.kind != GuideKind::Alignment)
            .map(|guide| {
                let horizontal = guide.start.y == guide.end.y;
                let (x, y, width, height) = if horizontal {
                    (
                        guide.start.x.min(guide.end.x),
                        guide.start.y - band,
                        (guide.end.x - guide.start.x).abs(),
                        band * 2.0,
                    )
                } else {
                    (
                        guide.start.x - band,
                        guide.start.y.min(guide.end.y),
                        band * 2.0,
                        (guide.end.y - guide.start.y).abs(),
                    )
                };
                // Each tick reaches past the band towards the piece beside it,
                // so it stays attached when the pieces differ in size.
                let caps = if horizontal {
                    [
                        (
                            Point::new(x, guide.start.y - cap),
                            Point::new(x, (guide.start.y + cap).max(guide.stops[0] - clearance)),
                        ),
                        (
                            Point::new(x + width, guide.start.y - cap),
                            Point::new(
                                x + width,
                                (guide.start.y + cap).max(guide.stops[1] - clearance),
                            ),
                        ),
                    ]
                } else {
                    [
                        (
                            Point::new(guide.start.x - cap, y),
                            Point::new((guide.start.x + cap).max(guide.stops[0] - clearance), y),
                        ),
                        (
                            Point::new(guide.start.x - cap, y + height),
                            Point::new(
                                (guide.start.x + cap).max(guide.stops[1] - clearance),
                                y + height,
                            ),
                        ),
                    ]
                };
                let [(a1, a2), (b1, b2)] = caps;
                view! {
                    <g>
                        <rect
                            x=x
                            y=y
                            width=width
                            height=height
                            rx=band
                            fill=GUIDE_SPACING_COLOR
                            fill-opacity="0.28"
                        />
                        {guide_line(a1, a2, GUIDE_SPACING_COLOR, 1.5, 0.9)}
                        {guide_line(b1, b2, GUIDE_SPACING_COLOR, 1.5, 0.9)}
                    </g>
                }
                .into_view()
            })
            .collect_view();

        // Alignment guides: a crisp line over a soft halo.
        let lines = state
            .guides
            .iter()
            .filter(|guide| guide.kind == GuideKind::Alignment)
            .map(|guide| {
                let (start, end) = (guide.start, guide.end);
                view! {
                    <g>
                        {guide_line(start, end, GUIDE_ALIGNMENT_COLOR, 4.0, 0.18)}
                        {guide_line(start, end, GUIDE_ALIGNMENT_COLOR, 1.5, 0.95)}
                    </g>
                }
                .into_view()
            })
            .collect_view();

        // Measurement readouts: a number beside each active band, in screen
        // pixels whatever the zoom, haloed so it reads over any artwork.
        let font = GUIDE_LABEL_FONT_PX / zoom;
        let label_offset = band + GUIDE_LABEL_CLEARANCE_PX / zoom;
        let labels = state
            .labels
            .iter()
            .filter(|label| label.distance * zoom >= GUIDE_MIN_LABEL_SPAN_PX)
            .map(|label| {
                let horizontal_band = label.axis == GuideAxis::Vertical;
                let (x, y, rotation) = if horizontal_band {
                    (label.at.x, label.at.y - label_offset, 0.0)
                } else {
                    (label.at.x - label_offset, label.at.y, -90.0)
                };
                let distance = format!("{:.0}", label.distance);
                view! {
                    <text
                        x=x
                        y=y
                        transform=format!("rotate({rotation} {x} {y})")
                        text-anchor="middle"
                        dominant-baseline="middle"
                        font-family="sans-serif"
                        font-size=font
                        fill=GUIDE_SPACING_COLOR
                        stroke="#0b0d12"
                        stroke-opacity="0.85"
                        stroke-width=font * 0.3
                        stroke-linejoin="round"
                        paint-order="stroke"
                    >
                        {distance}
                    </text>
                }
                .into_view()
            })
            .collect_view();

        Some(
            view! {
                <g pointer-events="none">
                    {markers}
                    {lines}
                    {labels}
                </g>
            }
            .into_view(),
        )
    }
}
