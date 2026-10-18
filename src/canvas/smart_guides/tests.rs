use super::geometry::*;
use super::guides::*;
use super::*;
use crate::model::elements::rect::Rectangle;
use crate::model::elements::ElementData;
use crate::model::interaction::{guide_bounds, guide_corners};
use crate::model::{Element, ElementId, Point, Rect};

fn rect(id: ElementId, x: f64, y: f64, w: f64, h: f64) -> GuideBox {
    let bounds = Rect::new(x, y, w, h);
    GuideBox {
        id,
        bounds,
        corners: bounds.corners(),
    }
}

/// A target whose frame is its own envelope (an element that does not rotate).
fn guide(id: ElementId, bounds: Rect) -> GuideBox {
    rect(id, bounds.x, bounds.y, bounds.w, bounds.h)
}

/// A target taken from a real element, so its frame comes from the same
/// helpers the canvas uses.
fn element(id: ElementId, x: f64, y: f64, w: f64, h: f64, rotation: f64) -> GuideBox {
    let mut data = ElementData::new(id);
    data.world_point = Point::new(x, y);
    data.width = w;
    data.height = h;
    data.rotation = rotation;
    let element = Element::Rectangle(Rectangle { data });
    GuideBox {
        id,
        bounds: guide_bounds(&element),
        corners: guide_corners(&element),
    }
}

/// World rectangle the tests treat as on screen: wide enough that small
/// fixtures are never filtered or trimmed.
fn view() -> Rect {
    Rect::new(-10_000.0, -10_000.0, 20_000.0, 20_000.0)
}

/// Solve as if the moving box were unrotated, so its frame is its envelope.
fn resolve(
    proposed: Rect,
    others: &[GuideBox],
    visible: Rect,
    zoom: f64,
    previous: &SmartGuideState,
) -> GuideResolution {
    resolve_move(
        proposed,
        proposed.corners(),
        others,
        visible,
        zoom,
        previous,
    )
}

