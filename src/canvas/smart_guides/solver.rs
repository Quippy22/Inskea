//! The resolution entry point: scan neighbours, snap both axes, assemble the
//! guides for every active candidate.

use super::candidates::axis_candidate;
use super::geometry::{chain_connections, inline_pairs, rect_distance};
use super::guides::{
    alignment_guides, chain_spacing_guides, equal_spacing_guides, equal_spacing_run_lines,
};
use super::{GuideAxis, GuideBox, GuideKind, GuideResolution, SmartGuideState, Tolerances};
use crate::model::{ElementId, Rect};

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
    let all: Vec<GuideBox> = others
        .iter()
        .map(|(id, bounds)| GuideBox {
            id: *id,
            bounds: *bounds,
        })
        .collect();
    let others: Vec<GuideBox> = all
        .iter()
        .copied()
        .filter(|other| rect_distance(proposed, other.bounds) <= tolerances.radius)
        .collect();
    if others.is_empty() {
        return GuideResolution::default();
    }

    let vertical_pairs = inline_pairs(&others, GuideAxis::Vertical, tolerances.lane);
    let horizontal_pairs = inline_pairs(&others, GuideAxis::Horizontal, tolerances.lane);
    let vertical_chain = chain_connections(&others, &all, GuideAxis::Vertical, tolerances.lane);
    let horizontal_chain = chain_connections(&others, &all, GuideAxis::Horizontal, tolerances.lane);
    let vertical = axis_candidate(
        proposed,
        &others,
        &all,
        &vertical_pairs,
        &vertical_chain,
        GuideAxis::Vertical,
        previous,
        &tolerances,
    );
    let horizontal = axis_candidate(
        proposed,
        &others,
        &all,
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
        match candidate.key.kind {
            GuideKind::Alignment => {
                guides.extend(alignment_guides(snapped, &others, candidate.key));
                guides.extend(equal_spacing_run_lines(&others, pairs, axis, &tolerances));
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
            GuideKind::ChainSpacing => {
                guides.extend(chain_spacing_guides(
                    snapped,
                    &all,
                    chain_pairs,
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
