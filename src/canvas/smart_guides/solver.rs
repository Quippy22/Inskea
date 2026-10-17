//! The resolution entry point: scan neighbours, snap both axes, assemble the
//! guides for every active candidate.

use super::candidates::axis_candidate;
use super::geometry::{
    axis_point, chain_connections, clamp_guide, inline_pairs, intersects, rect_distance,
};
use super::guides::{
    alignment_guides, chain_spacing_guides, equal_spacing_guides, equal_spacing_run_lines,
};
use super::{
    GuideAxis, GuideBox, GuideKind, GuideLabel, GuideLine, GuideResolution, SmartGuideState,
    Tolerances, GUIDE_MAX_PAIR_TARGETS,
};
use crate::model::{ElementId, Point, Rect};

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

/// Resolve smart-guide candidates for a proposed move.
///
/// Call this only while guides are enabled; a disabled caller keeps the
/// previous state cleared instead. `others` holds the bounds of every element
/// outside the drag and `visible` is the world rectangle on screen: only
/// targets the user can see take part, and guide lines are trimmed to it.
/// Alignment reads every one of those targets; the spacing scans pair up only
/// the ones close to the moving box, since they are the quadratic part.
/// Resolves both axes on every pointer update. Active guides use a slightly
/// wider release band so they remain stable while the pointer crosses the
/// candidate, but they are reacquired continuously after release. With nothing
/// on screen the resolution is empty and clears any previous guide.
pub fn resolve_move(
    proposed: Rect,
    others: &[(ElementId, Rect)],
    visible: Rect,
    zoom: f64,
    previous: &SmartGuideState,
) -> GuideResolution {
    let tolerances = Tolerances::for_zoom(zoom);
    let all: Vec<GuideBox> = others
        .iter()
        .filter(|(_, bounds)| intersects(*bounds, visible))
        .map(|(id, bounds)| GuideBox {
            id: *id,
            bounds: *bounds,
        })
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
    let vertical_chain = chain_connections(&near, &near, GuideAxis::Vertical, tolerances.lane);
    let horizontal_chain = chain_connections(&near, &near, GuideAxis::Horizontal, tolerances.lane);
    let vertical = axis_candidate(
        proposed,
        &all,
        &near,
        &vertical_pairs,
        &vertical_chain,
        GuideAxis::Vertical,
        previous,
        &tolerances,
    );
    let horizontal = axis_candidate(
        proposed,
        &all,
        &near,
        &horizontal_pairs,
        &horizontal_chain,
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
    let mut labels = Vec::new();
    for (axis, candidate, pairs, chain_pairs) in [
        (
            GuideAxis::Vertical,
            vertical,
            &vertical_pairs,
            &vertical_chain,
        ),
        (
            GuideAxis::Horizontal,
            horizontal,
            &horizontal_pairs,
            &horizontal_chain,
        ),
    ] {
        let Some(candidate) = candidate else {
            continue;
        };
        active.push(candidate.key);
        let lines = match candidate.key.kind {
            GuideKind::Alignment => {
                let mut lines = alignment_guides(snapped, &all, candidate.key);
                lines.extend(equal_spacing_run_lines(&near, pairs, axis, &tolerances));
                lines
            }
            GuideKind::EqualSpacing => {
                equal_spacing_guides(snapped, &near, pairs, &candidate, &tolerances)
            }
            GuideKind::ChainSpacing => {
                chain_spacing_guides(snapped, &near, chain_pairs, &candidate, &tolerances)
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
