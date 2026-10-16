use crate::model::{ElementId, Point, Rect};
use crate::util::now_ms;
use leptos::*;

/// Maximum distance at which a proposed position can become a smart-guide candidate.
pub const GUIDE_TOLERANCE: f64 = 6.0;
/// Distance at which an already active guide releases.
pub const GUIDE_RELEASE_TOLERANCE: f64 = 9.0;
/// Objects further away than this from the moving element are ignored outright,
/// so candidates only ever come from the neighbourhood the drag is happening in.
pub const GUIDE_SEARCH_RADIUS: f64 = 300.0;
/// Time to hold a newly snapped guide before allowing release (ms).
const GUIDE_LOCK_MS: u64 = 300;
const GUIDE_ALIGNMENT_COLOR: &str = "#7aa2f7";
const GUIDE_SPACING_COLOR: &str = "#9ece6a";

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

fn axis_start(bounds: Rect, axis: GuideAxis) -> f64 {
    match axis {
        GuideAxis::Vertical => bounds.x,
        GuideAxis::Horizontal => bounds.y,
    }
}

fn axis_end(bounds: Rect, axis: GuideAxis) -> f64 {
    match axis {
        GuideAxis::Vertical => bounds.right(),
        GuideAxis::Horizontal => bounds.bottom(),
    }
}

