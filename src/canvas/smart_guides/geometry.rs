//! Rectangle coordinate arithmetic shared by the candidate scan and the
//! guide emitters: start/end along the guide axis, start/end across it, and
//! the pair helpers built on top.

use super::{GuideAxis, GuideBox, GuideFeature, GuideLine, EPSILON};
use crate::model::{Point, Rect};

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

/// Split a point into its coordinate along and across the guide axis.
pub(super) fn along_cross(point: Point, axis: GuideAxis) -> (f64, f64) {
    match axis {
        GuideAxis::Vertical => (point.x, point.y),
        GuideAxis::Horizontal => (point.y, point.x),
    }
}

/// The stretch of the line at `coordinate` that runs inside the frame with
/// those corners: the frame's silhouette there. An unrotated box is its own
/// frame, so this is the box's cross extent. A rotated frame is met at one of
/// its corners or edges, so the guide ends on the shape instead of on the
/// corner of its envelope. The coordinate always lies within the frame's
/// projection, so at least one edge matches and a tangent coordinate meets the
/// frame at its corner.
pub(super) fn frame_span(corners: [Point; 4], axis: GuideAxis, coordinate: f64) -> (f64, f64) {
    let epsilon = EPSILON * (1.0 + coordinate.abs());
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for index in 0..4 {
        let (a_along, a_cross) = along_cross(corners[index], axis);
        let (b_along, b_cross) = along_cross(corners[(index + 1) % 4], axis);
        let (edge_start, edge_end) = (a_along.min(b_along), a_along.max(b_along));
        if coordinate < edge_start - epsilon || coordinate > edge_end + epsilon {
            continue;
        }
        if edge_end - edge_start <= epsilon {
            // The edge lies along the line: the whole edge is on it.
            low = low.min(a_cross.min(b_cross));
            high = high.max(a_cross.max(b_cross));
        } else {
            let t = ((coordinate - a_along) / (b_along - a_along)).clamp(0.0, 1.0);
            let cross = a_cross + (b_cross - a_cross) * t;
            low = low.min(cross);
            high = high.max(cross);
        }
    }
    (low, high)
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

/// Whether another box sits inside the gap from `start` to `end` in the lane
/// of `left`. `others` must be sorted by along start on `axis`: the scan then
/// jumps straight to the first box past `start` and gives up as soon as one
/// begins past `end`.
pub(super) fn gap_crossed(
    others: &[GuideBox],
    left: Rect,
    axis: GuideAxis,
    start: f64,
    end: f64,
    lane: f64,
) -> bool {
    let from = others.partition_point(|other| along_start(other.bounds, axis) <= start);
    for other in &others[from..] {
        if along_start(other.bounds, axis) >= end {
            return false;
        }
        if along_end(other.bounds, axis) < end
            && perpendicular_gap(left, other.bounds, axis) <= lane
        {
            return true;
        }
    }
    false
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

/// Perpendicular coordinate a spacing marker is drawn at: just outside the
/// boxes' shared edges, clear of the features alignment guides ride on.
pub(super) fn marker_lane(a: Rect, b: Rect, axis: GuideAxis, offset: f64) -> f64 {
    cross_start(a, axis).min(cross_start(b, axis)) - offset
}

pub(super) fn same_gap(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6 * a.abs().max(b.abs()).max(1.0)
}
