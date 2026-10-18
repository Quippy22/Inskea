//! Guide emission: turn a winning candidate into the lines to draw, in world
//! coordinates.

use super::candidates::AxisScan;
use super::geometry::{
    along_end, along_start, axis_point, cross_start, feature_value, features, frame_span,
    gap_crossed, gap_span, marker_lane, ordered_pair, same_gap, share_lane,
};
use super::{
    GuideAxis, GuideBox, GuideCandidate, GuideKey, GuideKind, GuideLine, Tolerances, EPSILON,
};
use crate::model::interaction::GuideFrame;
use crate::model::Rect;

/// The free stretches between the boxes that share the snapped coordinate.
/// Only those gaps are drawn, so the guide connects the exterior edges and
/// ends on the frames; a gap another box sits across is cut at that box rather
/// than drawn through it.
pub(super) fn alignment_guides(
    moving: Rect,
    moving_frame: GuideFrame,
    others: &[GuideBox],
    key: GuideKey,
) -> Vec<GuideLine> {
    let axis = key.axis;
    let coordinate = feature_value(moving, axis, key.moving_feature);
    let epsilon = EPSILON * (1.0 + coordinate.abs());

    let mut spans = vec![frame_span(moving_frame, axis, coordinate)];
    let mut blockers: Vec<(f64, f64)> = Vec::new();
    for other in others {
        let connected = features(other.bounds, axis)
            .iter()
            .any(|value| (value - coordinate).abs() <= epsilon);
        // Any box the line would run through breaks it, whether or not its own
        // features are among the ones being matched.
        let covering = along_start(other.bounds, axis) < coordinate - epsilon
            && along_end(other.bounds, axis) > coordinate + epsilon;
        if connected || covering {
            let span = frame_span(other.frame, axis, coordinate);
            if connected {
                spans.push(span);
            }
            if covering {
                blockers.push(span);
            }
        }
    }
    spans.sort_by(|a, b| a.0.total_cmp(&b.0));
    blockers.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut guides = Vec::new();
    let mut reach = spans[0].1;
    for &(start, end) in &spans[1..] {
        if start - reach > epsilon {
            free_stretches(&mut guides, reach, start, &blockers, axis, coordinate);
        }
        reach = reach.max(end);
    }
    guides
}

/// Draw the part of the free stretch from `from` to `to` that no blocker
/// covers, cutting the line where a box sits in the way.
fn free_stretches(
    guides: &mut Vec<GuideLine>,
    from: f64,
    to: f64,
    blockers: &[(f64, f64)],
    axis: GuideAxis,
    coordinate: f64,
) {
    let mut edge = from;
    for &(block_start, block_end) in blockers {
        if block_end <= edge {
            continue;
        }
        if block_start >= to {
            break;
        }
        if block_start > edge {
            guides.push(alignment_segment(axis, coordinate, edge, block_start));
        }
        edge = block_end;
        if edge >= to {
            return;
        }
    }
    if to > edge {
        guides.push(alignment_segment(axis, coordinate, edge, to));
    }
}

