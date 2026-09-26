use crate::model::{ElementId, Point, Rect};
use crate::util::now_ms;
use leptos::*;

/// Maximum distance at which a proposed position can become a smart-guide candidate.
pub const GUIDE_TOLERANCE: f64 = 6.0;
/// Distance at which an already active guide releases.
pub const GUIDE_RELEASE_TOLERANCE: f64 = 9.0;
/// Time to hold a newly snapped guide before allowing release (ms).
const GUIDE_LOCK_MS: u64 = 300;

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
    pub kind: GuideKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GuideCandidate {
    pub key: GuideKey,
    pub snap_delta: f64,
    pub lines: Vec<GuideLine>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SmartGuideState {
    pub guides: Vec<GuideLine>,
    pub active: Vec<GuideKey>,
    pub lock_until_ms: Option<f64>,
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
    tolerance: f64,
) -> Option<GuideCandidate> {
    let moving_features = features(moving.bounds, axis);
    let target_features = features(target.bounds, axis);
    let mut best: Option<GuideCandidate> = None;

    for (moving_feature, moving_value) in moving_features {
        for (target_feature, target_value) in target_features {
            let snap_delta = target_value - moving_value;
            if snap_delta.abs() > tolerance {
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
                .is_none_or(|current| alignment_candidate_is_better(&candidate, current))
            {
                best = Some(candidate);
            }
        }
    }

    best
}

/// Draw alignment guides between the nearest facing edges of the two elements.
/// If they overlap along the perpendicular axis, draw the line spanning the
/// full extent of both elements so the guide is always visible when snapping.
fn feature_value(bounds: Rect, axis: GuideAxis, feature: GuideFeature) -> f64 {
    match (axis, feature) {
        (GuideAxis::Vertical, GuideFeature::Start) => bounds.x,
        (GuideAxis::Vertical, GuideFeature::Center) => bounds.center().x,
        (GuideAxis::Vertical, GuideFeature::End) => bounds.right(),
        (GuideAxis::Horizontal, GuideFeature::Start) => bounds.y,
        (GuideAxis::Horizontal, GuideFeature::Center) => bounds.center().y,
        (GuideAxis::Horizontal, GuideFeature::End) => bounds.bottom(),
    }
}

fn candidate_lines(
    snapped_moving: GuideBox,
    others: &[GuideBox],
    candidate: &GuideCandidate,
) -> Vec<GuideLine> {
    if candidate.key.kind != GuideKind::Alignment {
        return candidate.lines.clone();
    }

    let axis = candidate.key.axis;
    let feature = candidate.key.moving_feature;
    let coordinate = feature_value(snapped_moving.bounds, axis, feature);

    let mut boxes = vec![snapped_moving];
    for other in others {
        let val = feature_value(other.bounds, axis, feature);
        if (val - coordinate).abs() < 1.0 {
            boxes.push(*other);
        }
    }

    if boxes.len() < 2 {
        return candidate.lines.clone();
    }

    boxes.sort_by(|a, b| {
        let (pos_a, pos_b) = match axis {
            GuideAxis::Vertical => (a.bounds.y, b.bounds.y),
            GuideAxis::Horizontal => (a.bounds.x, b.bounds.x),
        };
        pos_a.total_cmp(&pos_b)
    });

    let mut lines = Vec::new();
    for window in boxes.windows(2) {
        let a = window[0];
        let b = window[1];
        lines.extend(alignment_lines(a.bounds, b.bounds, axis, coordinate));
    }

    lines
}

fn alignment_lines(moving: Rect, target: Rect, axis: GuideAxis, coordinate: f64) -> Vec<GuideLine> {
    match axis {
        GuideAxis::Vertical => {
            let moving_top = moving.y;
            let moving_bottom = moving.bottom();
            let target_top = target.y;
            let target_bottom = target.bottom();

            let (start_y, end_y) = if moving_bottom <= target_top {
                (moving_bottom, target_top)
            } else if target_bottom <= moving_top {
                (target_bottom, moving_top)
            } else {
                (moving_top.min(target_top), moving_bottom.max(target_bottom))
            };

            vec![GuideLine {
                start: Point::new(coordinate, start_y),
                end: Point::new(coordinate, end_y),
                kind: GuideKind::Alignment,
            }]
        }
        GuideAxis::Horizontal => {
            let moving_left = moving.x;
            let moving_right = moving.right();
            let target_left = target.x;
            let target_right = target.right();

            let (start_x, end_x) = if moving_right <= target_left {
                (moving_right, target_left)
            } else if target_right <= moving_left {
                (target_right, moving_left)
            } else {
                (moving_left.min(target_left), moving_right.max(target_right))
            };

            vec![GuideLine {
                start: Point::new(start_x, coordinate),
                end: Point::new(end_x, coordinate),
                kind: GuideKind::Alignment,
            }]
        }
    }
}

fn perpendicular_gap(moving: Rect, target: Rect, axis: GuideAxis) -> f64 {
    match axis {
        GuideAxis::Vertical if moving.bottom() < target.y => target.y - moving.bottom(),
        GuideAxis::Vertical if target.bottom() < moving.y => moving.y - target.bottom(),
        GuideAxis::Horizontal if moving.right() < target.x => target.x - moving.right(),
        GuideAxis::Horizontal if target.right() < moving.x => moving.x - target.right(),
        _ => 0.0,
    }
}

fn equal_spacing_gap(
    moving: GuideBox,
    left: GuideBox,
    right: GuideBox,
    axis: GuideAxis,
    tolerance: f64,
) -> Option<(f64, f64, f64, f64, f64)> {
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
    let gap = end - start;
    if gap <= 0.0 || moving_size >= gap {
        return None;
    }
    if perpendicular_gap(moving.bounds, left.bounds, axis) > tolerance
        || perpendicular_gap(moving.bounds, right.bounds, axis) > tolerance
        || perpendicular_gap(left.bounds, right.bounds, axis) > tolerance
    {
        return None;
    }

    Some((start, end, moving_start, moving_size, guide_position))
}

fn equal_spacing_lines(
    start: f64,
    end: f64,
    moving_size: f64,
    guide_position: f64,
    axis: GuideAxis,
) -> Vec<GuideLine> {
    let desired_start = (start + end - moving_size) / 2.0;
    match axis {
        GuideAxis::Vertical => vec![
            GuideLine {
                start: Point::new(start, guide_position),
                end: Point::new(desired_start, guide_position),
                kind: GuideKind::EqualSpacing,
            },
            GuideLine {
                start: Point::new(desired_start + moving_size, guide_position),
                end: Point::new(end, guide_position),
                kind: GuideKind::EqualSpacing,
            },
        ],
        GuideAxis::Horizontal => vec![
            GuideLine {
                start: Point::new(guide_position, start),
                end: Point::new(guide_position, desired_start),
                kind: GuideKind::EqualSpacing,
            },
            GuideLine {
                start: Point::new(guide_position, desired_start + moving_size),
                end: Point::new(guide_position, end),
                kind: GuideKind::EqualSpacing,
            },
        ],
    }
}

fn equal_spacing_candidate(
    moving: GuideBox,
    left: GuideBox,
    right: GuideBox,
    axis: GuideAxis,
    tolerance: f64,
) -> Option<GuideCandidate> {
    let (start, end, moving_start, moving_size, guide_position) =
        equal_spacing_gap(moving, left, right, axis, tolerance)?;

    let moving_end = moving_start + moving_size;
    if moving_end < start - tolerance || moving_start > end + tolerance {
        return None;
    }

    let desired_start = (start + end - moving_size) / 2.0;
    let snap_delta = desired_start - moving_start;
    if snap_delta.abs() > tolerance {
        return None;
    }

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
        lines: equal_spacing_lines(start, end, moving_size, guide_position, axis),
    })
}