fn solve(proposed: Rect, others: &[GuideBox]) -> GuideResolution {
    resolve(proposed, others, view(), 1.0, &SmartGuideState::default())
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
            let resolution = solve(moving, &[guide(1, target)]);
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
fn alignment_guides_connect_every_matching_element() {
    let others = [
        rect(1, 140.0, 0.0, 20.0, 50.0),
        rect(2, 140.0, 100.0, 20.0, 50.0),
    ];
    let resolution = solve(Rect::new(134.0, 212.0, 20.0, 50.0), &others);
    assert_eq!(resolution.snap_dx, 6.0);
    assert_eq!(resolution.state.guides.len(), 2);
    assert_eq!(
        resolution.state.guides[0],
        GuideLine {
            start: Point::new(140.0, 50.0),
            end: Point::new(140.0, 100.0),
            stops: [140.0, 140.0],
            kind: GuideKind::Alignment,
        }
    );
    assert_eq!(
        resolution.state.guides[1],
        GuideLine {
            start: Point::new(140.0, 150.0),
            end: Point::new(140.0, 212.0),
            stops: [140.0, 140.0],
            kind: GuideKind::Alignment,
        }
    );
}

#[test]
fn alignment_guide_connects_the_gap_between_side_by_side_boxes() {
    let others = [rect(1, 0.0, 0.0, 100.0, 80.0)];
    let resolution = solve(Rect::new(160.0, 0.0, 100.0, 80.0), &others);
    assert_eq!((resolution.snap_dx, resolution.snap_dy), (0.0, 0.0));
    assert_eq!(
        resolution.state.guides,
        vec![GuideLine {
            start: Point::new(100.0, 0.0),
            end: Point::new(160.0, 0.0),
            stops: [0.0, 0.0],
            kind: GuideKind::Alignment,
        }]
    );
}

#[test]
fn coincident_boxes_emit_no_guides() {
    let resolution = solve(
        Rect::new(10.0, 10.0, 20.0, 20.0),
        &[rect(1, 10.0, 10.0, 20.0, 20.0)],
    );
    assert_eq!((resolution.snap_dx, resolution.snap_dy), (0.0, 0.0));
    assert_eq!(resolution.state.active.len(), 2);
    assert!(resolution.state.guides.is_empty());
}

#[test]
fn active_candidate_holds_through_the_release_band() {
    let others = [rect(1, 200.0, 100.0, 20.0, 20.0)];
    let first = solve(Rect::new(194.0, 300.0, 20.0, 20.0), &others);
    assert_eq!(first.snap_dx, 6.0);
    assert_eq!(first.state.active.len(), 1);

    // 8px away: outside the acquisition band, still inside the release band.
    let held = resolve(
        Rect::new(192.0, 300.0, 20.0, 20.0),
        &others,
        view(),
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
    let released = resolve(
        Rect::new(170.0, 300.0, 20.0, 20.0),
        &others,
        view(),
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
    let miss = resolve(
        Rect::new(196.5, 240.0, 20.0, 20.0),
        &others,
        view(),
        2.0,
        &SmartGuideState::default(),
    );
    assert_eq!(miss.snap_dx, 0.0);

    // ...3px acquires, and the same 3.5px offset acquires at zoom 1.
    let edge = resolve(
        Rect::new(197.0, 240.0, 20.0, 20.0),
        &others,
        view(),
        2.0,
        &SmartGuideState::default(),
    );
    assert_eq!(edge.snap_dx, 3.0);
    let close = solve(Rect::new(196.5, 240.0, 20.0, 20.0), &others);
    assert_eq!(close.snap_dx, 3.5);

    // A held guide survives to the wider release band (5px at zoom 2).
    let held = resolve(
        Rect::new(195.5, 240.0, 20.0, 20.0),
        &others,
        view(),
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
            start: Point::new(20.0, -9.0),
            end: Point::new(50.0, -9.0),
            stops: [0.0, 20.0],
            kind: GuideKind::EqualSpacing,
        }
    );

    // The snap tracks continuously while held.
    let again = resolve(
        Rect::new(45.0, 20.0, 20.0, 20.0),
        &others,
        view(),
        1.0,
        &first.state,
    );
    assert_eq!(again.snap_dx, 5.0);

    let released = resolve(
        Rect::new(61.0, 20.0, 20.0, 20.0),
        &others,
        view(),
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
        rect(3, 200.0, 10.0, 20.0, 100.0),
        rect(4, 300.0, 10.0, 20.0, 100.0),
    ];
    let resolution = solve(Rect::new(44.0, 20.0, 20.0, 20.0), &others);
    assert_eq!(resolution.snap_dx, 6.0);
    assert_eq!(resolution.state.guides.len(), 6);
    assert!(resolution
        .state
        .guides
        .iter()
        .all(|line| line.kind == GuideKind::EqualSpacing));
    // Markers ride just outside their own pair's boxes.
    assert_eq!(resolution.state.guides[0].start, Point::new(20.0, -9.0));
    assert_eq!(resolution.state.guides[1].end, Point::new(100.0, -9.0));
    assert_eq!(
        resolution
            .state
            .guides
            .iter()
            .filter(|line| line.start.y == -9.0)
            .count(),
        4
    );
    assert_eq!(
        resolution
            .state
            .guides
            .iter()
            .filter(|line| line.start.y == 1.0)
            .count(),
        2
    );
    assert_eq!(resolution.state.guides[2].start, Point::new(120.0, -9.0));
    assert_eq!(resolution.state.guides[5].end, Point::new(300.0, 1.0));
}

#[test]
fn chain_attachment_snaps_the_gap_to_the_chain() {
    let others = [
        rect(1, 0.0, 0.0, 20.0, 20.0),
        rect(2, 100.0, 0.0, 20.0, 20.0),
    ];
    let resolution = solve(Rect::new(202.0, 8.0, 20.0, 20.0), &others);
    assert_eq!((resolution.snap_dx, resolution.snap_dy), (-2.0, 0.0));
    assert_eq!(resolution.state.active.len(), 1);
    assert_eq!(resolution.state.active[0].kind, GuideKind::ChainSpacing);
    assert_eq!(resolution.state.active[0].target_a, 2);
    assert_eq!(
        resolution.state.guides,
        vec![
            GuideLine {
                start: Point::new(120.0, -9.0),
                end: Point::new(200.0, -9.0),
                stops: [0.0, 8.0],
                kind: GuideKind::ChainSpacing,
            },
            GuideLine {
                start: Point::new(20.0, -9.0),
                end: Point::new(100.0, -9.0),
                stops: [0.0, 0.0],
                kind: GuideKind::ChainSpacing,
            },
        ]
    );
}

#[test]
fn chain_attachment_snaps_from_the_left() {
    let others = [
        rect(1, 100.0, 0.0, 20.0, 20.0),
        rect(2, 200.0, 0.0, 20.0, 20.0),
    ];
    let resolution = solve(Rect::new(2.0, 8.0, 20.0, 20.0), &others);
    assert_eq!((resolution.snap_dx, resolution.snap_dy), (-2.0, 0.0));
    assert_eq!(resolution.state.active[0].kind, GuideKind::ChainSpacing);
    assert_eq!(
        (
            resolution.state.active[0].target_a,
            resolution.state.active[0].target_b
        ),
        (1, 2)
    );
    assert_eq!(
        resolution.state.guides,
        vec![
            GuideLine {
                start: Point::new(20.0, -9.0),
                end: Point::new(100.0, -9.0),
                stops: [8.0, 0.0],
                kind: GuideKind::ChainSpacing,
            },
            GuideLine {
                start: Point::new(120.0, -9.0),
                end: Point::new(200.0, -9.0),
                stops: [0.0, 0.0],
                kind: GuideKind::ChainSpacing,
            },
        ]
    );
}

#[test]
fn chain_attachment_requires_a_connection() {
    let resolution = solve(
        Rect::new(202.0, 8.0, 20.0, 20.0),
        &[rect(1, 100.0, 0.0, 20.0, 20.0)],
    );
    assert_eq!((resolution.snap_dx, resolution.snap_dy), (0.0, 0.0));
    assert!(resolution.state.active.is_empty());
    assert!(resolution.state.guides.is_empty());
}

#[test]
fn chain_attachment_skips_a_gap_crossed_by_a_box() {
    let others = [
        rect(1, 0.0, 0.0, 20.0, 20.0),
        rect(2, 50.0, 0.0, 20.0, 20.0),
        rect(3, 100.0, 0.0, 20.0, 20.0),
    ];
    let resolution = solve(Rect::new(202.0, 8.0, 20.0, 20.0), &others);
    assert_eq!((resolution.snap_dx, resolution.snap_dy), (0.0, 0.0));
    assert!(resolution.state.active.is_empty());
    assert!(resolution.state.guides.is_empty());
}

#[test]
fn chain_attachment_handles_negative_coordinates() {
    let others = [
        rect(1, -140.0, -116.5, 100.0, 80.0),
        rect(2, 60.0, -116.5, 100.0, 80.0),
    ];
    let resolution = solve(Rect::new(262.0, -116.5, 100.0, 80.0), &others);
    assert_eq!(resolution.snap_dx, -2.0);
}

#[test]
fn chain_attachment_extends_a_run() {
    let others = [
        rect(1, 0.0, 0.0, 20.0, 20.0),
        rect(2, 100.0, 0.0, 20.0, 20.0),
        rect(3, 200.0, 0.0, 20.0, 20.0),
    ];
    let resolution = solve(Rect::new(302.0, 8.0, 20.0, 20.0), &others);
    assert_eq!((resolution.snap_dx, resolution.snap_dy), (-2.0, 0.0));
    assert_eq!(resolution.state.active[0].kind, GuideKind::ChainSpacing);
    assert_eq!(
        (
            resolution.state.active[0].target_a,
            resolution.state.active[0].target_b
        ),
        (3, 2)
    );
    assert_eq!(resolution.state.guides.len(), 3);
    assert!(resolution
        .state
        .guides
        .iter()
        .all(|line| line.kind == GuideKind::ChainSpacing));
    assert_eq!(resolution.state.guides[0].start, Point::new(220.0, -9.0));
    assert_eq!(resolution.state.guides[0].end, Point::new(300.0, -9.0));
    assert_eq!(resolution.state.guides[1].start, Point::new(20.0, -9.0));
    assert_eq!(resolution.state.guides[2].end, Point::new(200.0, -9.0));
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
            start: Point::new(202.0, 20.0),
            end: Point::new(202.0, 300.0),
            stops: [202.0, 202.0],
            kind: GuideKind::Alignment,
        }
    );
}

