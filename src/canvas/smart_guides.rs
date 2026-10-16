//! Smart guides for move drags.
//!
//! Resolution is a pure function of the proposed rectangle, the neighbouring
//! element bounds, and the zoom: [`resolve_move`] returns the snap delta plus
//! the guides to draw. Tolerances are specified in screen pixels and divided
//! by the zoom, so snapping feels the same at every zoom level. The previous
//! state is threaded back in for hysteresis: an active guide stays latched
//! until the pointer leaves a slightly wider release band.

use crate::model::{ElementId, Point, Rect};
use leptos::*;

/// Distance at which a proposed position acquires a guide, in screen pixels.
pub const GUIDE_ACQUIRE_PX: f64 = 6.0;
/// Distance at which an already active guide releases, in screen pixels.
pub const GUIDE_RELEASE_PX: f64 = 10.0;
/// Perpendicular distance at which two boxes count as sharing a lane,
/// in screen pixels.
pub const GUIDE_LANE_PX: f64 = 12.0;
/// Objects further away than this from the moving box, in screen pixels, are
/// ignored outright.
pub const GUIDE_SEARCH_RADIUS_PX: f64 = 300.0;
const GUIDE_ALIGNMENT_COLOR: &str = "#7aa2f7";
const GUIDE_SPACING_COLOR: &str = "#9ece6a";
const EPSILON: f64 = 1e-9;
const FEATURES: [GuideFeature; 3] = [GuideFeature::Start, GuideFeature::Center, GuideFeature::End];

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SmartGuideState {
    pub guides: Vec<GuideLine>,
    pub active: Vec<GuideKey>,
}

#[derive(Clone, Debug, Default, PartialEq)]
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

#[derive(Clone, Copy)]
struct GuideCandidate {
    key: GuideKey,
    snap_delta: f64,
}

/// Screen-pixel tolerances converted to world units for one zoom level.
#[derive(Clone, Copy)]
struct Tolerances {
    acquire: f64,
    release: f64,
    lane: f64,
    radius: f64,
}

impl Tolerances {
    fn for_zoom(zoom: f64) -> Self {
        let zoom = if zoom.is_finite() && zoom > 0.0 {
            zoom
        } else {
            1.0
        };
        Self {
            acquire: GUIDE_ACQUIRE_PX / zoom,
            release: GUIDE_RELEASE_PX / zoom,
            lane: GUIDE_LANE_PX / zoom,
            radius: GUIDE_SEARCH_RADIUS_PX / zoom,
        }
    }
}

fn along_start(bounds: Rect, axis: GuideAxis) -> f64 {
    match axis {
        GuideAxis::Vertical => bounds.x,
        GuideAxis::Horizontal => bounds.y,
    }
}

fn along_end(bounds: Rect, axis: GuideAxis) -> f64 {
    match axis {
        GuideAxis::Vertical => bounds.right(),
        GuideAxis::Horizontal => bounds.bottom(),
    }
}

fn cross_start(bounds: Rect, axis: GuideAxis) -> f64 {
    match axis {
        GuideAxis::Vertical => bounds.y,
        GuideAxis::Horizontal => bounds.x,
    }
}

fn cross_end(bounds: Rect, axis: GuideAxis) -> f64 {
    match axis {
        GuideAxis::Vertical => bounds.bottom(),
        GuideAxis::Horizontal => bounds.right(),
    }
}

fn feature_value(bounds: Rect, axis: GuideAxis, feature: GuideFeature) -> f64 {
    let start = along_start(bounds, axis);
    let end = along_end(bounds, axis);
    match feature {
        GuideFeature::Start => start,
        GuideFeature::Center => (start + end) / 2.0,
        GuideFeature::End => end,
    }
}

fn features(bounds: Rect, axis: GuideAxis) -> [f64; 3] {
    let start = along_start(bounds, axis);
    let end = along_end(bounds, axis);
    [start, (start + end) / 2.0, end]
}

fn axis_point(axis: GuideAxis, along: f64, cross: f64) -> Point {
    match axis {
        GuideAxis::Vertical => Point::new(along, cross),
        GuideAxis::Horizontal => Point::new(cross, along),
    }
}