fn perpendicular(axis: GuideAxis) -> GuideAxis {
    match axis {
        GuideAxis::Vertical => GuideAxis::Horizontal,
        GuideAxis::Horizontal => GuideAxis::Vertical,
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

/// Shortest distance between two rects, zero when they touch or overlap.
fn rect_distance(a: Rect, b: Rect) -> f64 {
    let gap_x = (a.x - b.right()).max(b.x - a.right()).max(0.0);
    let gap_y = (a.y - b.bottom()).max(b.y - a.bottom()).max(0.0);
    (gap_x * gap_x + gap_y * gap_y).sqrt()
}

fn equal_spacing_gap(
    moving: GuideBox,
    left: GuideBox,
    right: GuideBox,
    axis: GuideAxis,
    tolerance: f64,
) -> Option<(f64, f64, f64, f64, f64)> {
    let start = axis_end(left.bounds, axis);
    let end = axis_start(right.bounds, axis);
    let (moving_start, moving_size, guide_position) = match axis {
        GuideAxis::Vertical => (moving.bounds.x, moving.bounds.w, moving.bounds.center().y),
        GuideAxis::Horizontal => (moving.bounds.y, moving.bounds.h, moving.bounds.center().x),
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

fn spacing_line(start: f64, end: f64, guide_position: f64, axis: GuideAxis) -> GuideLine {
    match axis {
        GuideAxis::Vertical => GuideLine {
            start: Point::new(start, guide_position),
            end: Point::new(end, guide_position),
            kind: GuideKind::EqualSpacing,
        },
        GuideAxis::Horizontal => GuideLine {
            start: Point::new(guide_position, start),
            end: Point::new(guide_position, end),
            kind: GuideKind::EqualSpacing,
        },
    }
}

fn equal_spacing_lines(
    start: f64,
    end: f64,
    moving_size: f64,
    guide_position: f64,
    axis: GuideAxis,
) -> Vec<GuideLine> {
    let desired_start = (start + end - moving_size) / 2.0;
    vec![
        spacing_line(start, desired_start, guide_position, axis),
        spacing_line(desired_start + moving_size, end, guide_position, axis),
    ]
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

fn ordered_pair(left: &GuideBox, right: &GuideBox, axis: GuideAxis) -> (GuideBox, GuideBox) {
    if axis_start(left.bounds, axis) <= axis_start(right.bounds, axis) {
        (*left, *right)
    } else {
        (*right, *left)
    }
}

/// Every pair of boxes, ordered along the axis, skipping pairs that do not share
/// a lane: those can never bound a spacing gap.
fn inline_pairs<'a>(
    others: &'a [GuideBox],
    axis: GuideAxis,
    tolerance: f64,
) -> impl Iterator<Item = (GuideBox, GuideBox)> + 'a {
    others
        .iter()
        .enumerate()
        .flat_map(move |(index, left)| {
            others
                .iter()
                .skip(index + 1)
                .map(move |right| ordered_pair(left, right, axis))
        })
        .filter(move |(left, right)| {
            perpendicular_gap(left.bounds, right.bounds, axis) <= tolerance
        })
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

    for (left, right) in inline_pairs(others, axis, tolerance) {
        if let Some(candidate) = equal_spacing_candidate(moving, left, right, axis, tolerance) {
            candidates.push(candidate);
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
    for (left, right) in inline_pairs(others, selected.key.axis, tolerance) {
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
    guides
}

/// Indicators for equal spacing that already exists in the scene, so a run the
/// moving box is not part of stays visible. Anchored to the pairs only, never to
/// the moving box, so the lane does not shift while the drag proceeds.
///
/// Only neighbouring pairs count: two evenly spaced boxes two apart measure the
/// same distance as their neighbours, but an arrow between them would cross the
/// object in between.
fn equal_spacing_run_lines(others: &[GuideBox], axis: GuideAxis) -> Vec<GuideLine> {
    let cross = perpendicular(axis);
    let mut runs: Vec<(f64, Vec<GuideLine>)> = Vec::new();
    for (left, right) in inline_pairs(others, axis, GUIDE_TOLERANCE) {
        let start = axis_end(left.bounds, axis);
        let end = axis_start(right.bounds, axis);
        let gap = end - start;
        if gap <= 0.0 {
            continue;
        }
        let crossed = others.iter().any(|other| {
            perpendicular_gap(left.bounds, other.bounds, axis) <= GUIDE_TOLERANCE
                && axis_start(other.bounds, axis) > start
                && axis_end(other.bounds, axis) < end
        });
        if crossed {
            continue;
        }

        let lane_start = axis_start(left.bounds, cross).max(axis_start(right.bounds, cross));
        let lane_end = axis_end(left.bounds, cross).min(axis_end(right.bounds, cross));
        let line = spacing_line(start, end, (lane_start + lane_end) / 2.0, axis);
        match runs
            .iter_mut()
            .find(|(distance, _)| equal_spacing_gap_matches(*distance, gap))
        {
            Some((_, lines)) => lines.push(line),
            None => runs.push((gap, vec![line])),
        }
    }

    runs.into_iter()
        .filter(|(_, lines)| lines.len() > 1)
        .flat_map(|(_, lines)| lines)
        .collect()
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
/// Objects beyond GUIDE_SEARCH_RADIUS are dropped before anything is scanned, so
/// with nothing nearby the resolution is empty and clears any previous guide.
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
        .filter(|other| rect_distance(proposed, other.bounds) <= GUIDE_SEARCH_RADIUS)
        .collect();
    if others.is_empty() {
        return GuideResolution {
            snap_dx: 0.0,
            snap_dy: 0.0,
            state: SmartGuideState::default(),
        };
    }
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
    let mut guides: Vec<GuideLine> = [GuideAxis::Vertical, GuideAxis::Horizontal]
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

    guides.extend(
        [GuideAxis::Vertical, GuideAxis::Horizontal]
            .into_iter()
            .filter(|axis| {
                active
                    .iter()
                    .any(|key| key.axis == *axis && key.kind == GuideKind::Alignment)
            })
            .flat_map(|axis| equal_spacing_run_lines(&others, axis)),
    );

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
                let stroke = match guide.kind {
                    GuideKind::Alignment => GUIDE_ALIGNMENT_COLOR,
                    GuideKind::EqualSpacing => GUIDE_SPACING_COLOR,
                };
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
                            stroke=stroke
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
                            <path d="M 0 0 L 6 3 L 0 6 z" fill=GUIDE_SPACING_COLOR/>
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

    #[test]
    fn run_lines_span_the_shared_gap() {
        let first = guide(1, Rect::new(0.0, 0.0, 20.0, 20.0));
        let second = guide(2, Rect::new(100.0, 0.0, 20.0, 20.0));
        let third = guide(3, Rect::new(200.0, 0.0, 20.0, 20.0));
        let others = [first, second, third];
        let lines = equal_spacing_run_lines(&others, GuideAxis::Vertical);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].start, Point::new(first.bounds.right(), 10.0));
        assert_eq!(lines[0].end, Point::new(second.bounds.x, 10.0));
        assert_eq!(lines[1].start, Point::new(second.bounds.right(), 10.0));
        assert_eq!(lines[1].end, Point::new(third.bounds.x, 10.0));
        assert!(lines
            .iter()
            .all(|line| line.kind == GuideKind::EqualSpacing));
    }

    #[test]
    fn run_lines_drop_a_single_inline_pair() {
        let others = [
            guide(1, Rect::new(0.0, 0.0, 20.0, 20.0)),
            guide(2, Rect::new(100.0, 0.0, 20.0, 20.0)),
            guide(3, Rect::new(600.0, 0.0, 20.0, 20.0)),
        ];
        assert!(equal_spacing_run_lines(&others, GuideAxis::Vertical).is_empty());
    }

    #[test]
    fn run_lines_exclude_pairs_outside_the_row() {
        let others = [
            guide(1, Rect::new(0.0, 0.0, 20.0, 20.0)),
            guide(2, Rect::new(100.0, 0.0, 20.0, 20.0)),
            guide(3, Rect::new(0.0, 200.0, 20.0, 20.0)),
            guide(4, Rect::new(100.0, 200.0, 20.0, 20.0)),
        ];
        let lines = equal_spacing_run_lines(&others, GuideAxis::Vertical);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].start, Point::new(20.0, 10.0));
        assert_eq!(lines[0].end, Point::new(100.0, 10.0));
        assert_eq!(lines[1].start, Point::new(20.0, 210.0));
        assert_eq!(lines[1].end, Point::new(100.0, 210.0));
    }

    #[test]
    fn run_lines_match_gaps_with_floating_point_roundoff() {
        let first = guide(1, Rect::new(0.1, 0.0, 20.0, 100.0));
        let second = guide(2, Rect::new(100.1, 0.0, 20.0, 100.0));
        let third = guide(3, Rect::new(200.1, 0.0, 20.0, 100.0));
        let others = [first, second, third];
        let lines = equal_spacing_run_lines(&others, GuideAxis::Vertical);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].start, Point::new(first.bounds.right(), 50.0));
        assert_eq!(lines[0].end, Point::new(second.bounds.x, 50.0));
        assert_eq!(lines[1].start, Point::new(second.bounds.right(), 50.0));
        assert_eq!(lines[1].end, Point::new(third.bounds.x, 50.0));
        assert!(lines
            .iter()
            .all(|line| (line.end.x - line.start.x - 80.0).abs() < 1e-6));
    }

    #[test]
    fn run_lines_skip_a_pair_that_jumps_an_object() {
        let others = [
            guide(1, Rect::new(0.0, 0.0, 20.0, 20.0)),
            guide(2, Rect::new(100.0, 0.0, 20.0, 20.0)),
            guide(3, Rect::new(200.0, 0.0, 20.0, 20.0)),
            guide(4, Rect::new(300.0, 0.0, 20.0, 20.0)),
        ];
        let lines = equal_spacing_run_lines(&others, GuideAxis::Vertical);
        let spans: Vec<_> = lines
            .iter()
            .map(|line| (line.start.x, line.end.x))
            .collect();
        assert_eq!(spans, vec![(20.0, 100.0), (120.0, 200.0), (220.0, 300.0)]);
        assert!(lines
            .iter()
            .all(|line| (line.end.x - line.start.x - 80.0).abs() < 1e-9));
    }

    #[test]
    fn run_indicator_appears_while_an_alignment_is_active() {
        let others = [
            rect(1, 0.0, 0.0, 20.0, 20.0),
            rect(2, 100.0, 0.0, 20.0, 20.0),
            rect(3, 200.0, 0.0, 20.0, 20.0),
            rect(4, 300.0, 0.0, 20.0, 20.0),
        ];
        let resolution = resolve_move(
            Rect::new(106.0, 250.0, 20.0, 20.0),
            &others,
            true,
            &SmartGuideState::default(),
        );
        assert_eq!(resolution.snap_dx, -6.0);
        assert_eq!(resolution.state.guides.len(), 4);
        assert_eq!(resolution.state.guides[0].start, Point::new(100.0, 20.0));
        assert_eq!(resolution.state.guides[0].end, Point::new(100.0, 250.0));
        assert_eq!(resolution.state.guides[1].start, Point::new(20.0, 10.0));
        assert_eq!(resolution.state.guides[1].end, Point::new(100.0, 10.0));
        assert_eq!(resolution.state.guides[2].start, Point::new(120.0, 10.0));
        assert_eq!(resolution.state.guides[2].end, Point::new(200.0, 10.0));
        assert_eq!(resolution.state.guides[3].start, Point::new(220.0, 10.0));
        assert_eq!(resolution.state.guides[3].end, Point::new(300.0, 10.0));

        // Compared on the run lines only: the point is that they are anchored to
        // the row and do not follow the dragged box.
        let nearer = resolve_move(
            Rect::new(106.0, 100.0, 20.0, 20.0),
            &others,
            true,
            &SmartGuideState::default(),
        );
        assert_eq!(nearer.snap_dx, resolution.snap_dx);
        let run_lines = |state: &SmartGuideState| -> Vec<GuideLine> {
            state
                .guides
                .iter()
                .filter(|line| line.kind == GuideKind::EqualSpacing)
                .copied()
                .collect()
        };
        assert_eq!(run_lines(&nearer.state), run_lines(&resolution.state));
    }

    #[test]
    fn run_indicator_needs_two_candidates_on_the_axis() {
        let row = [
            rect(1, 0.0, 0.0, 20.0, 20.0),
            rect(2, 100.0, 0.0, 20.0, 20.0),
            rect(3, 200.0, 0.0, 20.0, 20.0),
            rect(4, 300.0, 0.0, 20.0, 20.0),
        ];
        let unaligned = Rect::new(150.0, 100.0, 20.0, 20.0);
        assert!(row
            .iter()
            .all(|(_, bounds)| rect_distance(unaligned, *bounds) <= GUIDE_SEARCH_RADIUS));
        let inactive = resolve_move(unaligned, &row, true, &SmartGuideState::default());
        assert!(inactive.state.active.is_empty());
        assert!(inactive.state.guides.is_empty());

        let pair = [
            rect(1, 200.0, 0.0, 20.0, 20.0),
            rect(2, 300.0, 0.0, 20.0, 20.0),
        ];
        let single = resolve_move(
            Rect::new(194.0, 250.0, 20.0, 20.0),
            &pair,
            true,
            &SmartGuideState::default(),
        );
        assert_eq!(single.state.active[0].kind, GuideKind::Alignment);
        assert!(single
            .state
            .guides
            .iter()
            .all(|line| line.kind == GuideKind::Alignment));
    }

    #[test]
    fn run_indicator_is_gated_out_for_an_equal_spacing_axis() {
        let others = [
            rect(1, 0.0, 0.0, 20.0, 20.0),
            rect(2, 100.0, 0.0, 20.0, 20.0),
            rect(3, 200.0, 0.0, 20.0, 20.0),
            rect(4, 300.0, 0.0, 20.0, 20.0),
        ];
        let resolution = resolve_move(
            Rect::new(44.0, 18.0, 20.0, 20.0),
            &others,
            true,
            &SmartGuideState::default(),
        );
        assert_eq!(resolution.state.active[0].kind, GuideKind::EqualSpacing);
        assert_eq!(resolution.state.guides.len(), 7);
        assert_eq!(
            resolution
                .state
                .guides
                .iter()
                .filter(|line| line.start.y == 10.0)
                .count(),
            0
        );
    }

    #[test]
    fn run_indicator_only_covers_the_aligned_axis() {
        let others = [
            rect(1, 0.0, 0.0, 20.0, 20.0),
            rect(2, 100.0, 0.0, 20.0, 20.0),
            rect(3, 200.0, 0.0, 20.0, 20.0),
            rect(4, 300.0, 0.0, 20.0, 20.0),
            rect(5, 40.0, 100.0, 20.0, 20.0),
            rect(6, 40.0, 240.0, 20.0, 20.0),
            rect(7, 40.0, 380.0, 20.0, 20.0),
        ];
        let resolution = resolve_move(
            Rect::new(44.0, 170.0, 20.0, 20.0),
            &others,
            true,
            &SmartGuideState::default(),
        );
        assert_eq!((resolution.snap_dx, resolution.snap_dy), (-4.0, 0.0));
        assert_eq!(resolution.state.active[0].kind, GuideKind::Alignment);
        assert_eq!(resolution.state.active[1].kind, GuideKind::EqualSpacing);
        assert_eq!(resolution.state.guides.len(), 10);
        assert_eq!(
            resolution
                .state
                .guides
                .iter()
                .filter(|line| line.start.y == 10.0 && line.end.y == 10.0)
                .count(),
            3
        );
        assert!(!resolution.state.guides.iter().any(|line| {
            line.start.x == 50.0 && line.end.x == 50.0 && line.end.y - line.start.y == 120.0
        }));
    }

    #[test]
    fn objects_beyond_the_search_radius_are_ignored() {
        let moving = Rect::new(0.0, 0.0, 20.0, 20.0);
        let inside = Rect::new(5.0, 20.0 + GUIDE_SEARCH_RADIUS - 40.0, 20.0, 20.0);
        let outside = Rect::new(5.0, 20.0 + GUIDE_SEARCH_RADIUS + 40.0, 20.0, 20.0);
        assert_eq!(rect_distance(moving, inside), GUIDE_SEARCH_RADIUS - 40.0);
        assert_eq!(rect_distance(moving, outside), GUIDE_SEARCH_RADIUS + 40.0);

        let snapped = resolve_move(
            moving,
            &[rect(1, 5.0, inside.y, 20.0, 20.0)],
            true,
            &SmartGuideState::default(),
        );
        assert_eq!((snapped.snap_dx, snapped.snap_dy), (5.0, 0.0));
        assert_eq!(snapped.state.active.len(), 1);
        assert_eq!(snapped.state.active[0].kind, GuideKind::Alignment);
        assert_eq!(snapped.state.active[0].axis, GuideAxis::Vertical);

        let ignored = resolve_move(
            moving,
            &[rect(1, 5.0, outside.y, 20.0, 20.0)],
            true,
            &SmartGuideState::default(),
        );
        assert_eq!((ignored.snap_dx, ignored.snap_dy), (0.0, 0.0));
        assert!(ignored.state.active.is_empty());
        assert!(ignored.state.guides.is_empty());
    }

    #[test]
    fn an_aligned_axis_does_not_reach_across_the_search_radius() {
        let moving = Rect::new(0.0, 0.0, 20.0, 20.0);
        let target = Rect::new(0.0, 1000.0, 20.0, 20.0);
        assert_eq!(rect_distance(moving, target), 980.0);
        let candidate = alignment_candidate(
            guide(0, moving),
            guide(1, target),
            GuideAxis::Vertical,
            GUIDE_TOLERANCE,
        )
        .unwrap();
        assert_eq!(candidate.snap_delta, 0.0);

        let resolution = resolve_move(
            moving,
            &[rect(1, target.x, target.y, 20.0, 20.0)],
            true,
            &SmartGuideState::default(),
        );
        assert_eq!((resolution.snap_dx, resolution.snap_dy), (0.0, 0.0));
        assert!(resolution.state.active.is_empty());
        assert!(resolution.state.guides.is_empty());
    }

    #[test]
    fn an_object_on_the_far_side_of_the_radius_is_still_found() {
        let moving = Rect::new(0.0, 0.0, 20.0, 20.0);
        let gap = GUIDE_SEARCH_RADIUS - 20.0;
        let left = Rect::new(-(gap + 20.0), 5.0, 20.0, 20.0);
        assert_eq!(rect_distance(moving, left), gap);

        let resolution = resolve_move(
            moving,
            &[rect(1, left.x, left.y, 20.0, 20.0)],
            true,
            &SmartGuideState::default(),
        );
        assert_eq!((resolution.snap_dx, resolution.snap_dy), (0.0, 5.0));
        assert_eq!(resolution.state.active.len(), 1);
        assert_eq!(resolution.state.active[0].axis, GuideAxis::Horizontal);
    }

    #[test]
    fn touching_and_overlapping_rects_are_in_range() {
        let moving = Rect::new(0.0, 0.0, 20.0, 20.0);
        let overlapping = Rect::new(5.0, 5.0, 20.0, 20.0);
        let touching = Rect::new(20.0, 20.0, 20.0, 20.0);
        assert_eq!(rect_distance(moving, overlapping), 0.0);
        assert_eq!(rect_distance(moving, touching), 0.0);

        let overlap = resolve_move(
            moving,
            &[rect(1, overlapping.x, overlapping.y, 20.0, 20.0)],
            true,
            &SmartGuideState::default(),
        );
        assert_eq!((overlap.snap_dx, overlap.snap_dy), (5.0, 5.0));
        assert_eq!(overlap.state.active.len(), 2);
        assert!(overlap
            .state
            .active
            .iter()
            .all(|key| key.kind == GuideKind::Alignment && key.target_a == 1));

        let touch = resolve_move(
            moving,
            &[rect(1, touching.x, touching.y, 20.0, 20.0)],
            true,
            &SmartGuideState::default(),
        );
        assert_eq!((touch.snap_dx, touch.snap_dy), (0.0, 0.0));
        assert_eq!(touch.state.active.len(), 2);
        assert!(touch
            .state
            .active
            .iter()
            .all(|key| key.kind == GuideKind::Alignment && key.target_a == 1));
    }

    #[test]
    fn a_scene_without_any_object_in_range_resolves_to_nothing() {
        let far = GUIDE_SEARCH_RADIUS + 100.0;
        let others = [
            rect(1, 20.0 + far, 0.0, 20.0, 20.0),
            rect(2, 0.0, 20.0 + far, 20.0, 20.0),
            rect(3, -20.0 - far, 0.0, 20.0, 20.0),
            rect(4, 0.0, -20.0 - far, 20.0, 20.0),
        ];
        let resolution = resolve_move(
            Rect::new(0.0, 0.0, 20.0, 20.0),
            &others,
            true,
            &SmartGuideState::default(),
        );
        assert_eq!((resolution.snap_dx, resolution.snap_dy), (0.0, 0.0));
        assert!(resolution.state.active.is_empty());
        assert!(resolution.state.guides.is_empty());
        assert_eq!(resolution.state.lock_until_ms, None);
    }

    #[test]
    fn leaving_the_radius_clears_a_previously_active_guide() {
        let snapped = resolve_move(
            Rect::new(0.0, 0.0, 20.0, 20.0),
            &[rect(1, 5.0, 20.0 + GUIDE_SEARCH_RADIUS - 40.0, 20.0, 20.0)],
            true,
            &SmartGuideState::default(),
        );
        assert_eq!(snapped.state.active.len(), 1);

        let far = GUIDE_SEARCH_RADIUS + 100.0;
        let out_of_range = resolve_move(
            Rect::new(0.0, 0.0, 20.0, 20.0),
            &[rect(1, 5.0, 20.0 + far, 20.0, 20.0)],
            true,
            &snapped.state,
        );
        assert_eq!((out_of_range.snap_dx, out_of_range.snap_dy), (0.0, 0.0));
        assert!(out_of_range.state.active.is_empty());
        assert!(out_of_range.state.guides.is_empty());
        assert_eq!(out_of_range.state.lock_until_ms, None);
    }

    #[test]
    fn run_indicators_respect_the_search_radius() {
        let row = [
            rect(1, 0.0, 0.0, 20.0, 20.0),
            rect(2, 100.0, 0.0, 20.0, 20.0),
            rect(3, 200.0, 0.0, 20.0, 20.0),
        ];
        let run_lines = |resolution: &GuideResolution| -> Vec<GuideLine> {
            resolution
                .state
                .guides
                .iter()
                .filter(|line| line.kind == GuideKind::EqualSpacing)
                .copied()
                .collect()
        };

        let nearby = resolve_move(
            Rect::new(6.0, 20.0 + GUIDE_SEARCH_RADIUS - 100.0, 20.0, 20.0),
            &row,
            true,
            &SmartGuideState::default(),
        );
        assert_eq!(nearby.state.active.len(), 1);
        assert_eq!(nearby.state.active[0].kind, GuideKind::Alignment);
        assert_eq!(nearby.state.active[0].axis, GuideAxis::Vertical);
        assert_eq!(run_lines(&nearby).len(), 2);

        let distant = resolve_move(
            Rect::new(6.0, 20.0 + GUIDE_SEARCH_RADIUS + 100.0, 20.0, 20.0),
            &row,
            true,
            &SmartGuideState::default(),
        );
        assert!(run_lines(&distant).is_empty());
    }
}

