//! The resolution entry point: scan neighbours, snap both axes, assemble the
//! guides for every active candidate.

use super::candidates::AxisScan;
use super::geometry::{
    along_cross, along_end, along_start, axis_point, clamp_guide, cross_end, cross_start,
    intersects, rect_distance,
};
use super::guides::{
    alignment_guides, chain_spacing_guides, equal_spacing_guides, equal_spacing_run_lines,
};
use super::{
    GuideAxis, GuideBox, GuideKind, GuideLabel, GuideLine, GuideResolution, SmartGuideState,
    Tolerances, GUIDE_MAX_PAIR_TARGETS,
};
use crate::model::interaction::GuideFrame;
use crate::model::Rect;

/// Readout for a spacing band: the free space between its two ends, measured
/// along the guide axis and reported from the middle of the band.
fn band_label(band: GuideLine, axis: GuideAxis) -> GuideLabel {
    let (from, cross) = along_cross(band.start, axis);
    let (to, _) = along_cross(band.end, axis);
    GuideLabel {
        at: axis_point(axis, (from + to) / 2.0, cross),
        distance: (to - from).abs(),
        axis,
    }
}

/// Keep a label on screen: slide it along its band into view, and drop it when
/// the band itself sits off screen.
fn fit_label(mut label: GuideLabel, visible: Rect) -> Option<GuideLabel> {
    let axis = label.axis;
    let (along, cross) = along_cross(label.at, axis);
    if cross < cross_start(visible, axis) || cross > cross_end(visible, axis) {
        return None;
    }
    let along = along.clamp(along_start(visible, axis), along_end(visible, axis));
    label.at = axis_point(axis, along, cross);
    Some(label)
}

/// Resolve smart-guide candidates for a proposed move.
///
/// Call this only while guides are enabled; a disabled caller keeps the
/// previous state cleared instead. `proposed` is the moving selection's
/// envelope and `proposed_frame` the frame guides are drawn against: the
/// corners of the box the selection outline hugs, plus the shape inside it, so
/// a rotated element and an ellipse are both met on their own outline.
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
    proposed_frame: GuideFrame,
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

    let vertical = AxisScan::new(GuideAxis::Vertical, &near, tolerances.lane);
    let horizontal = AxisScan::new(GuideAxis::Horizontal, &near, tolerances.lane);
    let vertical_choice = vertical.candidate(proposed, &all, previous, &tolerances);
    let horizontal_choice = horizontal.candidate(proposed, &all, previous, &tolerances);

    let snap_dx = vertical_choice.map_or(0.0, |candidate| candidate.snap_delta);
    let snap_dy = horizontal_choice.map_or(0.0, |candidate| candidate.snap_delta);
    let snapped = Rect::new(
        proposed.x + snap_dx,
        proposed.y + snap_dy,
        proposed.w,
        proposed.h,
    );
    let snapped_frame = proposed_frame.translated(snap_dx, snap_dy);

    let mut active = Vec::new();
    let mut guides = Vec::new();
    let mut labels = Vec::new();
    for (scan, choice) in [
        (&vertical, vertical_choice),
        (&horizontal, horizontal_choice),
    ] {
        let Some(candidate) = choice else {
            continue;
        };
        active.push(candidate.key);
        let lines = match candidate.key.kind {
            GuideKind::Alignment => {
                let mut lines = alignment_guides(snapped, snapped_frame, &all, candidate.key);
                lines.extend(equal_spacing_run_lines(snapped, scan, &tolerances));
                lines
            }
            GuideKind::EqualSpacing => equal_spacing_guides(snapped, scan, &candidate, &tolerances),
            GuideKind::ChainSpacing => chain_spacing_guides(snapped, scan, &candidate, &tolerances),
        };
        if candidate.key.kind != GuideKind::Alignment {
            // The first line of a spacing guide is the band the moving box
            // closed against; it is the one worth measuring.
            if let Some(band) = lines.first() {
                labels.push(band_label(*band, scan.axis));
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
