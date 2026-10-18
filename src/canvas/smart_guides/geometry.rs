//! Rectangle coordinate arithmetic shared by the candidate scan and the
//! guide emitters: start/end along the guide axis, start/end across it, and
//! the pair helpers built on top.

use super::{GuideAxis, GuideBox, GuideFeature, GuideLine, EPSILON};
use crate::model::interaction::{FrameShape, GuideFrame};
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

/// Split a point into its coordinate along and across the guide axis.
pub(super) fn along_cross(point: Point, axis: GuideAxis) -> (f64, f64) {
    match axis {
        GuideAxis::Vertical => (point.x, point.y),
        GuideAxis::Horizontal => (point.y, point.x),
    }
}

/// The stretch of the line at `coordinate` that runs inside the frame: the
/// silhouette of the shape it carries. A box is met at its own edges, so a
/// guide ends on the shape instead of on the corner of its envelope; an
/// ellipse is met at the chord of its outline, which collapses to the tangent
/// point at an extreme coordinate. The coordinate always lies within the
/// shape's projection, so the line is never clear of it.
pub(super) fn frame_span(frame: GuideFrame, axis: GuideAxis, coordinate: f64) -> (f64, f64) {
    match frame.shape {
        FrameShape::Rect => box_span(frame.corners, axis, coordinate),
        FrameShape::Ellipse => ellipse_span(frame.corners, axis, coordinate),
    }
}

/// The box's silhouette on the line: the edges it crosses, plus the edges that
/// lie along it, which is what a tangent coordinate meets the box at.
fn box_span(corners: [Point; 4], axis: GuideAxis, coordinate: f64) -> (f64, f64) {
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

/// The chord of the ellipse inscribed in the box with these corners, on the
/// line at `coordinate`. Corners keep their box order: `0 -> 1` is the
/// half-width axis and `0 -> 3` the half-height one, both swung by the frame's
/// rotation.
fn ellipse_span(corners: [Point; 4], axis: GuideAxis, coordinate: f64) -> (f64, f64) {
    let center = Point::new(
        (corners[0].x + corners[2].x) / 2.0,
        (corners[0].y + corners[2].y) / 2.0,
    );
    let half_w = (corners[1].x - corners[0].x).hypot(corners[1].y - corners[0].y) / 2.0;
    let half_h = (corners[3].x - corners[0].x).hypot(corners[3].y - corners[0].y) / 2.0;
    if half_w <= EPSILON || half_h <= EPSILON {
        return box_span(corners, axis, coordinate);
    }
    let angle = (corners[1].y - corners[0].y).atan2(corners[1].x - corners[0].x);
    let (sin, cos) = angle.sin_cos();
    // Into the ellipse's own frame, where the outline is
    // (a / half_w)^2 + (b / half_h)^2 = 1.
    let to_local = |dx: f64, dy: f64| (dx * cos + dy * sin, -dx * sin + dy * cos);

    // Parameterise the line by its cross coordinate measured from the centre,
    // so the quadratic below only ever works with the ellipse's own scale: an
    // origin far from the shape must not affect how a graze is resolved.
    let center_cross = match axis {
        GuideAxis::Vertical => center.y,
        GuideAxis::Horizontal => center.x,
    };
    let (base_a, base_b) = match axis {
        GuideAxis::Vertical => to_local(coordinate - center.x, 0.0),
        GuideAxis::Horizontal => to_local(0.0, coordinate - center.y),
    };
    let (dir_a, dir_b) = match axis {
        GuideAxis::Vertical => (sin, cos),
        GuideAxis::Horizontal => (cos, -sin),
    };
    let scale_a = 1.0 / (half_w * half_w);
    let scale_b = 1.0 / (half_h * half_h);
    let qa = dir_a * dir_a * scale_a + dir_b * dir_b * scale_b;
    let qb = 2.0 * (base_a * dir_a * scale_a + base_b * dir_b * scale_b);
    let qc = base_a * base_a * scale_a + base_b * base_b * scale_b - 1.0;
    let disc = qb * qb - 4.0 * qa * qc;
    // A graze — a discriminant on or just under zero — resolves to its
    // vertex, the one point where the line touches the outline. At a turned
    // graze float noise can leave a vanishingly narrow chord instead, well
    // under a pixel wide, which no one can tell from the point.
    let (low, high) = if disc <= 0.0 {
        let tangent = -qb / (2.0 * qa);
        (tangent, tangent)
    } else {
        let root = disc.sqrt();
        ((-qb - root) / (2.0 * qa), (-qb + root) / (2.0 * qa))
    };
    (center_cross + low, center_cross + high)
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

/// Whether the moving box and both boxes of a pair share a lane across the
/// axis.
pub(super) fn share_lane(
    moving: Rect,
    left: Rect,
    right: Rect,
    axis: GuideAxis,
    lane: f64,
) -> bool {
    perpendicular_gap(moving, left, axis) <= lane && perpendicular_gap(moving, right, axis) <= lane
}

/// The free gap between two boxes ordered along the axis: from the end of the
/// left box to the start of the right one.
pub(super) fn gap_span(left: Rect, right: Rect, axis: GuideAxis) -> (f64, f64) {
    (along_end(left, axis), along_start(right, axis))
}

/// Copy of the boxes sorted along `axis`, so the crossed-gap scans can stop as
/// soon as a box starts past the gap they are testing.
pub(super) fn sorted_along(boxes: &[GuideBox], axis: GuideAxis) -> Vec<GuideBox> {
    let mut sorted = boxes.to_vec();
    sorted.sort_by(|a, b| along_start(a.bounds, axis).total_cmp(&along_start(b.bounds, axis)));
    sorted
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
