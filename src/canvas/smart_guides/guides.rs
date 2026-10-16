//! Guide emission: turn a winning candidate into the lines to draw, in world
//! coordinates.

use super::geometry::{
    along_end, along_start, axis_point, cross_end, cross_start, feature_value, features,
    marker_lane, ordered_pair, perpendicular_gap, same_gap,
};
use super::{
    GuideAxis, GuideBox, GuideCandidate, GuideKey, GuideKind, GuideLine, Tolerances, EPSILON,
};
use crate::model::Rect;

/// The free stretches between the boxes that share the snapped coordinate.
/// Only those gaps are drawn, so the guide connects the exterior edges and
/// never runs along or through a box.
pub(super) fn alignment_guides(moving: Rect, others: &[GuideBox], key: GuideKey) -> Vec<GuideLine> {
    let axis = key.axis;
    let coordinate = feature_value(moving, axis, key.moving_feature);
    let epsilon = EPSILON * (1.0 + coordinate.abs());

    let mut spans = vec![(cross_start(moving, axis), cross_end(moving, axis))];
    for other in others {
        if features(other.bounds, axis)
            .iter()
            .any(|value| (value - coordinate).abs() <= epsilon)
        {
            spans.push((
                cross_start(other.bounds, axis),
                cross_end(other.bounds, axis),
            ));
        }
    }
    spans.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut guides = Vec::new();
    let mut reach = spans[0].1;
    for &(start, end) in &spans[1..] {
        if start - reach > epsilon {
            guides.push(GuideLine {
                start: axis_point(axis, coordinate, reach),
                end: axis_point(axis, coordinate, start),
                stops: [coordinate, coordinate],
                kind: GuideKind::Alignment,
            });
        }
        reach = reach.max(end);
    }
    guides
}

fn spacing_line(
    start: f64,
    end: f64,
    cross: f64,
    stops: [f64; 2],
    axis: GuideAxis,
    kind: GuideKind,
) -> GuideLine {
    GuideLine {
        start: axis_point(axis, start, cross),
        end: axis_point(axis, end, cross),
        stops,
        kind,
    }
}

/// The two arrow segments a gap is drawn as: the free space left of the box
/// and the free space right of it, equal once the box sits centred. `stops`
/// are the cross starts of the piece left of the gap, the moving box, and the
/// piece right of it.
fn spacing_lines(
    start: f64,
    end: f64,
    moving_size: f64,
    cross: f64,
    stops: [f64; 3],
    axis: GuideAxis,
) -> Vec<GuideLine> {
    let desired_start = (start + end - moving_size) / 2.0;
    vec![
        spacing_line(
            start,
            desired_start,
            cross,
            [stops[0], stops[1]],
            axis,
            GuideKind::EqualSpacing,
        ),
        spacing_line(
            desired_start + moving_size,
            end,
            cross,
            [stops[1], stops[2]],
            axis,
            GuideKind::EqualSpacing,
        ),
    ]
}

/// Arrows for the snapped gap plus every other inline pair with the same gap,
/// drawn in that pair's own lane. Pairs the moving box is not inline with are
/// not shown: their arrows would float outside the row.
pub(super) fn equal_spacing_guides(
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
    let selected_cross = cross_start(moving, axis)
        .min(cross_start(left.bounds, axis))
        .min(cross_start(right.bounds, axis))
        - tolerances.marker;
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
            selected_cross
        } else {
            marker_lane(pair_left.bounds, pair_right.bounds, axis, tolerances.marker)
        };
        let stops = if pair_left.id == candidate.key.target_a {
            [
                cross_start(pair_left.bounds, axis),
                cross_start(moving, axis),
                cross_start(pair_right.bounds, axis),
            ]
        } else {
            // A phantom box fits the gap; its ticks keep the standard length.
            let top = cross_start(pair_left.bounds, axis).min(cross_start(pair_right.bounds, axis));
            [
                cross_start(pair_left.bounds, axis),
                top,
                cross_start(pair_right.bounds, axis),
            ]
        };
        guides.extend(spacing_lines(start, end, moving_size, cross, stops, axis));
    }

    guides
}

/// Arrows for the gap the moving box just closed against a chain, plus every
/// inline gap of the same length, so the chain lights up whole.
pub(super) fn chain_spacing_guides(
    moving: Rect,
    all: &[GuideBox],
    chain_pairs: &[(GuideBox, GuideBox)],
    candidate: &GuideCandidate,
    tolerances: &Tolerances,
) -> Vec<GuideLine> {
    let axis = candidate.key.axis;
    let selected = all
        .iter()
        .find(|b| b.id == candidate.key.target_a)
        .zip(all.iter().find(|b| b.id == candidate.key.target_b));
    let Some((near, far)) = selected else {
        return Vec::new();
    };
    let (left, right) = ordered_pair(*near, *far, axis);
    let chain_gap = along_start(right.bounds, axis) - along_end(left.bounds, axis);

    let moving_start = along_start(moving, axis);
    let (start, end, stops) = if moving_start >= along_end(near.bounds, axis) {
        (
            along_end(near.bounds, axis),
            moving_start,
            [cross_start(near.bounds, axis), cross_start(moving, axis)],
        )
    } else {
        (
            along_end(moving, axis),
            along_start(near.bounds, axis),
            [cross_start(moving, axis), cross_start(near.bounds, axis)],
        )
    };
    let mut guides = vec![spacing_line(
        start,
        end,
        marker_lane(moving, near.bounds, axis, tolerances.marker),
        stops,
        axis,
        GuideKind::ChainSpacing,
    )];

    for (pair_left, pair_right) in chain_pairs {
        let start = along_end(pair_left.bounds, axis);
        let end = along_start(pair_right.bounds, axis);
        if !same_gap(end - start, chain_gap) {
            continue;
        }
        if perpendicular_gap(moving, pair_left.bounds, axis) > tolerances.lane
            || perpendicular_gap(moving, pair_right.bounds, axis) > tolerances.lane
        {
            continue;
        }
        guides.push(spacing_line(
            start,
            end,
            marker_lane(pair_left.bounds, pair_right.bounds, axis, tolerances.marker),
            [
                cross_start(pair_left.bounds, axis),
                cross_start(pair_right.bounds, axis),
            ],
            axis,
            GuideKind::ChainSpacing,
        ));
    }

    guides
}

/// Highlight runs of evenly spaced boxes: at least two inline pairs sharing
/// one gap. A gap crossed by another box is not a run, and runs are shown
/// only while an alignment on the same axis is active.
pub(super) fn equal_spacing_run_lines(
    others: &[GuideBox],
    pairs: &[(GuideBox, GuideBox)],
    axis: GuideAxis,
    tolerances: &Tolerances,
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
            perpendicular_gap(left.bounds, other.bounds, axis) <= tolerances.lane
                && along_start(other.bounds, axis) > start
                && along_end(other.bounds, axis) < end
        });
        if crossed {
            continue;
        }

        let line = spacing_line(
            start,
            end,
            marker_lane(left.bounds, right.bounds, axis, tolerances.marker),
            [
                cross_start(left.bounds, axis),
                cross_start(right.bounds, axis),
            ],
            axis,
            GuideKind::EqualSpacing,
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
