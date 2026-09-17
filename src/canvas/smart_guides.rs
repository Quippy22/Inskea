use crate::model::{ElementId, Point, Rect};
use leptos::*;

/// Maximum distance at which a proposed position can become a smart-guide candidate.
pub const GUIDE_TOLERANCE: f64 = 8.0;
/// Time a candidate must remain stable before the move is held on the guide.
pub const GUIDE_HOLD_MS: f64 = 300.0;

pub fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuideAxis {
    /// A vertical line, matching an x-coordinate.
    Vertical,
    /// A horizontal line, matching a y-coordinate.
    Horizontal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuideKind {
    Alignment,
    EqualSpacing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuideFeature {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuideKey {
    pub axis: GuideAxis,
    pub kind: GuideKind,
    pub moving_feature: GuideFeature,
    pub target_feature: GuideFeature,
    pub target_a: ElementId,
    pub target_b: ElementId,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuideLine {
    pub start: Point,
    pub end: Point,
    pub label: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GuideCandidate {
    pub key: GuideKey,
    pub snap_delta: f64,
    pub lines: Vec<GuideLine>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PendingGuide {
    pub key: GuideKey,
    pub since_ms: f64,
    /// Whether the one-time alignment correction has already been applied.
    pub applied: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SmartGuideState {
    pub guides: Vec<GuideLine>,
    pub pending: Vec<PendingGuide>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GuideResolution {
    pub snap_dx: f64,
    pub snap_dy: f64,
    pub state: SmartGuideState,
}

#[derive(Clone, Copy)]
struct GuideBox {
    id: ElementId,
    bounds: Rect,
}

fn features(bounds: Rect, axis: GuideAxis) -> [(GuideFeature, f64); 3] {
    match axis {
        GuideAxis::Vertical => [
            (GuideFeature::Start, bounds.x),
            (GuideFeature::Center, bounds.center().x),
            (GuideFeature::End, bounds.right()),
        ],
        GuideAxis::Horizontal => [
            (GuideFeature::Start, bounds.y),
            (GuideFeature::Center, bounds.center().y),
            (GuideFeature::End, bounds.bottom()),
        ],
    }
}

fn alignment_candidate(
    moving: GuideBox,
    target: GuideBox,
    axis: GuideAxis,
) -> Option<GuideCandidate> {
    let moving_features = features(moving.bounds, axis);
    let target_features = features(target.bounds, axis);
    let mut best: Option<GuideCandidate> = None;

    for (moving_feature, moving_value) in moving_features {
        for (target_feature, target_value) in target_features {
            let snap_delta = target_value - moving_value;
            if snap_delta.abs() > GUIDE_TOLERANCE {
                continue;
            }

            let candidate = GuideCandidate {
                key: GuideKey {
                    axis,
                    kind: GuideKind::Alignment,
                    moving_feature,
                    target_feature,
                    target_a: target.id,
                    target_b: 0,
                },
                snap_delta,
                lines: alignment_lines(moving.bounds, target.bounds, axis, target_value),
            };

            if best
                .as_ref()
                .is_none_or(|current| candidate_is_better(&candidate, current))
            {
                best = Some(candidate);
            }
        }
    }

    best
}

/// Draw alignment guides only through the space between element edges. If
/// their bounds overlap along the guide's length, no connector is drawn
/// instead of drawing through the middle of either element.
fn alignment_lines(moving: Rect, target: Rect, axis: GuideAxis, coordinate: f64) -> Vec<GuideLine> {
    match axis {
        GuideAxis::Vertical => {
            let (moving_edge, target_edge) = if moving.center().y <= target.center().y {
                (moving.bottom(), target.y)
            } else {
                (moving.y, target.bottom())
            };
            if moving_edge <= target_edge {
                vec![GuideLine {
                    start: Point::new(coordinate, moving_edge),
                    end: Point::new(coordinate, target_edge),
                    label: None,
                }]
            } else {
                Vec::new()
            }
        }
        GuideAxis::Horizontal => {
            let (moving_edge, target_edge) = if moving.center().x <= target.center().x {
                (moving.right(), target.x)
            } else {
                (moving.x, target.right())
            };
            if moving_edge <= target_edge {
                vec![GuideLine {
                    start: Point::new(moving_edge, coordinate),
                    end: Point::new(target_edge, coordinate),
                    label: None,
                }]
            } else {
                Vec::new()
            }
        }
    }
}

fn equal_spacing_candidate(
    moving: GuideBox,
    left: GuideBox,
    right: GuideBox,
    axis: GuideAxis,
) -> Option<GuideCandidate> {
    let (start, end, moving_start, moving_size, guide_position) = match axis {
        GuideAxis::Vertical => (
            left.bounds.right(),
            right.bounds.x,
            moving.bounds.x,
            moving.bounds.w,
            moving.bounds.center().y,
        ),
        GuideAxis::Horizontal => (
            left.bounds.bottom(),
            right.bounds.y,
            moving.bounds.y,
            moving.bounds.h,
            moving.bounds.center().x,
        ),
    };
    if start > end {
        return None;
    }

    let moving_end = moving_start + moving_size;
    if moving_end < start - GUIDE_TOLERANCE || moving_start > end + GUIDE_TOLERANCE {
        return None;
    }

    let desired_start = (start + end - moving_size) / 2.0;
    let snap_delta = desired_start - moving_start;
    if snap_delta.abs() > GUIDE_TOLERANCE {
        return None;
    }

    let gap = desired_start - start;
    let lines = match axis {
        GuideAxis::Vertical => vec![
            GuideLine {
                start: Point::new(start, guide_position),
                end: Point::new(desired_start, guide_position),
                label: Some(gap),
            },
            GuideLine {
                start: Point::new(desired_start + moving_size, guide_position),
                end: Point::new(end, guide_position),
                label: Some(gap),
            },
        ],
        GuideAxis::Horizontal => vec![
            GuideLine {
                start: Point::new(guide_position, start),
                end: Point::new(guide_position, desired_start),
                label: Some(gap),
            },
            GuideLine {
                start: Point::new(guide_position, desired_start + moving_size),
                end: Point::new(guide_position, end),
                label: Some(gap),
            },
        ],
    };

    let (moving_feature, target_feature) = match axis {
        GuideAxis::Vertical => (GuideFeature::Start, GuideFeature::End),
        GuideAxis::Horizontal => (GuideFeature::Start, GuideFeature::End),
    };
    Some(GuideCandidate {
        key: GuideKey {
            axis,
            kind: GuideKind::EqualSpacing,
            moving_feature,
            target_feature,
            target_a: left.id,
            target_b: right.id,
        },
        snap_delta,
        lines,
    })
}

fn candidate_is_better(candidate: &GuideCandidate, current: &GuideCandidate) -> bool {
    let candidate_priority = u8::from(candidate.key.moving_feature != candidate.key.target_feature);
    let current_priority = u8::from(current.key.moving_feature != current.key.target_feature);
    (candidate_priority, candidate.snap_delta.abs()) < (current_priority, current.snap_delta.abs())
}

fn best_candidate(
    moving: GuideBox,
    others: &[GuideBox],
    axis: GuideAxis,
) -> Option<GuideCandidate> {
    let mut best = others
        .iter()
        .filter_map(|target| alignment_candidate(moving, *target, axis))
        .min_by(|left, right| left.snap_delta.abs().total_cmp(&right.snap_delta.abs()));

    for (index, left) in others.iter().enumerate() {
        for right in others.iter().skip(index + 1) {
            let (left, right) = match axis {
                GuideAxis::Vertical if left.bounds.x <= right.bounds.x => (*left, *right),
                GuideAxis::Vertical => (*right, *left),
                GuideAxis::Horizontal if left.bounds.y <= right.bounds.y => (*left, *right),
                GuideAxis::Horizontal => (*right, *left),
            };
            if let Some(candidate) = equal_spacing_candidate(moving, left, right, axis) {
                let is_better = best
                    .as_ref()
                    .is_none_or(|current| candidate_is_better(&candidate, current));
                if is_better {
                    best = Some(candidate);
                }
            }
        }
    }

    best
}

fn same_candidate_slot(left: GuideKey, right: GuideKey) -> bool {
    left.axis == right.axis
        && left.kind == right.kind
        && left.target_a == right.target_a
        && left.target_b == right.target_b
}

fn previous_pending(previous: &SmartGuideState, key: GuideKey, now_ms: f64) -> (f64, bool) {
    previous
        .pending
        .iter()
        .find(|pending| same_candidate_slot(pending.key, key))
        .map(|pending| (pending.since_ms, pending.applied))
        .unwrap_or((now_ms, false))
}

/// Resolve smart-guide candidates for a proposed move.
///
/// A candidate is applied only after it remains stable for
/// [`GUIDE_HOLD_MS`]. Once applied, it is not reapplied until the candidate is
/// left and entered again, so moving away from an alignment never feels stuck.
pub fn resolve_move(
    proposed: Rect,
    others: &[(ElementId, Rect)],
    now_ms: f64,
    enabled: bool,
    previous: &SmartGuideState,
) -> GuideResolution {
    if !enabled {
        return GuideResolution {
            snap_dx: 0.0,
            snap_dy: 0.0,
            state: SmartGuideState::default(),
        };
    }

    let moving = GuideBox {
        id: 0,
        bounds: proposed,
    };
    let others: Vec<_> = others
        .iter()
        .map(|(id, bounds)| GuideBox {
            id: *id,
            bounds: *bounds,
        })
        .collect();
    let candidates = [
        best_candidate(moving, &others, GuideAxis::Vertical),
        best_candidate(moving, &others, GuideAxis::Horizontal),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    let mut snap_dx = 0.0;
    let mut snap_dy = 0.0;
    let mut pending = Vec::new();
    let mut preview_guides = Vec::new();
    let mut show_vertical = false;
    let mut show_horizontal = false;
    for candidate in candidates {
        let (since_ms, was_applied) = previous_pending(previous, candidate.key, now_ms);
        let held = now_ms - since_ms >= GUIDE_HOLD_MS;
        pending.push(PendingGuide {
            key: candidate.key,
            since_ms,
            applied: was_applied || held,
        });
        preview_guides.extend(candidate.lines.iter().copied());
        if held && !was_applied {
            match candidate.key.axis {
                GuideAxis::Vertical => snap_dx = candidate.snap_delta,
                GuideAxis::Horizontal => snap_dy = candidate.snap_delta,
            }
        }
        let exact_after_apply = was_applied && candidate.snap_delta.abs() <= f64::EPSILON;
        match candidate.key.axis {
            GuideAxis::Vertical => show_vertical |= held && (!was_applied || exact_after_apply),
            GuideAxis::Horizontal => show_horizontal |= held && (!was_applied || exact_after_apply),
        }
    }

    let snapped_moving = Rect::new(
        proposed.x + snap_dx,
        proposed.y + snap_dy,
        proposed.w,
        proposed.h,
    );
    let applied_guides = [
        (GuideAxis::Vertical, show_vertical),
        (GuideAxis::Horizontal, show_horizontal),
    ]
    .into_iter()
    .filter_map(|(axis, show)| {
        show.then(|| {
            best_candidate(
                GuideBox {
                    id: 0,
                    bounds: snapped_moving,
                },
                &others,
                axis,
            )
        })
    })
    .flatten()
    .flat_map(|candidate| candidate.lines)
    .collect();
    let guides = if show_vertical || show_horizontal {
        applied_guides
    } else {
        preview_guides
    };

    GuideResolution {
        snap_dx,
        snap_dy,
        state: SmartGuideState { guides, pending },
    }
}

pub fn guide_overlay(guide_state: RwSignal<SmartGuideState>) -> impl Fn() -> Option<View> {
    move || {
        let state = guide_state.get();
        if state.guides.is_empty() {
            return None;
        }

        let lines = state
            .guides
            .iter()
            .map(|guide| {
                let label = guide.label.map(|value| {
                    let x = (guide.start.x + guide.end.x) / 2.0;
                    let y = (guide.start.y + guide.end.y) / 2.0 - 4.0;
                    view! {
                        <text
                            x=x
                            y=y
                            fill="#7aa2f7"
                            font-size="10"
                            text-anchor="middle"
                            pointer-events="none"
                        >
                            {format!("{:.0}", value.abs())}
                        </text>
                    }
                    .into_view()
                });
                view! {
                    <g pointer-events="none">
                        <line
                            x1=guide.start.x
                            y1=guide.start.y
                            x2=guide.end.x
                            y2=guide.end.y
                            stroke="#7aa2f7"
                            stroke-width="1"
                            stroke-dasharray="4 3"
                            opacity="0.9"
                        />
                        {label}
                    </g>
                }
                .into_view()
            })
            .collect_view();

        Some(view! { <g pointer-events="none">{lines}</g> }.into_view())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(id: ElementId, x: f64, y: f64, w: f64, h: f64) -> (ElementId, Rect) {
        (id, Rect::new(x, y, w, h))
    }

    #[test]
    fn alignment_candidate_waits_before_snapping() {
        let others = [rect(1, 200.0, 140.0, 20.0, 20.0)];
        let proposed = Rect::new(194.0, 100.0, 20.0, 20.0);
        let first = resolve_move(proposed, &others, 0.0, true, &SmartGuideState::default());
        assert_eq!(first.snap_dx, 0.0);
        assert!(!first.state.guides.is_empty());

        let held = resolve_move(proposed, &others, GUIDE_HOLD_MS, true, &first.state);
        assert_eq!(held.snap_dx, 6.0);
        assert!(!held.state.guides.is_empty());

        let moving_again = resolve_move(
            Rect::new(195.0, 100.0, 20.0, 20.0),
            &others,
            GUIDE_HOLD_MS + 16.0,
            true,
            &held.state,
        );
        assert_eq!(moving_again.snap_dx, 0.0);
    }

    #[test]
    fn alignment_feature_changes_do_not_rearm_the_same_target() {
        let others = [rect(1, 200.0, 140.0, 20.0, 20.0)];
        let first = resolve_move(
            Rect::new(180.0, 100.0, 20.0, 20.0),
            &others,
            0.0,
            true,
            &SmartGuideState::default(),
        );
        let held = resolve_move(
            Rect::new(180.0, 100.0, 20.0, 20.0),
            &others,
            GUIDE_HOLD_MS,
            true,
            &first.state,
        );
        assert!(held.state.pending[0].applied);

        let feature_changed = resolve_move(
            Rect::new(194.0, 100.0, 20.0, 20.0),
            &others,
            GUIDE_HOLD_MS + 16.0,
            true,
            &held.state,
        );
        assert_eq!(feature_changed.snap_dx, 0.0);
    }

    #[test]
    fn alignment_guides_stop_at_element_edges() {
        let moving = GuideBox {
            id: 0,
            bounds: Rect::new(100.0, 0.0, 20.0, 20.0),
        };
        let target = GuideBox {
            id: 1,
            bounds: Rect::new(100.0, 40.0, 20.0, 20.0),
        };
        let candidate = alignment_candidate(moving, target, GuideAxis::Vertical).unwrap();
        assert_eq!(candidate.lines.len(), 1);
        assert_eq!(candidate.lines[0].start.y, moving.bounds.bottom());
        assert_eq!(candidate.lines[0].end.y, target.bounds.y);

        let overlapping = GuideBox {
            id: 1,
            bounds: Rect::new(100.0, 10.0, 20.0, 20.0),
        };
        let candidate = alignment_candidate(moving, overlapping, GuideAxis::Vertical).unwrap();
        assert!(candidate.lines.is_empty());
    }

    #[test]
    fn equal_spacing_candidate_holds_at_the_middle_gap() {
        let others = [
            rect(1, 0.0, 100.0, 20.0, 20.0),
            rect(2, 100.0, 100.0, 20.0, 20.0),
        ];
        let proposed = Rect::new(44.0, 50.0, 20.0, 20.0);
        let first = resolve_move(proposed, &others, 0.0, true, &SmartGuideState::default());
        assert_eq!(first.state.guides.len(), 2);
        assert_eq!(first.state.guides[0].label, Some(30.0));

        let held = resolve_move(proposed, &others, GUIDE_HOLD_MS, true, &first.state);
        assert_eq!(held.snap_dx, 6.0);
        assert_eq!(held.state.guides.len(), 2);
        assert_eq!(held.state.guides[0].label, Some(30.0));
        assert_eq!(held.state.guides[1].label, Some(30.0));
    }

    #[test]
    fn leaving_a_candidate_clears_the_hold() {
        let others = [rect(1, 200.0, 100.0, 20.0, 20.0)];
        let proposed = Rect::new(194.0, 100.0, 20.0, 20.0);
        let first = resolve_move(proposed, &others, 0.0, true, &SmartGuideState::default());
        let cleared = resolve_move(
            Rect::new(170.0, 170.0, 20.0, 20.0),
            &others,
            GUIDE_HOLD_MS,
            true,
            &first.state,
        );
        assert!(cleared.state.guides.is_empty());
        assert_eq!(cleared.snap_dx, 0.0);
    }
}
