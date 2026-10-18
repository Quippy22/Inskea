//! Candidate scan: one alignment, spacing, or chain candidate per target,
//! then a single winner per axis with hysteresis against the previous state.

use super::geometry::{
    along_end, along_start, features, gap_crossed, gap_span, inline_pairs, perpendicular_gap,
    share_lane, sorted_along,
};
use super::{
    GuideAxis, GuideBox, GuideCandidate, GuideFeature, GuideKey, GuideKind, SmartGuideState,
    Tolerances, FEATURES,
};
use crate::model::Rect;

/// One axis' neighbourhood scan: the boxes sorted along the axis for the
/// crossed-gap tests, and the inline pairs they form. The same scan backs the
/// candidate search and the guides later drawn for its winner.
pub(super) struct AxisScan {
    pub(super) axis: GuideAxis,
    pub(super) ordered: Vec<GuideBox>,
    pub(super) pairs: Vec<(GuideBox, GuideBox)>,
}

impl AxisScan {
    pub(super) fn new(axis: GuideAxis, near: &[GuideBox], lane: f64) -> Self {
        Self {
            axis,
            ordered: sorted_along(near, axis),
            pairs: inline_pairs(near, axis, lane),
        }
    }

    /// Pick the candidate for this axis. A candidate that was active on the
    /// previous frame stays active while it remains within the wider release
    /// band; otherwise the closest candidate inside the acquisition band wins.
    pub(super) fn candidate(
        &self,
        moving: Rect,
        targets: &[GuideBox],
        previous: &SmartGuideState,
        tolerances: &Tolerances,
    ) -> Option<GuideCandidate> {
        let candidates = self.candidates(moving, targets, tolerances);

        if let Some(active) = previous.active.iter().find(|key| key.axis == self.axis) {
            if let Some(candidate) = candidates.iter().find(|c| c.key == *active) {
                if candidate.snap_delta.abs() <= tolerances.release {
                    return Some(*candidate);
                }
            }
        }

        candidates
            .into_iter()
            .filter(|candidate| candidate.snap_delta.abs() <= tolerances.acquire)
            .min_by(|a, b| candidate_order(a, b))
    }

    fn candidates(
        &self,
        moving: Rect,
        targets: &[GuideBox],
        tolerances: &Tolerances,
    ) -> Vec<GuideCandidate> {
        let mut candidates = targets
            .iter()
            .filter_map(|target| {
                alignment_candidate(moving, *target, self.axis, tolerances.release)
            })
            .collect::<Vec<_>>();

        for (left, right) in &self.pairs {
            if let Some(candidate) = spacing_candidate(
                moving,
                *left,
                *right,
                self.axis,
                tolerances.lane,
                tolerances.release,
            ) {
                candidates.push(candidate);
            }
            if let Some(candidate) = chain_candidate(
                moving,
                &self.ordered,
                *left,
                *right,
                self.axis,
                tolerances.lane,
                tolerances.release,
            ) {
                candidates.push(candidate);
            }
        }

        candidates
    }
}

/// Per-target order: matching features (left to left) win over cross features
/// (left to right), then the smallest offset wins.
fn per_target_order(a: &GuideCandidate, b: &GuideCandidate) -> std::cmp::Ordering {
    feature_rank(a.key)
        .cmp(&feature_rank(b.key))
        .then_with(|| a.snap_delta.abs().total_cmp(&b.snap_delta.abs()))
}

fn alignment_candidate(
    moving: Rect,
    target: GuideBox,
    axis: GuideAxis,
    tolerance: f64,
) -> Option<GuideCandidate> {
    let moving_values = features(moving, axis);
    let target_values = features(target.bounds, axis);
    let mut best: Option<GuideCandidate> = None;

    for (moving_feature, moving_value) in FEATURES.into_iter().zip(moving_values) {
        for (target_feature, target_value) in FEATURES.into_iter().zip(target_values) {
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
            };
            if best
                .as_ref()
                .is_none_or(|current| per_target_order(&candidate, current).is_lt())
            {
                best = Some(candidate);
            }
        }
    }

    best
}