#[cfg(test)]
mod scratch {
    use super::*;

    #[test]
    fn scratch_drag_into_row() {
        let others = [
            (1, Rect::new(-240.0, -106.5, 60.0, 60.0)),
            (2, Rect::new(-40.0, -106.5, 60.0, 60.0)),
            (3, Rect::new(160.0, -106.5, 60.0, 60.0)),
        ];
        let proposed = Rect::new(-190.0, -136.5, 60.0, 60.0);
        let r = resolve_move(proposed, &others, true, &SmartGuideState::default());
        eprintln!("snap=({}, {}) guides={:#?} active={:#?}", r.snap_dx, r.snap_dy, r.state.guides, r.state.active);
    }
}

#[cfg(test)]
mod fuzz {
    use super::*;

    // Deterministic xorshift.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 % 100000) as f64 / 100.0 - 500.0
        }
    }

    #[test]
    fn fuzz_resolve_move_never_panics() {
        let mut rng = Rng(0x9E3779B97F4A7C15);
        let mut state = SmartGuideState::default();
        for _round in 0..2000 {
            let count = (rng.next().abs() as usize % 8) + 1;
            let mut others: Vec<(ElementId, Rect)> = Vec::new();
            for id in 0..count {
                let x = rng.next();
                let y = rng.next();
                let w = rng.next().abs() + 0.1;
                let h = rng.next().abs() + 0.1;
                others.push((id as ElementId + 1, Rect::new(x, y, w, h)));
            }
            let proposed = Rect::new(rng.next(), rng.next(), rng.next().abs() + 0.1, rng.next().abs() + 0.1);
            let r = resolve_move(proposed, &others, true, &state);
            for g in &r.state.guides {
                assert!(g.start.x.is_finite() && g.start.y.is_finite() && g.end.x.is_finite() && g.end.y.is_finite(), "non-finite guide {g:?} for proposed {proposed:?} others {others:?}");
            }
            assert!(r.snap_dx.is_finite() && r.snap_dy.is_finite());
            state = r.state;
        }
    }

    #[test]
    fn fuzz_equal_spacing_never_panics() {
        let mut rng = Rng(0xDEADBEEFCAFEBABE);
        for _round in 0..5000 {
            let a = Rect::new(rng.next(), rng.next(), rng.next().abs() + 1.0, rng.next().abs() + 1.0);
            let b = Rect::new(rng.next(), rng.next(), rng.next().abs() + 1.0, rng.next().abs() + 1.0);
            let m = Rect::new(rng.next(), rng.next(), rng.next().abs() + 1.0, rng.next().abs() + 1.0);
            let _ = equal_spacing_candidate(
                guide_b(m),
                guide_b(a),
                guide_b(b),
                if rng.next() > 0.0 { GuideAxis::Vertical } else { GuideAxis::Horizontal },
                GUIDE_TOLERANCE,
            );
        }
        fn guide_b(r: Rect) -> GuideBox {
            GuideBox { id: 0, bounds: r }
        }
    }
}
