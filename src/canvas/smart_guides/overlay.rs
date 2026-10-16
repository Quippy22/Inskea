//! Leptos view for the guides, drawn in world coordinates under the canvas
//! transform.

use super::{
    GuideKind, SmartGuideState, GUIDE_ALIGNMENT_COLOR, GUIDE_SPACING_BAND_PX, GUIDE_SPACING_CAP_PX,
    GUIDE_SPACING_CLEARANCE_PX, GUIDE_SPACING_COLOR,
};
use crate::canvas::viewport::Viewport;
use leptos::*;

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
                            x,
                            guide.start.y - cap,
                            x,
                            (guide.start.y + cap).max(guide.stops[0] - clearance),
                        ),
                        (
                            x + width,
                            guide.start.y - cap,
                            x + width,
                            (guide.start.y + cap).max(guide.stops[1] - clearance),
                        ),
                    ]
                } else {
                    [
                        (
                            guide.start.x - cap,
                            y,
                            (guide.start.x + cap).max(guide.stops[0] - clearance),
                            y,
                        ),
                        (
                            guide.start.x - cap,
                            y + height,
                            (guide.start.x + cap).max(guide.stops[1] - clearance),
                            y + height,
                        ),
                    ]
                };
                let [(ax1, ay1, ax2, ay2), (bx1, by1, bx2, by2)] = caps;
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
                        <line
                            x1=ax1
                            y1=ay1
                            x2=ax2
                            y2=ay2
                            stroke=GUIDE_SPACING_COLOR
                            stroke-width="1.5"
                            stroke-linecap="round"
                            vector-effect="non-scaling-stroke"
                            opacity="0.9"
                        />
                        <line
                            x1=bx1
                            y1=by1
                            x2=bx2
                            y2=by2
                            stroke=GUIDE_SPACING_COLOR
                            stroke-width="1.5"
                            stroke-linecap="round"
                            vector-effect="non-scaling-stroke"
                            opacity="0.9"
                        />
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
                view! {
                    <g>
                        <line
                            x1=guide.start.x
                            y1=guide.start.y
                            x2=guide.end.x
                            y2=guide.end.y
                            stroke=GUIDE_ALIGNMENT_COLOR
                            stroke-width="4"
                            stroke-linecap="round"
                            vector-effect="non-scaling-stroke"
                            opacity="0.18"
                        />
                        <line
                            x1=guide.start.x
                            y1=guide.start.y
                            x2=guide.end.x
                            y2=guide.end.y
                            stroke=GUIDE_ALIGNMENT_COLOR
                            stroke-width="1.5"
                            stroke-linecap="round"
                            vector-effect="non-scaling-stroke"
                            opacity="0.95"
                        />
                    </g>
                }
                .into_view()
            })
            .collect_view();

        Some(
            view! {
                <g pointer-events="none">
                    {markers}
                    {lines}
                </g>
            }
            .into_view(),
        )
    }
}