/// A candidate for centring the moving box in the gap between two inline
/// boxes, so the free space on either side is equal.
fn spacing_candidate(
    moving: Rect,
    left: GuideBox,
    right: GuideBox,
    axis: GuideAxis,
    lane: f64,
    tolerance: f64,
) -> Option<GuideCandidate> {
    if !share_lane(moving, left.bounds, right.bounds, axis, lane)
        || perpendicular_gap(left.bounds, right.bounds, axis) > lane
    {
        return None;
    }

    let (start, end) = gap_span(left.bounds, right.bounds, axis);
    let moving_start = along_start(moving, axis);
    let moving_size = along_end(moving, axis) - moving_start;
    let gap = end - start;
    if gap <= 0.0
        || moving_size >= gap
        || moving_start + moving_size < start - tolerance
        || moving_start > end + tolerance
    {
        return None;
    }

    let desired_start = (start + end - moving_size) / 2.0;
    let snap_delta = desired_start - moving_start;
    if snap_delta.abs() > tolerance {
        return None;
    }

    Some(GuideCandidate {
        key: GuideKey {
            axis,
            kind: GuideKind::EqualSpacing,
            moving_feature: GuideFeature::Start,
            target_feature: GuideFeature::End,
            target_a: left.id,
            target_b: right.id,
        },
        snap_delta,
    })
}

/// A candidate for closing the gap between the moving box and one end of an
/// inline pair, to match the pair's gap and grow the chain by one link.
fn chain_candidate(
    moving: Rect,
    nearby: &[GuideBox],
    left: GuideBox,
    right: GuideBox,
    axis: GuideAxis,
    lane: f64,
    tolerance: f64,
) -> Option<GuideCandidate> {
    let (left_end, right_start) = gap_span(left.bounds, right.bounds, axis);
    let gap = right_start - left_end;
    if gap <= 0.0 {
        return None;
    }

    let moving_start = along_start(moving, axis);
    let moving_end = along_end(moving, axis);
    let (near, far, delta) = if moving_start >= along_end(right.bounds, axis) {
        (
            right,
            left,
            along_end(right.bounds, axis) + gap - moving_start,
        )
    } else if moving_end <= along_start(left.bounds, axis) {
        (
            left,
            right,
            along_start(left.bounds, axis) - gap - moving_end,
        )
    } else {
        return None;
    };

    // The tolerance test is cheap and rejects almost every pair, so it runs
    // before the crossing scan, which walks the whole scene.
    if delta.abs() > tolerance || perpendicular_gap(moving, near.bounds, axis) > lane {
        return None;
    }

    let crossed = gap_crossed(nearby, left.bounds, axis, left_end, right_start, lane);
    if crossed {
        return None;
    }

    Some(GuideCandidate {
        key: GuideKey {
            axis,
            kind: GuideKind::ChainSpacing,
            moving_feature: GuideFeature::Start,
            target_feature: GuideFeature::End,
            target_a: near.id,
            target_b: far.id,
        },
        snap_delta: delta,
    })
}

fn feature_rank(key: GuideKey) -> u8 {
    u8::from(key.moving_feature != key.target_feature)
}

fn kind_rank(kind: GuideKind) -> u8 {
    match kind {
        // Equal spacing is the more specific relationship, so it wins ties.
        GuideKind::EqualSpacing | GuideKind::ChainSpacing => 0,
        GuideKind::Alignment => 1,
    }
}

/// Total order over competing candidates: nearest snap wins, then equal
/// spacing, then matching features, then target IDs. Only the last three can
/// tie, and they can only tie for candidates with identical slots.
fn candidate_order(a: &GuideCandidate, b: &GuideCandidate) -> std::cmp::Ordering {
    a.snap_delta
        .abs()
        .total_cmp(&b.snap_delta.abs())
        .then_with(|| kind_rank(a.key.kind).cmp(&kind_rank(b.key.kind)))
        .then_with(|| feature_rank(a.key).cmp(&feature_rank(b.key)))
        .then_with(|| a.key.target_a.cmp(&b.key.target_a))
        .then_with(|| a.key.target_b.cmp(&b.key.target_b))
        .then_with(|| a.key.moving_feature.cmp(&b.key.moving_feature))
        .then_with(|| a.key.target_feature.cmp(&b.key.target_feature))
}