/// Shortest distance between two rects, zero when they touch or overlap.
fn rect_distance(a: Rect, b: Rect) -> f64 {
    let gap_x = (a.x - b.right()).max(b.x - a.right()).max(0.0);
    let gap_y = (a.y - b.bottom()).max(b.y - a.bottom()).max(0.0);
    (gap_x * gap_x + gap_y * gap_y).sqrt()
}

/// Gap between two boxes across the perpendicular of `axis`; zero when they
/// overlap or touch on that axis.
fn perpendicular_gap(a: Rect, b: Rect, axis: GuideAxis) -> f64 {
    match axis {
        GuideAxis::Vertical if a.bottom() < b.y => b.y - a.bottom(),
        GuideAxis::Vertical if b.bottom() < a.y => a.y - b.bottom(),
        GuideAxis::Horizontal if a.right() < b.x => b.x - a.right(),
        GuideAxis::Horizontal if b.right() < a.x => a.x - b.right(),
        _ => 0.0,
    }
}

fn ordered_pair(left: GuideBox, right: GuideBox, axis: GuideAxis) -> (GuideBox, GuideBox) {
    if along_start(left.bounds, axis) <= along_start(right.bounds, axis) {
        (left, right)
    } else {
        (right, left)
    }
}

/// Every pair of boxes, ordered along the axis, whose perpendicular extents
/// are close enough to bound a spacing gap together.
fn inline_pairs(others: &[GuideBox], axis: GuideAxis, lane: f64) -> Vec<(GuideBox, GuideBox)> {
    let mut pairs = Vec::new();
    for (index, left) in others.iter().enumerate() {
        for right in others.iter().skip(index + 1) {
            if perpendicular_gap(left.bounds, right.bounds, axis) <= lane {
                pairs.push(ordered_pair(*left, *right, axis));
            }
        }
    }
    pairs
}

/// Perpendicular centre of the strip the two boxes share; the lane a spacing
/// line between them should be drawn in.
fn lane_center(left: Rect, right: Rect, axis: GuideAxis) -> f64 {
    let start = cross_start(left, axis).max(cross_start(right, axis));
    let end = cross_end(left, axis).min(cross_end(right, axis));
    (start + end) / 2.0
}