fn alignment_candidate_is_better(candidate: &GuideCandidate, current: &GuideCandidate) -> bool {
    let candidate_priority = u8::from(candidate.key.moving_feature != candidate.key.target_feature);
    let current_priority = u8::from(current.key.moving_feature != current.key.target_feature);
    (candidate_priority, candidate.snap_delta.abs()) < (current_priority, current.snap_delta.abs())
}

fn candidate_is_better(candidate: &GuideCandidate, current: &GuideCandidate) -> bool {
    let candidate_priority = u8::from(candidate.key.moving_feature != candidate.key.target_feature);
    let current_priority = u8::from(current.key.moving_feature != current.key.target_feature);
    let delta_order = candidate
        .snap_delta
        .abs()
        .total_cmp(&current.snap_delta.abs());
    delta_order.is_lt() || (delta_order.is_eq() && candidate_priority < current_priority)
}

fn candidates_for_axis(
    moving: GuideBox,
    others: &[GuideBox],
    axis: GuideAxis,
    tolerance: f64,
) -> Vec<GuideCandidate> {
    let mut candidates = others
        .iter()
        .filter_map(|target| alignment_candidate(moving, *target, axis, tolerance))
        .collect::<Vec<_>>();

    for (index, left) in others.iter().enumerate() {
        for right in others.iter().skip(index + 1) {
            let (left, right) = match axis {
                GuideAxis::Vertical if left.bounds.x <= right.bounds.x => (*left, *right),
                GuideAxis::Vertical => (*right, *left),
                GuideAxis::Horizontal if left.bounds.y <= right.bounds.y => (*left, *right),
                GuideAxis::Horizontal => (*right, *left),
            };
            if let Some(candidate) = equal_spacing_candidate(moving, left, right, axis, tolerance) {
                candidates.push(candidate);
            }
        }
    }

    candidates
}