fn alignment_segment(axis: GuideAxis, coordinate: f64, from: f64, to: f64) -> GuideLine {
    GuideLine {
        start: axis_point(axis, coordinate, from),
        end: axis_point(axis, coordinate, to),
        stops: [coordinate, coordinate],
        kind: GuideKind::Alignment,
    }
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

/// The boxes a candidate's key names, in key order.
fn selected_pair(boxes: &[GuideBox], key: GuideKey) -> Option<(GuideBox, GuideBox)> {
    let left = boxes.iter().find(|b| b.id == key.target_a)?;
    let right = boxes.iter().find(|b| b.id == key.target_b)?;
    Some((*left, *right))
}

/// The band drawn across one inline pair's gap, in the pair's own lane.
fn pair_band(
    left: GuideBox,
    right: GuideBox,
    axis: GuideAxis,
    marker: f64,
    kind: GuideKind,
) -> GuideLine {
    let (start, end) = gap_span(left.bounds, right.bounds, axis);
    spacing_line(
        start,
        end,
        marker_lane(left.bounds, right.bounds, axis, marker),
        [
            cross_start(left.bounds, axis),
            cross_start(right.bounds, axis),
        ],
        axis,
        kind,
    )
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
    scan: &AxisScan,
    candidate: &GuideCandidate,
    tolerances: &Tolerances,
) -> Vec<GuideLine> {
    let axis = scan.axis;
    let Some((left, right)) = selected_pair(&scan.ordered, candidate.key) else {
        return Vec::new();
    };

    let (selected_start, selected_end) = gap_span(left.bounds, right.bounds, axis);
    let selected_gap = selected_end - selected_start;
    let moving_start = along_start(moving, axis);
    let moving_size = along_end(moving, axis) - moving_start;
    let selected_cross = cross_start(moving, axis)
        .min(cross_start(left.bounds, axis))
        .min(cross_start(right.bounds, axis))
        - tolerances.marker;
    let mut guides = Vec::new();

    for (pair_left, pair_right) in &scan.pairs {
        let (start, end) = gap_span(pair_left.bounds, pair_right.bounds, axis);
        if !same_gap(end - start, selected_gap) || moving_size >= end - start {
            continue;
        }
        if !share_lane(
            moving,
            pair_left.bounds,
            pair_right.bounds,
            axis,
            tolerances.lane,
        ) {
            continue;
        }
        // The pair the drag snaps into keeps the moving box's lane; the others
        // draw in their own. A phantom box fits their free gaps; its ticks
        // keep the standard length.
        let (cross, stops) = if pair_left.id == candidate.key.target_a {
            (
                selected_cross,
                [
                    cross_start(pair_left.bounds, axis),
                    cross_start(moving, axis),
                    cross_start(pair_right.bounds, axis),
                ],
            )
        } else {
            let top = cross_start(pair_left.bounds, axis).min(cross_start(pair_right.bounds, axis));
            (
                marker_lane(pair_left.bounds, pair_right.bounds, axis, tolerances.marker),
                [
                    cross_start(pair_left.bounds, axis),
                    top,
                    cross_start(pair_right.bounds, axis),
                ],
            )
        };
        guides.extend(spacing_lines(start, end, moving_size, cross, stops, axis));
    }

    guides
}

/// Arrows for the gap the moving box just closed against a chain, plus every
/// inline gap of the same length, so the chain lights up whole.
pub(super) fn chain_spacing_guides(
    moving: Rect,
    scan: &AxisScan,
    candidate: &GuideCandidate,
    tolerances: &Tolerances,
) -> Vec<GuideLine> {
    let axis = scan.axis;
    let Some((near, far)) = selected_pair(&scan.ordered, candidate.key) else {
        return Vec::new();
    };
    let (left, right) = ordered_pair(near, far, axis);
    let (chain_start, chain_end) = gap_span(left.bounds, right.bounds, axis);
    let chain_gap = chain_end - chain_start;

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

    for (pair_left, pair_right) in &scan.pairs {
        let (start, end) = gap_span(pair_left.bounds, pair_right.bounds, axis);
        if !same_gap(end - start, chain_gap) {
            continue;
        }
        if !share_lane(
            moving,
            pair_left.bounds,
            pair_right.bounds,
            axis,
            tolerances.lane,
        ) {
            continue;
        }
        guides.push(pair_band(
            *pair_left,
            *pair_right,
            axis,
            tolerances.marker,
            GuideKind::ChainSpacing,
        ));
    }

    guides
}

/// Highlight runs of evenly spaced boxes: at least two inline pairs sharing
/// one gap. A gap crossed by another box is not a run, and runs are shown
/// only while an alignment on the same axis is active and the moving box
/// shares their lane: the rhythm of a row the drag is nowhere near is noise,
/// not a measurement of the drag. The scan's boxes are sorted along the axis,
/// which the crossed-gap test relies on.
pub(super) fn equal_spacing_run_lines(
    moving: Rect,
    scan: &AxisScan,
    tolerances: &Tolerances,
) -> Vec<GuideLine> {
    let axis = scan.axis;
    let mut runs: Vec<(f64, Vec<GuideLine>)> = Vec::new();

    for (left, right) in &scan.pairs {
        if !share_lane(moving, left.bounds, right.bounds, axis, tolerances.lane) {
            continue;
        }
        let (start, end) = gap_span(left.bounds, right.bounds, axis);
        let gap = end - start;
        if gap <= 0.0 {
            continue;
        }

        let crossed = gap_crossed(
            &scan.ordered,
            left.bounds,
            axis,
            start,
            end,
            tolerances.lane,
        );
        if crossed {
            continue;
        }
        let line = pair_band(
            *left,
            *right,
            axis,
            tolerances.marker,
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