fn same_gap(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6 * a.abs().max(b.abs()).max(1.0)
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
    if perpendicular_gap(moving, left.bounds, axis) > lane
        || perpendicular_gap(moving, right.bounds, axis) > lane
        || perpendicular_gap(left.bounds, right.bounds, axis) > lane
    {
        return None;
    }

    let start = along_end(left.bounds, axis);
    let end = along_start(right.bounds, axis);
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

fn feature_rank(key: GuideKey) -> u8 {
    u8::from(key.moving_feature != key.target_feature)
}

fn kind_rank(kind: GuideKind) -> u8 {
    match kind {
        // Equal spacing is the more specific relationship, so it wins ties.
        GuideKind::EqualSpacing => 0,
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

fn candidates_for_axis(
    moving: Rect,
    others: &[GuideBox],
    pairs: &[(GuideBox, GuideBox)],
    axis: GuideAxis,
    lane: f64,
    tolerance: f64,
) -> Vec<GuideCandidate> {
    let mut candidates = others
        .iter()
        .filter_map(|target| alignment_candidate(moving, *target, axis, tolerance))
        .collect::<Vec<_>>();

    for (left, right) in pairs {
        if let Some(candidate) = spacing_candidate(moving, *left, *right, axis, lane, tolerance) {
            candidates.push(candidate);
        }
    }

    candidates
}

/// Pick the candidate for one axis. A candidate that was active on the
/// previous frame stays active while it remains within the wider release
/// band; otherwise the closest candidate inside the acquisition band wins.
fn axis_candidate(
    moving: Rect,
    others: &[GuideBox],
    pairs: &[(GuideBox, GuideBox)],
    axis: GuideAxis,
    previous: &SmartGuideState,
    tolerances: &Tolerances,
) -> Option<GuideCandidate> {
    let candidates = candidates_for_axis(
        moving,
        others,
        pairs,
        axis,
        tolerances.lane,
        tolerances.release,
    );

    if let Some(active) = previous.active.iter().find(|key| key.axis == axis) {
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

/// One continuous line through every box that shares the snapped coordinate,
/// so the guide spans the whole aligned group instead of a chain of gaps.
fn alignment_guides(moving: Rect, others: &[GuideBox], key: GuideKey) -> Vec<GuideLine> {
    let axis = key.axis;
    let coordinate = feature_value(moving, axis, key.moving_feature);
    let epsilon = EPSILON * (1.0 + coordinate.abs());

    let mut lo = cross_start(moving, axis);
    let mut hi = cross_end(moving, axis);
    for other in others {
        if features(other.bounds, axis)
            .iter()
            .any(|value| (value - coordinate).abs() <= epsilon)
        {
            lo = lo.min(cross_start(other.bounds, axis));
            hi = hi.max(cross_end(other.bounds, axis));
        }
    }

    if hi - lo <= epsilon {
        return Vec::new();
    }
    vec![GuideLine {
        start: axis_point(axis, coordinate, lo),
        end: axis_point(axis, coordinate, hi),
        kind: GuideKind::Alignment,
    }]
}

fn spacing_line(start: f64, end: f64, cross: f64, axis: GuideAxis) -> GuideLine {
    GuideLine {
        start: axis_point(axis, start, cross),
        end: axis_point(axis, end, cross),
        kind: GuideKind::EqualSpacing,
    }
}

/// The two arrow segments a gap is drawn as: the free space left of the box
/// and the free space right of it, equal once the box sits centred.
fn spacing_lines(
    start: f64,
    end: f64,
    moving_size: f64,
    cross: f64,
    axis: GuideAxis,
) -> Vec<GuideLine> {
    let desired_start = (start + end - moving_size) / 2.0;
    vec![
        spacing_line(start, desired_start, cross, axis),
        spacing_line(desired_start + moving_size, end, cross, axis),
    ]
}

/// Arrows for the snapped gap plus every other inline pair with the same gap,
/// drawn in that pair's own lane. Pairs the moving box is not inline with are
/// not shown: their arrows would float outside the row.
fn equal_spacing_guides(
    moving: Rect,
    others: &[GuideBox],
    pairs: &[(GuideBox, GuideBox)],
    candidate: &GuideCandidate,
    tolerances: &Tolerances,
) -> Vec<GuideLine> {
    let axis = candidate.key.axis;
    let selected = others
        .iter()
        .find(|b| b.id == candidate.key.target_a)
        .zip(others.iter().find(|b| b.id == candidate.key.target_b));
    let Some((left, right)) = selected else {
        return Vec::new();
    };

    let selected_gap = along_start(right.bounds, axis) - along_end(left.bounds, axis);
    let moving_start = along_start(moving, axis);
    let moving_size = along_end(moving, axis) - moving_start;
    let moving_cross = (cross_start(moving, axis) + cross_end(moving, axis)) / 2.0;
    let mut guides = Vec::new();

    for (pair_left, pair_right) in pairs {
        let start = along_end(pair_left.bounds, axis);
        let end = along_start(pair_right.bounds, axis);
        if !same_gap(end - start, selected_gap) || moving_size >= end - start {
            continue;
        }
        if perpendicular_gap(moving, pair_left.bounds, axis) > tolerances.lane
            || perpendicular_gap(moving, pair_right.bounds, axis) > tolerances.lane
        {
            continue;
        }
        let cross = if pair_left.id == candidate.key.target_a {
            moving_cross
        } else {
            lane_center(pair_left.bounds, pair_right.bounds, axis)
        };
        guides.extend(spacing_lines(start, end, moving_size, cross, axis));
    }

    guides
}

/// Highlight runs of evenly spaced boxes: at least two inline pairs sharing
/// one gap. A gap crossed by another box is not a run, and runs are shown
/// only while an alignment on the same axis is active.
fn equal_spacing_run_lines(
    others: &[GuideBox],
    pairs: &[(GuideBox, GuideBox)],
    axis: GuideAxis,
    lane: f64,
) -> Vec<GuideLine> {
    let mut runs: Vec<(f64, Vec<GuideLine>)> = Vec::new();

    for (left, right) in pairs {
        let start = along_end(left.bounds, axis);
        let end = along_start(right.bounds, axis);
        let gap = end - start;
        if gap <= 0.0 {
            continue;
        }

        let crossed = others.iter().any(|other| {
            perpendicular_gap(left.bounds, other.bounds, axis) <= lane
                && along_start(other.bounds, axis) > start
                && along_end(other.bounds, axis) < end
        });
        if crossed {
            continue;
        }

        let line = spacing_line(
            start,
            end,
            lane_center(left.bounds, right.bounds, axis),
            axis,
        );
        match runs
            .iter_mut()
            .find(|(distance, _)| same_gap(*distance, gap))
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

/// Resolve smart-guide candidates for a proposed move.
///
/// Resolves both axes on every pointer update. Active guides use a slightly
/// wider release band so they remain stable while the pointer crosses the
/// candidate, but they are reacquired continuously after release. Objects
/// beyond the search radius are dropped before anything is scanned, so with
/// nothing nearby the resolution is empty and clears any previous guide.
pub fn resolve_move(
    proposed: Rect,
    others: &[(ElementId, Rect)],
    enabled: bool,
    zoom: f64,
    previous: &SmartGuideState,
) -> GuideResolution {
    if !enabled {
        return GuideResolution::default();
    }

    let tolerances = Tolerances::for_zoom(zoom);
    let others: Vec<GuideBox> = others
        .iter()
        .map(|(id, bounds)| GuideBox {
            id: *id,
            bounds: *bounds,
        })
        .filter(|other| rect_distance(proposed, other.bounds) <= tolerances.radius)
        .collect();
    if others.is_empty() {
        return GuideResolution::default();
    }

    let vertical_pairs = inline_pairs(&others, GuideAxis::Vertical, tolerances.lane);
    let horizontal_pairs = inline_pairs(&others, GuideAxis::Horizontal, tolerances.lane);
    let vertical = axis_candidate(
        proposed,
        &others,
        &vertical_pairs,
        GuideAxis::Vertical,
        previous,
        &tolerances,
    );
    let horizontal = axis_candidate(
        proposed,
        &others,
        &horizontal_pairs,
        GuideAxis::Horizontal,
        previous,
        &tolerances,
    );

    let snap_dx = vertical.map_or(0.0, |candidate| candidate.snap_delta);
    let snap_dy = horizontal.map_or(0.0, |candidate| candidate.snap_delta);
    let snapped = Rect::new(
        proposed.x + snap_dx,
        proposed.y + snap_dy,
        proposed.w,
        proposed.h,
    );

    let mut active = Vec::new();
    let mut guides = Vec::new();
    for (axis, candidate, pairs) in [
        (GuideAxis::Vertical, vertical, &vertical_pairs),
        (GuideAxis::Horizontal, horizontal, &horizontal_pairs),
    ] {
        let Some(candidate) = candidate else {
            continue;
        };
        active.push(candidate.key);
        match candidate.key.kind {
            GuideKind::Alignment => {
                guides.extend(alignment_guides(snapped, &others, candidate.key));
                guides.extend(equal_spacing_run_lines(
                    &others,
                    pairs,
                    axis,
                    tolerances.lane,
                ));
            }
            GuideKind::EqualSpacing => {
                guides.extend(equal_spacing_guides(
                    snapped,
                    &others,
                    pairs,
                    &candidate,
                    &tolerances,
                ));
            }
        }
    }

    GuideResolution {
        snap_dx,
        snap_dy,
        state: SmartGuideState { guides, active },
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

    fn solve(proposed: Rect, others: &[(ElementId, Rect)]) -> GuideResolution {
        resolve_move(proposed, others, true, 1.0, &SmartGuideState::default())
    }

    #[test]
    fn alignment_snaps_start_center_and_end_on_both_axes() {
        for axis in [GuideAxis::Vertical, GuideAxis::Horizontal] {
            for (moving_start, moving_size, target_start, target_size, feature, delta) in [
                (100.0, 40.0, 106.0, 80.0, GuideFeature::Start, 6.0),
                (100.0, 40.0, 86.0, 80.0, GuideFeature::Center, 6.0),
                (100.0, 40.0, 66.0, 80.0, GuideFeature::End, 6.0),
            ] {
                let moving = match axis {
                    GuideAxis::Vertical => Rect::new(moving_start, 300.0, moving_size, 20.0),
                    GuideAxis::Horizontal => Rect::new(300.0, moving_start, 20.0, moving_size),
                };
                let target = match axis {
                    GuideAxis::Vertical => Rect::new(target_start, 0.0, target_size, 20.0),
                    GuideAxis::Horizontal => Rect::new(0.0, target_start, 20.0, target_size),
                };
                let resolution = solve(moving, &[(1, target)]);
                let snap = match axis {
                    GuideAxis::Vertical => resolution.snap_dx,
                    GuideAxis::Horizontal => resolution.snap_dy,
                };
                assert_eq!(snap, delta);
                assert_eq!(resolution.state.active.len(), 1);
                assert_eq!(resolution.state.active[0].moving_feature, feature);
                assert_eq!(resolution.state.active[0].target_feature, feature);
            }
        }
    }

    #[test]
    fn alignment_guide_spans_every_matching_element() {
        let others = [
            rect(1, 140.0, 0.0, 20.0, 50.0),
            rect(2, 140.0, 100.0, 20.0, 50.0),
        ];
        let resolution = solve(Rect::new(134.0, 200.0, 20.0, 50.0), &others);
        assert_eq!(resolution.snap_dx, 6.0);
        assert_eq!(resolution.state.guides.len(), 1);
        assert_eq!(
            resolution.state.guides[0],
            GuideLine {
                start: Point::new(140.0, 0.0),
                end: Point::new(140.0, 250.0),
                kind: GuideKind::Alignment,
            }
        );
    }

    #[test]
    fn coincident_boxes_emit_two_full_length_guides() {
        let resolution = solve(
            Rect::new(10.0, 10.0, 20.0, 20.0),
            &[rect(1, 10.0, 10.0, 20.0, 20.0)],
        );
        assert_eq!((resolution.snap_dx, resolution.snap_dy), (0.0, 0.0));
        assert_eq!(resolution.state.guides.len(), 2);
        assert_eq!(
            resolution.state.guides[0],
            GuideLine {
                start: Point::new(10.0, 10.0),
                end: Point::new(10.0, 30.0),
                kind: GuideKind::Alignment,
            }
        );
        assert_eq!(
            resolution.state.guides[1],
            GuideLine {
                start: Point::new(10.0, 10.0),
                end: Point::new(30.0, 10.0),
                kind: GuideKind::Alignment,
            }
        );
    }

    #[test]
    fn active_candidate_holds_through_the_release_band() {
        let others = [rect(1, 200.0, 100.0, 20.0, 20.0)];
        let first = solve(Rect::new(194.0, 300.0, 20.0, 20.0), &others);
        assert_eq!(first.snap_dx, 6.0);
        assert_eq!(first.state.active.len(), 1);

        // 8px away: outside the acquisition band, still inside the release band.
        let held = resolve_move(
            Rect::new(192.0, 300.0, 20.0, 20.0),
            &others,
            true,
            1.0,
            &first.state,
        );
        assert_eq!(held.snap_dx, 8.0);
        assert_eq!(held.state.active, first.state.active);

        let fresh = solve(Rect::new(192.0, 300.0, 20.0, 20.0), &others);
        assert_eq!(fresh.snap_dx, 0.0);
        assert!(fresh.state.active.is_empty());
        assert!(fresh.state.guides.is_empty());
    }

    #[test]
    fn active_candidate_releases_beyond_the_band() {
        let others = [rect(1, 200.0, 100.0, 20.0, 20.0)];
        let first = solve(Rect::new(194.0, 300.0, 20.0, 20.0), &others);
        let released = resolve_move(
            Rect::new(170.0, 300.0, 20.0, 20.0),
            &others,
            true,
            1.0,
            &first.state,
        );
        assert_eq!(released.snap_dx, 0.0);
        assert!(released.state.active.is_empty());
        assert!(released.state.guides.is_empty());
    }

    #[test]
    fn tolerances_are_screen_space_and_scale_with_zoom() {
        let others = [rect(1, 200.0, 100.0, 20.0, 20.0)];

        // At zoom 2 the acquisition band is 3 world px, so 3.5px misses...
        let miss = resolve_move(
            Rect::new(196.5, 240.0, 20.0, 20.0),
            &others,
            true,
            2.0,
            &SmartGuideState::default(),
        );
        assert_eq!(miss.snap_dx, 0.0);

        // ...3px acquires, and the same 3.5px offset acquires at zoom 1.
        let edge = resolve_move(
            Rect::new(197.0, 240.0, 20.0, 20.0),
            &others,
            true,
            2.0,
            &SmartGuideState::default(),
        );
        assert_eq!(edge.snap_dx, 3.0);
        let close = solve(Rect::new(196.5, 240.0, 20.0, 20.0), &others);
        assert_eq!(close.snap_dx, 3.5);

        // A held guide survives to the wider release band (5px at zoom 2).
        let held = resolve_move(
            Rect::new(195.5, 240.0, 20.0, 20.0),
            &others,
            true,
            2.0,
            &edge.state,
        );
        assert_eq!(held.snap_dx, 4.5);
    }

    #[test]
    fn equal_spacing_snaps_into_the_middle_of_a_gap() {
        let others = [
            rect(1, 0.0, 0.0, 20.0, 100.0),
            rect(2, 100.0, 0.0, 20.0, 100.0),
        ];
        let first = solve(Rect::new(44.0, 20.0, 20.0, 20.0), &others);
        assert_eq!((first.snap_dx, first.snap_dy), (6.0, 0.0));
        assert_eq!(first.state.active.len(), 1);
        assert_eq!(first.state.active[0].kind, GuideKind::EqualSpacing);
        assert_eq!(first.state.guides.len(), 2);
        assert_eq!(
            first.state.guides[0],
            GuideLine {
                start: Point::new(20.0, 30.0),
                end: Point::new(50.0, 30.0),
                kind: GuideKind::EqualSpacing,
            }
        );

        // The snap tracks continuously while held.
        let again = resolve_move(
            Rect::new(45.0, 20.0, 20.0, 20.0),
            &others,
            true,
            1.0,
            &first.state,
        );
        assert_eq!(again.snap_dx, 5.0);

        let released = resolve_move(
            Rect::new(61.0, 20.0, 20.0, 20.0),
            &others,
            true,
            1.0,
            &again.state,
        );
        assert_eq!(released.snap_dx, 0.0);
        assert!(released.state.active.is_empty());
        assert!(released.state.guides.is_empty());
    }

    #[test]
    fn equal_spacing_draws_matching_gaps_at_their_own_lanes() {
        let others = [
            rect(1, 0.0, 0.0, 20.0, 100.0),
            rect(2, 100.0, 0.0, 20.0, 100.0),
            rect(3, 200.0, 0.0, 20.0, 100.0),
            rect(4, 300.0, 0.0, 20.0, 100.0),
        ];
        let resolution = solve(Rect::new(44.0, 20.0, 20.0, 20.0), &others);
        assert_eq!(resolution.snap_dx, 6.0);
        assert_eq!(resolution.state.guides.len(), 6);
        assert!(resolution
            .state
            .guides
            .iter()
            .all(|line| line.kind == GuideKind::EqualSpacing));
        // The snapped pair draws at the moving box, the others in their lane.
        assert_eq!(resolution.state.guides[0].start, Point::new(20.0, 30.0));
        assert_eq!(resolution.state.guides[1].end, Point::new(100.0, 30.0));
        assert_eq!(
            resolution
                .state
                .guides
                .iter()
                .filter(|line| line.start.y == 50.0)
                .count(),
            4
        );
        assert_eq!(resolution.state.guides[2].start, Point::new(120.0, 50.0));
        assert_eq!(resolution.state.guides[5].end, Point::new(300.0, 50.0));
    }

    #[test]
    fn equal_spacing_requires_a_shared_lane() {
        let others = [
            rect(1, 0.0, 0.0, 20.0, 20.0),
            rect(2, 100.0, 0.0, 20.0, 20.0),
        ];
        let resolution = solve(Rect::new(44.0, 300.0, 20.0, 20.0), &others);
        assert_eq!((resolution.snap_dx, resolution.snap_dy), (0.0, 0.0));
        assert!(resolution.state.active.is_empty());
        assert!(resolution.state.guides.is_empty());
    }

    #[test]
    fn closest_alignment_wins_between_targets() {
        let others = [
            rect(1, 106.0, 0.0, 20.0, 20.0),
            rect(2, 202.0, 0.0, 20.0, 20.0),
        ];
        let resolution = solve(Rect::new(100.0, 300.0, 100.0, 20.0), &others);
        assert_eq!(resolution.snap_dx, 2.0);
        assert_eq!(resolution.state.active[0].target_a, 2);
        assert_eq!(resolution.state.active[0].moving_feature, GuideFeature::End);
        assert_eq!(
            resolution.state.guides[0],
            GuideLine {
                start: Point::new(202.0, 0.0),
                end: Point::new(202.0, 320.0),
                kind: GuideKind::Alignment,
            }
        );
    }

    #[test]
    fn exact_ties_between_targets_resolve_to_the_smallest_id() {
        let others = [
            rect(1, 94.0, 0.0, 20.0, 20.0),
            rect(2, 106.0, 0.0, 20.0, 20.0),
        ];
        let moving = Rect::new(100.0, 300.0, 20.0, 20.0);
        let resolution = solve(moving, &others);
        assert_eq!(resolution.snap_dx, -6.0);
        assert_eq!(resolution.state.active[0].target_a, 1);

        let mut reversed = others;
        reversed.reverse();
        assert_eq!(solve(moving, &reversed), resolution);
    }

    #[test]
    fn exact_ties_between_spacing_and_alignment_prefer_spacing() {
        let others = [
            rect(1, 0.0, 0.0, 20.0, 100.0),
            rect(2, 100.0, 0.0, 20.0, 100.0),
            rect(3, 60.0, 200.0, 20.0, 20.0),
        ];
        let resolution = solve(Rect::new(55.0, 20.0, 20.0, 20.0), &others);
        assert_eq!(resolution.snap_dx, -5.0);
        assert_eq!(resolution.state.active[0].kind, GuideKind::EqualSpacing);
        assert_eq!(resolution.state.guides.len(), 2);
    }

    #[test]
    fn runs_are_highlighted_while_an_alignment_is_active() {
        let others = [
            rect(1, 0.0, 0.0, 20.0, 20.0),
            rect(2, 100.0, 0.0, 20.0, 20.0),
            rect(3, 200.0, 0.0, 20.0, 20.0),
            rect(4, 300.0, 0.0, 20.0, 20.0),
        ];
        let resolution = solve(Rect::new(106.0, 250.0, 20.0, 20.0), &others);
        assert_eq!(resolution.snap_dx, -6.0);
        assert_eq!(resolution.state.guides.len(), 4);
        assert_eq!(
            resolution.state.guides[0],
            GuideLine {
                start: Point::new(100.0, 0.0),
                end: Point::new(100.0, 270.0),
                kind: GuideKind::Alignment,
            }
        );
        assert_eq!(
            resolution.state.guides[1],
            GuideLine {
                start: Point::new(20.0, 10.0),
                end: Point::new(100.0, 10.0),
                kind: GuideKind::EqualSpacing,
            }
        );
        assert_eq!(
            resolution.state.guides[3],
            GuideLine {
                start: Point::new(220.0, 10.0),
                end: Point::new(300.0, 10.0),
                kind: GuideKind::EqualSpacing,
            }
        );
    }

    #[test]
    fn runs_need_two_matching_gaps() {
        let boxes = [
            guide(1, Rect::new(0.0, 0.0, 20.0, 20.0)),
            guide(2, Rect::new(100.0, 0.0, 20.0, 20.0)),
            guide(3, Rect::new(600.0, 0.0, 20.0, 20.0)),
        ];
        let lane = GUIDE_LANE_PX;
        let pairs = inline_pairs(&boxes, GuideAxis::Vertical, lane);
        assert!(equal_spacing_run_lines(&boxes, &pairs, GuideAxis::Vertical, lane).is_empty());
    }

    #[test]
    fn runs_skip_a_gap_crossed_by_an_object() {
        let lane = GUIDE_LANE_PX;
        let run_count = |boxes: &[GuideBox]| {
            let pairs = inline_pairs(boxes, GuideAxis::Vertical, lane);
            equal_spacing_run_lines(boxes, &pairs, GuideAxis::Vertical, lane).len()
        };

        let clear = [
            guide(1, Rect::new(0.0, 0.0, 20.0, 20.0)),
            guide(2, Rect::new(100.0, 0.0, 20.0, 20.0)),
            guide(3, Rect::new(200.0, 0.0, 20.0, 20.0)),
        ];
        assert_eq!(run_count(&clear), 2);

        let blocked = [
            guide(1, Rect::new(0.0, 0.0, 20.0, 20.0)),
            guide(2, Rect::new(100.0, 0.0, 20.0, 20.0)),
            guide(3, Rect::new(200.0, 0.0, 20.0, 20.0)),
            guide(4, Rect::new(140.0, 0.0, 20.0, 20.0)),
        ];
        assert_eq!(run_count(&blocked), 0);
    }

    #[test]
    fn objects_beyond_the_search_radius_are_ignored() {
        let moving = Rect::new(0.0, 0.0, 20.0, 20.0);
        let near = [rect(1, 5.0, 280.0, 20.0, 20.0)];
        let far = [rect(1, 5.0, 340.0, 20.0, 20.0)];

        let snapped = solve(moving, &near);
        assert_eq!((snapped.snap_dx, snapped.snap_dy), (5.0, 0.0));
        assert_eq!(snapped.state.active.len(), 1);

        let ignored = solve(moving, &far);
        assert_eq!((ignored.snap_dx, ignored.snap_dy), (0.0, 0.0));
        assert!(ignored.state.active.is_empty());

        // Zoomed out the radius covers more world space...
        let zoomed_out = resolve_move(moving, &far, true, 0.5, &SmartGuideState::default());
        assert_eq!((zoomed_out.snap_dx, zoomed_out.snap_dy), (5.0, 0.0));
        // ...and zoomed in, less.
        let zoomed_in = resolve_move(moving, &near, true, 2.0, &SmartGuideState::default());
        assert_eq!((zoomed_in.snap_dx, zoomed_in.snap_dy), (0.0, 0.0));
    }

    #[test]
    fn disabled_resolution_is_empty() {
        let others = [rect(1, 10.0, 0.0, 20.0, 20.0)];
        let previous = SmartGuideState {
            guides: Vec::new(),
            active: vec![GuideKey {
                axis: GuideAxis::Vertical,
                kind: GuideKind::Alignment,
                moving_feature: GuideFeature::Start,
                target_feature: GuideFeature::Start,
                target_a: 1,
                target_b: 0,
            }],
        };
        let resolution = resolve_move(
            Rect::new(10.0, 0.0, 20.0, 20.0),
            &others,
            false,
            1.0,
            &previous,
        );
        assert_eq!((resolution.snap_dx, resolution.snap_dy), (0.0, 0.0));
        assert_eq!(resolution.state, SmartGuideState::default());
    }

    #[test]
    fn randomized_positions_stay_finite() {
        struct Rng(u64);
        impl Rng {
            fn next(&mut self) -> f64 {
                self.0 ^= self.0 << 13;
                self.0 ^= self.0 >> 7;
                self.0 ^= self.0 << 17;
                (self.0 % 100_000) as f64 / 100.0 - 500.0
            }
        }

        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let mut state = SmartGuideState::default();
        for round in 0..400 {
            let count = (rng.next().abs() as usize % 6) + 1;
            let others: Vec<(ElementId, Rect)> = (0..count)
                .map(|i| {
                    (
                        i as ElementId + 1,
                        Rect::new(
                            rng.next(),
                            rng.next(),
                            rng.next().abs() + 0.1,
                            rng.next().abs() + 0.1,
                        ),
                    )
                })
                .collect();
            let zoom = [0.25, 1.0, 4.0][round % 3];
            let proposed = Rect::new(
                rng.next(),
                rng.next(),
                rng.next().abs() + 0.1,
                rng.next().abs() + 0.1,
            );
            let resolution = resolve_move(proposed, &others, round % 7 != 0, zoom, &state);
            assert!(resolution.snap_dx.is_finite() && resolution.snap_dy.is_finite());
            assert!(resolution.state.active.len() <= 2);
            for line in &resolution.state.guides {
                assert!(
                    line.start.x.is_finite()
                        && line.start.y.is_finite()
                        && line.end.x.is_finite()
                        && line.end.y.is_finite(),
                    "non-finite guide {line:?}"
                );
                assert_ne!(line.start, line.end, "degenerate guide {line:?}");
            }
            state = resolution.state;
        }
    }
}