fn equal_spacing_distance(candidate: &GuideCandidate) -> f64 {
    let first = &candidate.lines[0];
    let second = &candidate.lines[1];
    match candidate.key.axis {
        GuideAxis::Vertical => first.end.x - first.start.x + second.end.x - second.start.x,
        GuideAxis::Horizontal => first.end.y - first.start.y + second.end.y - second.start.y,
    }
}

fn equal_spacing_gap_matches(left: f64, right: f64) -> bool {
    let epsilon = 1e-6 * left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= epsilon
}

fn equal_spacing_guides(
    moving: GuideBox,
    others: &[GuideBox],
    selected: &GuideCandidate,
    tolerance: f64,
) -> Vec<GuideLine> {
    let distance = equal_spacing_distance(selected);
    let mut guides = Vec::new();
    for (index, left) in others.iter().enumerate() {
        for right in others.iter().skip(index + 1) {
            let (left, right) = match selected.key.axis {
                GuideAxis::Vertical if left.bounds.x <= right.bounds.x => (*left, *right),
                GuideAxis::Vertical => (*right, *left),
                GuideAxis::Horizontal if left.bounds.y <= right.bounds.y => (*left, *right),
                GuideAxis::Horizontal => (*right, *left),
            };
            let Some((start, end, _, moving_size, guide_position)) =
                equal_spacing_gap(moving, left, right, selected.key.axis, tolerance)
            else {
                continue;
            };
            if equal_spacing_gap_matches(end - start - moving_size, distance) {
                guides.extend(equal_spacing_lines(
                    start,
                    end,
                    moving_size,
                    guide_position,
                    selected.key.axis,
                ));
            }
        }
    }
    guides
}

fn best_candidate(candidates: Vec<GuideCandidate>) -> Option<GuideCandidate> {
    candidates.into_iter().reduce(|left, right| {
        if candidate_is_better(&left, &right) {
            left
        } else {
            right
        }
    })
}

fn same_candidate_slot(left: GuideKey, right: GuideKey) -> bool {
    left.axis == right.axis
        && left.kind == right.kind
        && left.moving_feature == right.moving_feature
        && left.target_feature == right.target_feature
        && left.target_a == right.target_a
        && left.target_b == right.target_b
}

fn active_candidate(
    moving: GuideBox,
    others: &[GuideBox],
    axis: GuideAxis,
    previous: &SmartGuideState,
    now_ms: f64,
) -> Option<GuideCandidate> {
    let release_candidates = candidates_for_axis(moving, others, axis, GUIDE_RELEASE_TOLERANCE);

    let is_locked = previous.lock_until_ms.map_or(false, |lock| now_ms < lock);

    if let Some(active_key) = previous.active.iter().find(|key| key.axis == axis) {
        if let Some(candidate) = release_candidates
            .iter()
            .find(|candidate| same_candidate_slot(candidate.key, *active_key))
        {
            if is_locked || candidate.snap_delta.abs() <= GUIDE_RELEASE_TOLERANCE {
                return Some(candidate.clone());
            }
        }
    }

    best_candidate(candidates_for_axis(moving, others, axis, GUIDE_TOLERANCE))
}

