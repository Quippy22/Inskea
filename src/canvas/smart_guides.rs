use crate::model::{ElementId, Point, Rect};

/// Maximum distance at which a proposed position can become a smart-guide candidate.
pub const GUIDE_TOLERANCE: f64 = 8.0;
/// Time a candidate must remain stable before the move is held on the guide.
pub const GUIDE_HOLD_MS: f64 = 300.0;

pub fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|window| window.performance().ok())
            .map(|performance| performance.now())
            .unwrap_or(0.0)
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

            let (start, end) = match axis {
                GuideAxis::Vertical => (
                    Point::new(target_value, moving.bounds.center().y),
                    Point::new(target_value, target.bounds.center().y),
                ),
                GuideAxis::Horizontal => (
                    Point::new(moving.bounds.center().x, target_value),
                    Point::new(target.bounds.center().x, target_value),
                ),
            };
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
                lines: vec![GuideLine {
                    start,
                    end,
                    label: None,
                }],
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
            moving.bounds.y.min(left.bounds.y).min(right.bounds.y) - 12.0,
        ),
        GuideAxis::Horizontal => (
            left.bounds.bottom(),
            right.bounds.y,
            moving.bounds.y,
            moving.bounds.h,
            moving.bounds.x.min(left.bounds.x).min(right.bounds.x) - 12.0,
        ),
    };
    if start > end {
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
                label: None,
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
                label: None,
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

fn pending_since(previous: &SmartGuideState, key: GuideKey, now_ms: f64) -> f64 {
    previous
        .pending
        .iter()
        .find(|pending| pending.key == key)
        .map(|pending| pending.since_ms)
        .unwrap_or(now_ms)
}

/// Resolve smart-guide candidates for a proposed move.
///
/// Candidates are displayed immediately, but their correction is applied only
/// after the same candidate remains stable for [`GUIDE_HOLD_MS`].
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
    let mut guides = Vec::new();
    for candidate in candidates {
        let since_ms = pending_since(previous, candidate.key, now_ms);
        pending.push(PendingGuide {
            key: candidate.key,
            since_ms,
        });
        guides.extend(candidate.lines.iter().copied());
        if now_ms - since_ms >= GUIDE_HOLD_MS {
            match candidate.key.axis {
                GuideAxis::Vertical => snap_dx = candidate.snap_delta,
                GuideAxis::Horizontal => snap_dy = candidate.snap_delta,
            }
        }
    }

    GuideResolution {
        snap_dx,
        snap_dy,
        state: SmartGuideState { guides, pending },
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
        let others = [rect(1, 200.0, 100.0, 20.0, 20.0)];
        let proposed = Rect::new(194.0, 100.0, 20.0, 20.0);
        let first = resolve_move(proposed, &others, 0.0, true, &SmartGuideState::default());
        assert_eq!(first.snap_dx, 0.0);
        assert!(first.state.guides.len() >= 2);

        let held = resolve_move(proposed, &others, GUIDE_HOLD_MS, true, &first.state);
        assert_eq!(held.snap_dx, 6.0);
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
