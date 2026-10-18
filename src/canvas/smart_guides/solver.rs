//! The resolution entry point: scan neighbours, snap both axes, assemble the
//! guides for every active candidate.

use super::candidates::axis_candidate;
use super::geometry::{
    along_start, axis_point, clamp_guide, inline_pairs, intersects, rect_distance,
};
use super::guides::{
    alignment_guides, chain_spacing_guides, equal_spacing_guides, equal_spacing_run_lines,
};
use super::{
    GuideAxis, GuideBox, GuideKind, GuideLabel, GuideLine, GuideResolution, SmartGuideState,
    Tolerances, GUIDE_MAX_PAIR_TARGETS,
};
use crate::model::{Point, Rect};

/// Readout for a spacing band: the free space between its two ends, measured
/// along the guide axis and reported from the middle of the band.
fn band_label(band: GuideLine, axis: GuideAxis) -> GuideLabel {
    let (from, to, cross) = match axis {
        GuideAxis::Vertical => (band.start.x, band.end.x, band.start.y),
        GuideAxis::Horizontal => (band.start.y, band.end.y, band.start.x),
    };
    GuideLabel {
        at: axis_point(axis, (from + to) / 2.0, cross),
        distance: (to - from).abs(),
        axis,
    }
}

/// Keep a label on screen: slide it along its band into view, and drop it when
/// the band itself sits off screen.
fn fit_label(mut label: GuideLabel, visible: Rect) -> Option<GuideLabel> {
    let horizontal_band = label.axis == GuideAxis::Vertical;
    let (along_range, cross_range) = if horizontal_band {
        ((visible.x, visible.right()), (visible.y, visible.bottom()))
    } else {
        ((visible.y, visible.bottom()), (visible.x, visible.right()))
    };
    let (along, cross) = if horizontal_band {
        (label.at.x, label.at.y)
    } else {
        (label.at.y, label.at.x)
    };
    if cross < cross_range.0 || cross > cross_range.1 {
        return None;
    }
    let along = along.clamp(along_range.0, along_range.1);
    label.at = if horizontal_band {
        Point::new(along, cross)
    } else {
        Point::new(cross, along)
    };
    Some(label)
}

/// Copy of the local targets sorted along `axis`, so the crossed-gap scans
/// can stop as soon as a box starts past the gap they are testing.
pub(super) fn sorted_along(near: &[GuideBox], axis: GuideAxis) -> Vec<GuideBox> {
    let mut sorted = near.to_vec();
    sorted.sort_by(|a, b| along_start(a.bounds, axis).total_cmp(&along_start(b.bounds, axis)));
    sorted
}

/// Resolve smart-guide candidates for a proposed move.
///
/// Call this only while guides are enabled; a disabled caller keeps the
/// previous state cleared instead. `proposed` is the moving selection's
/// envelope with `proposed_corners` the corners of its frame — the box the
/// selection outline hugs, which is rotated for a single rotated element.
/// `others` holds the frames of every element outside the drag and `visible`
/// is the world rectangle on screen: only targets the user can see take part,
/// and guide lines are trimmed to it. Alignment reads every one of those
/// targets; the spacing scans pair up only the ones close to the moving box,
/// since they are the quadratic part. Resolves both axes on every pointer
/// update. Active guides use a slightly wider release band so they remain
/// stable while the pointer crosses the candidate, but they are reacquired
/// continuously after release. With nothing on screen the resolution is empty
/// and clears any previous guide.
pub fn resolve_move(
    proposed: Rect,
    proposed_corners: [Point; 4],
    others: &[GuideBox],
    visible: Rect,
    zoom: f64,
    previous: &SmartGuideState,
) -> GuideResolution {
    let tolerances = Tolerances::for_zoom(zoom);
    let all: Vec<GuideBox> = others
        .iter()
        .filter(|other| intersects(other.bounds, visible))
        .copied()
        .collect();
    if all.is_empty() {
        return GuideResolution::default();
    }

    // Alignment reads every visible target; the spacing and chain scans are
    // quadratic in the boxes sharing a lane, so they only look at the ones
    // close enough to matter.
    let mut near: Vec<GuideBox> = all
        .iter()
        .copied()
        .filter(|other| rect_distance(proposed, other.bounds) <= tolerances.reach)
        .collect();
    if near.len() > GUIDE_MAX_PAIR_TARGETS {
        near.select_nth_unstable_by(GUIDE_MAX_PAIR_TARGETS, |a, b| {
            rect_distance(proposed, a.bounds).total_cmp(&rect_distance(proposed, b.bounds))
        });
        near.truncate(GUIDE_MAX_PAIR_TARGETS);
    }

    let vertical_pairs = inline_pairs(&near, GuideAxis::Vertical, tolerances.lane);
    let horizontal_pairs = inline_pairs(&near, GuideAxis::Horizontal, tolerances.lane);
    let vertical_crossed = sorted_along(&near, GuideAxis::Vertical);
    let horizontal_crossed = sorted_along(&near, GuideAxis::Horizontal);
    let vertical = axis_candidate(
        proposed,
        &all,
        &vertical_crossed,
        &vertical_pairs,
        GuideAxis::Vertical,
        previous,
        &tolerances,
    );
    let horizontal = axis_candidate(
        proposed,
        &all,
        &horizontal_crossed,
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
    let snapped_corners =
        proposed_corners.map(|corner| Point::new(corner.x + snap_dx, corner.y + snap_dy));

    let mut active = Vec::new();
    let mut guides = Vec::new();
    let mut labels = Vec::new();
    for (axis, candidate, pairs, crossed) in [
        (
            GuideAxis::Vertical,
            vertical,
            &vertical_pairs,
            &vertical_crossed,
        ),
        (
            GuideAxis::Horizontal,
            horizontal,
            &horizontal_pairs,
            &horizontal_crossed,
        ),
    ] {
        let Some(candidate) = candidate else {
            continue;
        };
        active.push(candidate.key);
        let lines = match candidate.key.kind {
            GuideKind::Alignment => {
                let mut lines = alignment_guides(snapped, snapped_corners, &all, candidate.key);
                lines.extend(equal_spacing_run_lines(
                    snapped,
                    crossed,
                    pairs,
                    axis,
                    &tolerances,
                ));
                lines
            }
            GuideKind::EqualSpacing => {
                equal_spacing_guides(snapped, crossed, pairs, &candidate, &tolerances)
            }
            GuideKind::ChainSpacing => {
                chain_spacing_guides(snapped, crossed, pairs, &candidate, &tolerances)
            }
        };
        if candidate.key.kind != GuideKind::Alignment {
            // The first line of a spacing guide is the band the moving box
            // closed against; it is the one worth measuring.
            if let Some(band) = lines.first() {
                labels.push(band_label(*band, axis));
            }
        }
        guides.extend(lines);
    }

    let guides = guides
        .into_iter()
        .filter_map(|line| clamp_guide(line, visible))
        .collect();
    let labels = labels
        .into_iter()
        .filter_map(|label| fit_label(label, visible))
        .collect();

    GuideResolution {
        snap_dx,
        snap_dy,
        state: SmartGuideState {
            guides,
            labels,
            active,
        },
    }
}