#[test]
fn alignment_guide_breaks_at_a_box_across_its_path() {
    let others = [
        rect(1, 100.0, 100.0, 20.0, 20.0),
        // Straddles the matched coordinate without sharing it, so reaching the
        // far box would run the line through this one.
        rect(2, 80.0, 150.0, 50.0, 20.0),
        rect(3, 100.0, 200.0, 20.0, 20.0),
    ];
    let resolution = solve(Rect::new(100.0, 0.0, 20.0, 20.0), &others);
    assert_eq!(resolution.snap_dx, 0.0);
    assert!(resolution
        .state
        .guides
        .iter()
        .all(|line| line.start.x == 100.0 && line.end.x == 100.0));
    let segments: Vec<(f64, f64)> = resolution
        .state
        .guides
        .iter()
        .map(|line| (line.start.y, line.end.y))
        .collect();
    assert_eq!(
        segments,
        vec![(20.0, 100.0), (120.0, 150.0), (170.0, 200.0)]
    );
}

#[test]
fn alignment_guide_meets_a_rotated_target_at_its_corner() {
    // A square turned 45 degrees: the frame's left corner sits on the
    // envelope's left edge, level with the centre. Matching the envelope would
    // end the line on its own corner, far below the shape.
    let square = element(1, 0.0, 0.0, 100.0, 100.0, std::f64::consts::FRAC_PI_4);
    let resolution = solve(Rect::new(square.bounds.x, 400.0, 20.0, 20.0), &[square]);
    assert_eq!(resolution.snap_dx, 0.0);
    assert_eq!(resolution.state.guides.len(), 1);
    let line = resolution.state.guides[0];
    assert_eq!(line.start.x, square.bounds.x);
    assert!((line.start.y - 50.0).abs() < 1e-9, "{line:?}");
    assert_eq!(line.end.y, 400.0);
}

