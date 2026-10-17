//! Rectangle coordinate arithmetic shared by the candidate scan and the
//! guide emitters: start/end along the guide axis, start/end across it, and
//! the pair helpers built on top.

use super::{GuideAxis, GuideBox, GuideFeature, GuideLine};
use crate::model::{ElementId, Point, Rect};
use std::collections::HashSet;

pub(super) fn along_start(bounds: Rect, axis: GuideAxis) -> f64 {
    match axis {
        GuideAxis::Vertical => bounds.x,
        GuideAxis::Horizontal => bounds.y,
    }
}

pub(super) fn along_end(bounds: Rect, axis: GuideAxis) -> f64 {
    match axis {
        GuideAxis::Vertical => bounds.right(),
        GuideAxis::Horizontal => bounds.bottom(),
    }
}

pub(super) fn cross_start(bounds: Rect, axis: GuideAxis) -> f64 {
    match axis {
        GuideAxis::Vertical => bounds.y,
        GuideAxis::Horizontal => bounds.x,
    }
}

pub(super) fn cross_end(bounds: Rect, axis: GuideAxis) -> f64 {
    match axis {
        GuideAxis::Vertical => bounds.bottom(),
        GuideAxis::Horizontal => bounds.right(),
    }
}

pub(super) fn feature_value(bounds: Rect, axis: GuideAxis, feature: GuideFeature) -> f64 {
    let start = along_start(bounds, axis);
    let end = along_end(bounds, axis);
    match feature {
        GuideFeature::Start => start,
        GuideFeature::Center => (start + end) / 2.0,
        GuideFeature::End => end,
    }
}

pub(super) fn features(bounds: Rect, axis: GuideAxis) -> [f64; 3] {
    let start = along_start(bounds, axis);
    let end = along_end(bounds, axis);
    [start, (start + end) / 2.0, end]
}

pub(super) fn axis_point(axis: GuideAxis, along: f64, cross: f64) -> Point {
    match axis {
        GuideAxis::Vertical => Point::new(along, cross),
        GuideAxis::Horizontal => Point::new(cross, along),
    }
}

/// Shortest distance between two rects, zero when they touch or overlap.
pub(super) fn rect_distance(a: Rect, b: Rect) -> f64 {
    let gap_x = (a.x - b.right()).max(b.x - a.right()).max(0.0);
    let gap_y = (a.y - b.bottom()).max(b.y - a.bottom()).max(0.0);
    (gap_x * gap_x + gap_y * gap_y).sqrt()
}

/// Overlap test that counts touching edges as overlap.
pub(super) fn intersects(a: Rect, b: Rect) -> bool {
    a.x <= b.right() && b.x <= a.right() && a.y <= b.bottom() && b.y <= a.bottom()
}

/// Clamp a point into `area`.
pub(super) fn clamp_to(p: Point, area: Rect) -> Point {
    Point::new(
        p.x.clamp(area.x, area.right()),
        p.y.clamp(area.y, area.bottom()),
    )
}

/// Trim a guide to the visible area so a long row does not draw a line far
/// past the edge of the screen. Guides left degenerate are dropped.
pub(super) fn clamp_guide(line: GuideLine, area: Rect) -> Option<GuideLine> {
    let start = clamp_to(line.start, area);
    let end = clamp_to(line.end, area);
    (start != end).then_some(GuideLine {
        start,
        end,
        stops: line.stops,
        kind: line.kind,
    })
}

/// Gap between two boxes across the perpendicular of `axis`; zero when they
/// overlap or touch on that axis.
pub(super) fn perpendicular_gap(a: Rect, b: Rect, axis: GuideAxis) -> f64 {
    match axis {
        GuideAxis::Vertical if a.bottom() < b.y => b.y - a.bottom(),
        GuideAxis::Vertical if b.bottom() < a.y => a.y - b.bottom(),
        GuideAxis::Horizontal if a.right() < b.x => b.x - a.right(),
        GuideAxis::Horizontal if b.right() < a.x => a.x - b.right(),
        _ => 0.0,
    }
}

pub(super) fn ordered_pair(
    left: GuideBox,
    right: GuideBox,
    axis: GuideAxis,
) -> (GuideBox, GuideBox) {
    if along_start(left.bounds, axis) <= along_start(right.bounds, axis) {
        (left, right)
    } else {
        (right, left)
    }
}

/// Every pair of boxes, ordered along the axis, whose perpendicular extents
/// are close enough to bound a spacing gap together.
pub(super) fn inline_pairs(
    others: &[GuideBox],
    axis: GuideAxis,
    lane: f64,
) -> Vec<(GuideBox, GuideBox)> {
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

/// Inline pairs for chain attachments: the near box is always a scanned box
/// while the far box may sit beyond the search radius, since it only supplies
/// the distance to match.
pub(super) fn chain_connections(
    scanned: &[GuideBox],
    all: &[GuideBox],
    axis: GuideAxis,
    lane: f64,
) -> Vec<(GuideBox, GuideBox)> {
    let scanned_ids: HashSet<ElementId> = scanned.iter().map(|b| b.id).collect();
    let mut pairs = Vec::new();
    for near in scanned {
        for far in all {
            if far.id == near.id || perpendicular_gap(near.bounds, far.bounds, axis) > lane {
                continue;
            }
            // Emit each pair once; skip the mirrored visit when both are scanned.
            if scanned_ids.contains(&far.id) && far.id < near.id {
                continue;
            }
            pairs.push(ordered_pair(*near, *far, axis));
        }
    }
    pairs
}

/// Perpendicular coordinate a spacing marker is drawn at: just outside the
/// boxes' shared edges, clear of the features alignment guides ride on.
pub(super) fn marker_lane(a: Rect, b: Rect, axis: GuideAxis, offset: f64) -> f64 {
    cross_start(a, axis).min(cross_start(b, axis)) - offset
}

pub(super) fn same_gap(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6 * a.abs().max(b.abs()).max(1.0)
}