/// Resolve smart-guide candidates for a proposed move.
///
/// Resolve both axes on every pointer update. Active guides use a slightly
/// wider release band so they remain stable while the pointer crosses the
/// candidate, but they are reacquired continuously after release.
/// Newly snapped guides are locked for GUIDE_LOCK_MS to prevent aggressive re-snapping.
pub fn resolve_move(
    proposed: Rect,
    others: &[(ElementId, Rect)],
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

    let now_ms = now_ms();
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
        active_candidate(moving, &others, GuideAxis::Vertical, previous, now_ms),
        active_candidate(moving, &others, GuideAxis::Horizontal, previous, now_ms),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    let mut snap_dx = 0.0;
    let mut snap_dy = 0.0;
    let mut active = Vec::new();
    let mut new_lock = false;
    for candidate in &candidates {
        active.push(candidate.key);
        match candidate.key.axis {
            GuideAxis::Vertical => snap_dx = candidate.snap_delta,
            GuideAxis::Horizontal => snap_dy = candidate.snap_delta,
        }
        if !previous
            .active
            .iter()
            .any(|k| same_candidate_slot(*k, candidate.key))
        {
            new_lock = true;
        }
    }

    let lock_until_ms = if new_lock {
        Some(now_ms + GUIDE_LOCK_MS as f64)
    } else {
        previous.lock_until_ms.filter(|&lock| now_ms < lock)
    };

    let snapped_moving = GuideBox {
        id: 0,
        bounds: Rect::new(
            proposed.x + snap_dx,
            proposed.y + snap_dy,
            proposed.w,
            proposed.h,
        ),
    };
    let guides = [GuideAxis::Vertical, GuideAxis::Horizontal]
        .into_iter()
        .filter(|axis| active.iter().any(|key| key.axis == *axis))
        .filter_map(|axis| {
            let candidate = active_candidate(
                snapped_moving,
                &others,
                axis,
                &SmartGuideState {
                    guides: Vec::new(),
                    active: active.clone(),
                    lock_until_ms,
                },
                now_ms,
            )?;
            Some(if candidate.key.kind == GuideKind::EqualSpacing {
                let tolerance = if previous
                    .active
                    .iter()
                    .any(|key| same_candidate_slot(*key, candidate.key))
                {
                    GUIDE_RELEASE_TOLERANCE
                } else {
                    GUIDE_TOLERANCE
                };
                equal_spacing_guides(snapped_moving, &others, &candidate, tolerance)
            } else {
                candidate_lines(snapped_moving, &others, &candidate)
            })
        })
        .flatten()
        .collect();

    GuideResolution {
        snap_dx,
        snap_dy,
        state: SmartGuideState {
            guides,
            active,
            lock_until_ms,
        },
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
                let marker = if guide.kind == GuideKind::EqualSpacing {
                    "url(#smart-guide-arrow)"
                } else {
                    "none"
                };
                view! {
                    <g pointer-events="none">
                        <line
                            x1=guide.start.x
                            y1=guide.start.y
                            x2=guide.end.x
                            y2=guide.end.y
                            stroke="#7aa2f7"
                            stroke-width="1.25"
                            stroke-dasharray="4 3"
                            stroke-linecap="round"
                            marker-start=marker
                            marker-end=marker
                            opacity="0.9"
                        />
                    </g>
                }
                .into_view()
            })
            .collect_view();

        Some(
            view! {
                <g pointer-events="none">
                    <defs>
                        <marker
                            id="smart-guide-arrow"
                            marker-width="6"
                            marker-height="6"
                            ref-x="3"
                            ref-y="3"
                            orient="auto-start-reverse"
                            marker-units="userSpaceOnUse"
                        >
                            <path d="M 0 0 L 6 3 L 0 6 z" fill="#7aa2f7"/>
                        </marker>
                    </defs>
                    {lines}
                </g>
            }
            .into_view(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(id: ElementId, x: f64, y: f64, w: f64, h: f64) -> (ElementId, Rect) {
        (id, Rect::new(x, y, w, h))
    }

    fn guide(id: ElementId, bounds: Rect) -> GuideBox {
        GuideBox { id, bounds }
    }

    #[test]
    fn alignment_snaps_edges_and_centers() {
        for axis in [GuideAxis::Vertical, GuideAxis::Horizontal] {
            for (feature, moving_start, moving_size, target_start, target_size) in [
                (GuideFeature::Start, 100.0, 40.0, 106.0, 80.0),
                (GuideFeature::End, 0.0, 100.0, 66.0, 40.0),
                (GuideFeature::Center, 100.0, 40.0, 86.0, 80.0),
            ] {
                let moving = match axis {
                    GuideAxis::Vertical => Rect::new(moving_start, 0.0, moving_size, 20.0),
                    GuideAxis::Horizontal => Rect::new(0.0, moving_start, 20.0, moving_size),
                };
                let target = match axis {
                    GuideAxis::Vertical => Rect::new(target_start, 0.0, target_size, 20.0),
                    GuideAxis::Horizontal => Rect::new(0.0, target_start, 20.0, target_size),
                };
                let candidate =
                    alignment_candidate(guide(0, moving), guide(1, target), axis, GUIDE_TOLERANCE)
                        .unwrap();
                assert_eq!(candidate.key.moving_feature, feature);
                assert_eq!(candidate.snap_delta, 6.0);
            }
        }
    }

    #[test]
    fn equal_spacing_renders_all_matching_inline_gaps() {
        let vertical = [
            rect(1, 0.0, 0.0, 100.0, 100.0),
            rect(2, 50.0, 0.0, 50.0, 100.0),
            rect(3, 200.0, 0.0, 20.0, 100.0),
            rect(4, 200.0, 0.0, 70.0, 100.0),
        ];
        let resolution = resolve_move(
            Rect::new(141.0, 20.0, 20.0, 20.0),
            &vertical,
            true,
            &SmartGuideState::default(),
        );
        assert_eq!((resolution.snap_dx, resolution.snap_dy), (-1.0, 0.0));
        assert_eq!(resolution.state.guides.len(), 8);
    }

    #[test]
    fn equal_spacing_renders_matching_gap_outside_moving_pair() {
        let others = [
            rect(1, 0.0, 0.0, 20.0, 100.0),
            rect(2, 100.0, 0.0, 20.0, 100.0),
            rect(3, 200.0, 0.0, 20.0, 100.0),
            rect(4, 300.0, 0.0, 20.0, 100.0),
            rect(5, 200.0, 200.0, 20.0, 20.0),
            rect(6, 300.0, 200.0, 20.0, 20.0),
        ];
        let resolution = resolve_move(
            Rect::new(45.0, 20.0, 20.0, 20.0),
            &others,
            true,
            &SmartGuideState::default(),
        );
        assert_eq!(resolution.snap_dx, 5.0);
        assert_eq!(resolution.state.guides.len(), 6);
        assert!(resolution
            .state
            .guides
            .iter()
            .all(|line| line.kind == GuideKind::EqualSpacing));
    }

    #[test]
    fn fresh_equal_spacing_uses_acquisition_tolerance_for_matching_gaps() {
        let others = [
            rect(1, 0.0, 0.0, 20.0, 100.0),
            rect(2, 100.0, 0.0, 20.0, 100.0),
            rect(3, 200.0, 0.0, 20.0, 100.0),
            rect(4, 300.0, 0.0, 20.0, 100.0),
        ];
        let resolution = resolve_move(
            Rect::new(44.0, 24.0, 20.0, 1.0),
            &others,
            true,
            &SmartGuideState::default(),
        );
        assert_eq!(resolution.snap_dx, 6.0);
        assert_eq!(resolution.state.guides.len(), 6);
    }

    #[test]
    fn equal_spacing_matches_gaps_with_floating_point_roundoff() {
        let moving = guide(0, Rect::new(54.1, 20.0, 20.0, 20.0));
        let left = guide(1, Rect::new(0.1, 0.0, 20.0, 100.0));
        let right = guide(2, Rect::new(100.1, 0.0, 20.0, 100.0));
        let shifted_left = guide(3, Rect::new(200.1, 0.0, 20.0, 100.0));
        let shifted_right = guide(4, Rect::new(300.1, 0.0, 20.0, 100.0));
        let selected =
            equal_spacing_candidate(moving, left, right, GuideAxis::Vertical, GUIDE_TOLERANCE)
                .unwrap();
        let others = [left, right, shifted_left, shifted_right];
        let lines = equal_spacing_guides(moving, &others, &selected, GUIDE_TOLERANCE);
        assert_eq!(lines.len(), 6);
    }

    #[test]
    fn equal_spacing_rejects_invalid_gaps_and_wrong_rows() {
        let left = guide(1, Rect::new(0.0, 0.0, 20.0, 20.0));
        let right = guide(2, Rect::new(100.0, 0.0, 20.0, 20.0));
        let gap = right.bounds.x - left.bounds.right();
        for moving_size in [gap, gap + 20.0] {
            let position = (left.bounds.right() + right.bounds.x - moving_size) / 2.0 - 4.0;
            assert!(equal_spacing_candidate(
                guide(0, Rect::new(position, 0.0, moving_size, 20.0)),
                left,
                right,
                GuideAxis::Vertical,
                GUIDE_TOLERANCE,
            )
            .is_none());
        }
        assert!(equal_spacing_candidate(
            guide(0, Rect::new(46.0, 30.0, 20.0, 20.0)),
            left,
            right,
            GuideAxis::Vertical,
            GUIDE_TOLERANCE,
        )
        .is_none());
    }

    #[test]
    fn alignment_snaps_continuously_until_released() {
        let others = [rect(1, 200.0, 140.0, 20.0, 20.0)];
        let proposed = Rect::new(194.0, 100.0, 20.0, 20.0);
        let first = resolve_move(proposed, &others, true, &SmartGuideState::default());
        assert_eq!(first.snap_dx, 6.0);
        assert_eq!(first.state.active.len(), 1);
        assert!(!first.state.guides.is_empty());

        let moving_again = resolve_move(
            Rect::new(195.0, 100.0, 20.0, 20.0),
            &others,
            true,
            &first.state,
        );
        assert_eq!(moving_again.snap_dx, 5.0);

        let released = resolve_move(
            Rect::new(232.0, 100.0, 20.0, 20.0),
            &others,
            true,
            &moving_again.state,
        );
        assert_eq!(released.snap_dx, 0.0);
        assert!(released.state.active.is_empty());
    }

    #[test]
    fn alignment_feature_changes_remain_continuous() {
        let others = [rect(1, 200.0, 140.0, 20.0, 20.0)];
        let first = resolve_move(
            Rect::new(180.0, 100.0, 20.0, 20.0),
            &others,
            true,
            &SmartGuideState::default(),
        );
        let feature_changed = resolve_move(
            Rect::new(194.0, 100.0, 20.0, 20.0),
            &others,
            true,
            &first.state,
        );
        assert_eq!(feature_changed.snap_dx, 6.0);
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
        let candidate =
            alignment_candidate(moving, target, GuideAxis::Vertical, GUIDE_RELEASE_TOLERANCE)
                .unwrap();
        assert_eq!(candidate.lines.len(), 1);
        assert_eq!(candidate.lines[0].start.y, moving.bounds.bottom());
        assert_eq!(candidate.lines[0].end.y, target.bounds.y);

        let overlapping = GuideBox {
            id: 1,
            bounds: Rect::new(100.0, 10.0, 20.0, 20.0),
        };
        let candidate = alignment_candidate(
            moving,
            overlapping,
            GuideAxis::Vertical,
            GUIDE_RELEASE_TOLERANCE,
        )
        .unwrap();
        assert_eq!(candidate.lines.len(), 1);
        assert_eq!(
            candidate.lines[0].start.y,
            moving.bounds.y.min(overlapping.bounds.y)
        );
        assert_eq!(
            candidate.lines[0].end.y,
            moving.bounds.bottom().max(overlapping.bounds.bottom())
        );
    }

    #[test]
    fn equal_spacing_snaps_continuously_at_the_middle_gap() {
        let others = [
            rect(1, 0.0, 0.0, 20.0, 100.0),
            rect(2, 100.0, 0.0, 20.0, 100.0),
        ];
        let proposed = Rect::new(44.0, 20.0, 20.0, 20.0);
        let first = resolve_move(proposed, &others, true, &SmartGuideState::default());
        assert_eq!(first.snap_dx, 6.0);
        assert_eq!(first.state.guides.len(), 2);
        assert_eq!(first.state.guides[0].kind, GuideKind::EqualSpacing);

        let moving_again = resolve_move(
            Rect::new(45.0, 20.0, 20.0, 20.0),
            &others,
            true,
            &first.state,
        );
        assert_eq!(moving_again.snap_dx, 5.0);

        let released = resolve_move(
            Rect::new(60.0, 20.0, 20.0, 20.0),
            &others,
            true,
            &moving_again.state,
        );
        assert_eq!(released.snap_dx, 0.0);
        assert!(released.state.active.is_empty());
    }

    #[test]
    fn active_equal_spacing_uses_release_tolerance() {
        let others = [
            rect(1, 0.0, 0.0, 20.0, 20.0),
            rect(2, 100.0, 0.0, 20.0, 20.0),
        ];
        let first = resolve_move(
            Rect::new(44.0, 0.0, 20.0, 20.0),
            &others,
            true,
            &SmartGuideState::default(),
        );
        let moving_again = resolve_move(
            Rect::new(43.0, 27.0, 20.0, 20.0),
            &others,
            true,
            &first.state,
        );
        assert_eq!(moving_again.snap_dx, 7.0);
        assert_eq!(moving_again.state.active.len(), 1);
        assert_eq!(moving_again.state.active[0].kind, GuideKind::EqualSpacing);
        assert_eq!(moving_again.state.guides.len(), 2);

        let fresh = resolve_move(
            Rect::new(43.0, 27.0, 20.0, 20.0),
            &others,
            true,
            &SmartGuideState::default(),
        );
        assert_eq!(fresh.snap_dx, 0.0);
        assert!(fresh.state.active.is_empty());
        assert!(fresh.state.guides.is_empty());
    }

    #[test]
    fn fresh_equal_spacing_rejects_offsets_outside_acquisition_band() {
        for (others, proposed) in [
            (
                [
                    rect(1, 0.0, 0.0, 20.0, 20.0),
                    rect(2, 100.0, 0.0, 20.0, 20.0),
                ],
                Rect::new(45.0, 27.0, 20.0, 20.0),
            ),
            (
                [
                    rect(1, 0.0, 0.0, 20.0, 20.0),
                    rect(2, 0.0, 100.0, 20.0, 20.0),
                ],
                Rect::new(27.0, 45.0, 20.0, 20.0),
            ),
        ] {
            let resolution = resolve_move(proposed, &others, true, &SmartGuideState::default());
            assert_eq!((resolution.snap_dx, resolution.snap_dy), (0.0, 0.0));
            assert!(resolution.state.active.is_empty());
            assert!(resolution.state.guides.is_empty());
        }
    }

    #[test]
    fn exact_equal_spacing_beats_a_nearby_alignment_candidate() {
        let others = [
            rect(1, 0.0, 0.0, 20.0, 100.0),
            rect(2, 100.0, 0.0, 20.0, 100.0),
            rect(3, 52.0, 200.0, 20.0, 20.0),
        ];
        let proposed = Rect::new(50.0, 20.0, 20.0, 20.0);
        let resolution = resolve_move(proposed, &others, true, &SmartGuideState::default());
        assert_eq!(resolution.state.guides.len(), 2);
        assert_eq!(resolution.state.guides[0].kind, GuideKind::EqualSpacing);
        assert_eq!(resolution.state.guides[1].kind, GuideKind::EqualSpacing);
    }

    #[test]
    fn multi_element_alignment_guides_connect_adjacent_elements() {
        let others = [
            rect(1, 140.0, 0.0, 20.0, 50.0),
            rect(2, 140.0, 100.0, 20.0, 50.0),
        ];
        let proposed = Rect::new(134.0, 200.0, 20.0, 50.0);
        let resolution = resolve_move(proposed, &others, true, &SmartGuideState::default());

        assert_eq!(resolution.snap_dx, 6.0);
        assert_eq!(resolution.state.guides.len(), 2);
        assert_eq!(resolution.state.guides[0].start.y, 50.0);
        assert_eq!(resolution.state.guides[0].end.y, 100.0);
        assert_eq!(resolution.state.guides[1].start.y, 150.0);
        assert_eq!(resolution.state.guides[1].end.y, 200.0);
    }
    #[test]
    fn leaving_a_candidate_clears_the_active_guide() {
        let others = [rect(1, 200.0, 100.0, 20.0, 20.0)];
        let proposed = Rect::new(194.0, 100.0, 20.0, 20.0);
        let first = resolve_move(proposed, &others, true, &SmartGuideState::default());
        let cleared = resolve_move(
            Rect::new(170.0, 170.0, 20.0, 20.0),
            &others,
            true,
            &first.state,
        );
        assert!(cleared.state.guides.is_empty());
        assert!(cleared.state.active.is_empty());
        assert_eq!(cleared.snap_dx, 0.0);
    }
}