#[test]
fn alignment_guide_leaves_a_rotated_mover_at_its_corner() {
    // The same square on the moving side: the line leaves its own corner
    // rather than a corner of the envelope the drag reads.
    let square = element(9, 0.0, 0.0, 100.0, 100.0, std::f64::consts::FRAC_PI_4);
    let target = rect(1, square.bounds.x, 400.0, 20.0, 20.0);
    let resolution = resolve_move(
        square.bounds,
        square.corners,
        &[target],
        view(),
        1.0,
        &SmartGuideState::default(),
    );
    assert_eq!(resolution.snap_dx, 0.0);
    assert_eq!(resolution.state.guides.len(), 1);
    let line = resolution.state.guides[0];
    assert_eq!(line.start.x, square.bounds.x);
    assert!((line.start.y - 50.0).abs() < 1e-9, "{line:?}");
    assert_eq!(line.end.y, 400.0);
}

#[test]
fn alignment_guide_meets_a_rotated_target_horizontally() {
    // The diamond matched on the other axis: its top corner is a single
    // point, so the line starts there rather than at the envelope's top edge.
    let square = element(1, 0.0, 0.0, 100.0, 100.0, std::f64::consts::FRAC_PI_4);
    let moving = Rect::new(400.0, square.bounds.y, 20.0, 20.0);
    let resolution = solve(moving, &[square]);
    assert_eq!(resolution.snap_dx, 0.0);
    assert_eq!(resolution.snap_dy, 0.0);
    assert_eq!(resolution.state.guides.len(), 1);
    let line = resolution.state.guides[0];
    assert_eq!(line.start.y, square.bounds.y);
    assert!((line.start.x - 50.0).abs() < 1e-9, "{line:?}");
    assert_eq!(line.end.x, 400.0);
}

#[test]
fn alignment_guide_cuts_a_rotated_blocker_at_its_frame() {
    // A diamond whose envelope straddles the matched coordinate, but whose
    // frame only crosses it 5px inside: the line is cut at the frame, not at
    // the box drawn around it.
    let diamond = element(2, 55.0, 650.0, 100.0, 100.0, std::f64::consts::FRAC_PI_4);
    let others = [
        rect(1, 100.0, 100.0, 20.0, 20.0),
        diamond,
        rect(3, 100.0, 900.0, 20.0, 20.0),
    ];
    let resolution = solve(Rect::new(100.0, 0.0, 20.0, 20.0), &others);
    let segments: Vec<(f64, f64)> = resolution
        .state
        .guides
        .iter()
        .map(|line| (line.start.y, line.end.y))
        .collect();
    let reach = 50.0 * 2.0f64.sqrt();
    assert_eq!(segments.len(), 3);
    assert_eq!(segments[0], (20.0, 100.0));
    assert!((segments[1].0 - 120.0).abs() < 1e-9 && (segments[1].1 - (705.0 - reach)).abs() < 1e-9);
    assert!((segments[2].0 - (695.0 + reach)).abs() < 1e-9 && (segments[2].1 - 900.0).abs() < 1e-9);
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
            start: Point::new(100.0, 20.0),
            end: Point::new(100.0, 250.0),
            stops: [100.0, 100.0],
            kind: GuideKind::Alignment,
        }
    );
    assert_eq!(
        resolution.state.guides[1],
        GuideLine {
            start: Point::new(20.0, -9.0),
            end: Point::new(100.0, -9.0),
            stops: [0.0, 0.0],
            kind: GuideKind::EqualSpacing,
        }
    );
    assert_eq!(
        resolution.state.guides[3],
        GuideLine {
            start: Point::new(220.0, -9.0),
            end: Point::new(300.0, -9.0),
            stops: [0.0, 0.0],
            kind: GuideKind::EqualSpacing,
        }
    );
}

#[test]
fn runs_need_two_matching_gaps() {
    let boxes = solver::sorted_along(
        &[
            guide(1, Rect::new(0.0, 0.0, 20.0, 20.0)),
            guide(2, Rect::new(100.0, 0.0, 20.0, 20.0)),
            guide(3, Rect::new(600.0, 0.0, 20.0, 20.0)),
        ],
        GuideAxis::Vertical,
    );
    let lane = GUIDE_LANE_PX;
    let pairs = inline_pairs(&boxes, GuideAxis::Vertical, lane);
    assert!(equal_spacing_run_lines(
        &boxes,
        &pairs,
        GuideAxis::Vertical,
        &Tolerances::for_zoom(1.0)
    )
    .is_empty());
}

#[test]
fn runs_skip_a_gap_crossed_by_an_object() {
    let lane = GUIDE_LANE_PX;
    let run_count = |boxes: &[GuideBox]| {
        let boxes = solver::sorted_along(boxes, GuideAxis::Vertical);
        let pairs = inline_pairs(&boxes, GuideAxis::Vertical, lane);
        equal_spacing_run_lines(
            &boxes,
            &pairs,
            GuideAxis::Vertical,
            &Tolerances::for_zoom(1.0),
        )
        .len()
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
fn only_targets_on_screen_are_considered() {
    let moving = Rect::new(0.0, 0.0, 20.0, 20.0);
    let view = Rect::new(0.0, 0.0, 300.0, 300.0);
    let near = [rect(1, 5.0, 280.0, 20.0, 20.0)];
    let far = [rect(1, 5.0, 340.0, 20.0, 20.0)];
    let solve_in =
        |others: &[GuideBox]| resolve(moving, others, view, 1.0, &SmartGuideState::default());

    let snapped = solve_in(&near);
    assert_eq!((snapped.snap_dx, snapped.snap_dy), (5.0, 0.0));
    assert_eq!(snapped.state.active.len(), 1);

    // 40 world px further out, so off screen and out of reach.
    let ignored = solve_in(&far);
    assert_eq!((ignored.snap_dx, ignored.snap_dy), (0.0, 0.0));
    assert!(ignored.state.active.is_empty());
}

#[test]
fn targets_far_away_on_screen_still_align() {
    let moving = Rect::new(0.0, 0.0, 20.0, 20.0);
    let view = Rect::new(-500.0, -500.0, 4_000.0, 4_000.0);
    let far = [rect(1, 5.0, 3_000.0, 20.0, 20.0)];

    let resolution = resolve(moving, &far, view, 1.0, &SmartGuideState::default());
    assert_eq!((resolution.snap_dx, resolution.snap_dy), (5.0, 0.0));
    assert_eq!(resolution.state.active.len(), 1);
}

#[test]
fn guides_are_trimmed_to_the_screen() {
    let others = [rect(1, 0.0, 0.0, 100.0, 80.0)];
    let view = Rect::new(-50.0, -50.0, 400.0, 400.0);

    // The moving box is dragged below the screen while still sharing its left
    // edge with the box above it.
    let moving = Rect::new(0.0, 400.0, 100.0, 80.0);
    let resolution = resolve(moving, &others, view, 1.0, &SmartGuideState::default());
    assert_eq!(resolution.state.guides.len(), 1);
    assert_eq!(resolution.state.guides[0].start, Point::new(0.0, 80.0));
    // The span stops at the bottom edge instead of running to the box.
    assert_eq!(resolution.state.guides[0].end, Point::new(0.0, 350.0));
}

#[test]
fn spacing_pairs_are_bounded_by_the_reach() {
    let others = [
        rect(1, 0.0, 0.0, 20.0, 100.0),
        rect(2, 1_300.0, 0.0, 20.0, 100.0),
    ];
    let moving = Rect::new(650.0, 20.0, 20.0, 20.0);
    let view = Rect::new(-500.0, -500.0, 3_000.0, 3_000.0);

    // 630px from each box, just past the pair reach at zoom 1...
    let zoomed_in = resolve(moving, &others, view, 1.0, &SmartGuideState::default());
    assert!(zoomed_in.state.active.is_empty());

    // ...but the reach is measured in screen pixels, so zooming out brings
    // the pair back into play.
    let zoomed_out = resolve(moving, &others, view, 0.5, &SmartGuideState::default());
    assert_eq!(zoomed_out.state.active[0].kind, GuideKind::EqualSpacing);
}

#[test]
fn spacing_labels_report_the_gap() {
    let others = [
        rect(1, 0.0, 0.0, 20.0, 100.0),
        rect(2, 100.0, 0.0, 20.0, 100.0),
    ];
    let spacing = solve(Rect::new(44.0, 20.0, 20.0, 20.0), &others);
    assert_eq!(spacing.state.active[0].kind, GuideKind::EqualSpacing);
    // Centred, both bands measure 30; the label hangs off the first one.
    assert_eq!(
        spacing.state.labels,
        vec![GuideLabel {
            at: Point::new(35.0, -9.0),
            distance: 30.0,
            axis: GuideAxis::Vertical,
        }]
    );

    // Alignment on its own is not a measurement.
    let aligned = solve(Rect::new(0.0, 300.0, 20.0, 20.0), &others[..1]);
    assert_eq!(aligned.state.active[0].kind, GuideKind::Alignment);
    assert!(aligned.state.labels.is_empty());
}

#[test]
fn chain_labels_report_the_closed_gap() {
    let others = [
        rect(1, 0.0, 0.0, 20.0, 20.0),
        rect(2, 100.0, 0.0, 20.0, 20.0),
    ];
    let resolution = solve(Rect::new(202.0, 8.0, 20.0, 20.0), &others);
    assert_eq!(resolution.state.active[0].kind, GuideKind::ChainSpacing);
    assert_eq!(
        resolution.state.labels,
        vec![GuideLabel {
            at: Point::new(160.0, -9.0),
            distance: 80.0,
            axis: GuideAxis::Vertical,
        }]
    );
}

#[test]
fn labels_slide_into_view_and_drop_with_their_band() {
    let others = [
        rect(1, 0.0, 0.0, 20.0, 20.0),
        rect(2, 100.0, 0.0, 20.0, 20.0),
    ];
    // The chain's closed gap runs from the last box out to the moving box,
    // which sits past the right edge: the label slides left to stay visible.
    let visible = Rect::new(-50.0, -50.0, 200.0, 400.0);
    let resolution = resolve(
        Rect::new(202.0, 8.0, 20.0, 20.0),
        &others,
        visible,
        1.0,
        &SmartGuideState::default(),
    );
    assert_eq!(resolution.state.labels[0].at, Point::new(150.0, -9.0));
    assert_eq!(resolution.state.labels[0].distance, 80.0);

    // A window whose top edge clips the band's lane entirely: the measurement
    // goes with it, since there is nothing on screen to read it against.
    let clipped = Rect::new(-50.0, 0.0, 400.0, 400.0);
    let resolution = resolve(
        Rect::new(202.0, 8.0, 20.0, 20.0),
        &others,
        clipped,
        1.0,
        &SmartGuideState::default(),
    );
    assert_eq!(resolution.state.active[0].kind, GuideKind::ChainSpacing);
    assert!(resolution.state.labels.is_empty());
}

#[test]
fn no_targets_clear_the_previous_state() {
    let previous = SmartGuideState {
        guides: Vec::new(),
        labels: Vec::new(),
        active: vec![GuideKey {
            axis: GuideAxis::Vertical,
            kind: GuideKind::Alignment,
            moving_feature: GuideFeature::Start,
            target_feature: GuideFeature::Start,
            target_a: 1,
            target_b: 0,
        }],
    };
    let resolution = resolve(
        Rect::new(10.0, 0.0, 20.0, 20.0),
        &[],
        view(),
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
        let others: Vec<GuideBox> = (0..count)
            .map(|i| {
                rect(
                    i as ElementId + 1,
                    rng.next(),
                    rng.next(),
                    rng.next().abs() + 0.1,
                    rng.next().abs() + 0.1,
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
        let resolution = resolve(proposed, &others, view(), zoom, &state);
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
