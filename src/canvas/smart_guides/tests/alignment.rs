//! Alignment guides: which boxes connect, and where the lines meet the
//! shapes on either side.

use super::*;

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
    let segments = y_segments(&resolution);
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
        square.frame,
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
    let segments = y_segments(&resolution);
    let reach = 50.0 * 2.0f64.sqrt();
    assert_eq!(segments.len(), 3);
    assert_eq!(segments[0], (20.0, 100.0));
    assert!((segments[1].0 - 120.0).abs() < 1e-9 && (segments[1].1 - (705.0 - reach)).abs() < 1e-9);
    assert!((segments[2].0 - (695.0 + reach)).abs() < 1e-9 && (segments[2].1 - 900.0).abs() < 1e-9);
}

#[test]
fn alignment_guide_meets_a_circle_at_its_tangent() {
    // A circle matched on its left edge: the line ends level with the centre,
    // where it touches the outline, not on the corner of the box around it.
    let circle = ellipse(1, 0.0, 0.0, 100.0, 100.0, 0.0);
    let resolution = solve(Rect::new(circle.bounds.x, 400.0, 20.0, 20.0), &[circle]);
    assert_eq!(resolution.snap_dx, 0.0);
    assert_eq!(resolution.state.guides.len(), 1);
    let line = resolution.state.guides[0];
    assert_eq!(line.start.x, 0.0);
    assert!((line.start.y - 50.0).abs() < 1e-9, "{line:?}");
    assert_eq!(line.end.y, 400.0);
}

#[test]
fn alignment_guide_meets_a_circle_at_its_tangent_horizontally() {
    // The same on the other axis, through the ellipse solver's other branch.
    let circle = ellipse(1, 0.0, 0.0, 100.0, 100.0, 0.0);
    let resolution = solve(Rect::new(400.0, circle.bounds.y, 20.0, 20.0), &[circle]);
    assert_eq!(resolution.snap_dy, 0.0);
    assert_eq!(resolution.state.guides.len(), 1);
    let line = resolution.state.guides[0];
    assert_eq!(line.start.y, 0.0);
    assert!((line.start.x - 50.0).abs() < 1e-9, "{line:?}");
    assert_eq!(line.end.x, 400.0);
}

#[test]
fn alignment_guide_meets_a_turned_circle_at_its_tangent() {
    // Turning a circle changes nothing about its outline or its extent: the
    // box's envelope would swing out by sqrt(2), the guides keep riding the
    // circle itself.
    let circle = ellipse(1, 0.0, 0.0, 100.0, 100.0, std::f64::consts::FRAC_PI_4);
    let resolution = solve(Rect::new(circle.bounds.x, 400.0, 20.0, 20.0), &[circle]);
    assert!(resolution.snap_dx.abs() < 1e-9);
    assert_eq!(resolution.state.guides.len(), 1);
    let line = resolution.state.guides[0];
    assert!((line.start.x).abs() < 1e-9, "{line:?}");
    // A turned graze is ill-conditioned: the tangent point can come out with
    // float noise around it, far below a pixel.
    assert!((line.start.y - 50.0).abs() < 1e-5, "{line:?}");
    assert_eq!(line.end.y, 400.0);
}

#[test]
fn alignment_guide_meets_a_turned_oval_at_its_tangent() {
    // An oval is not a circle: the extreme that faces the guide sits off the
    // centre line, and the line must end where the tangent touches there.
    let oval = ellipse(1, 0.0, 0.0, 200.0, 100.0, std::f64::consts::FRAC_PI_4);
    let resolution = solve(Rect::new(oval.bounds.x, 400.0, 20.0, 20.0), &[oval]);
    assert!(resolution.snap_dx.abs() < 1e-9);
    assert_eq!(resolution.state.guides.len(), 1);
    let line = resolution.state.guides[0];
    // That tangent point sits (rx^2 - ry^2) sin * cos / half_x off the
    // centre, on the side where the outline's tangent turns vertical.
    let (sin, cos) = std::f64::consts::FRAC_PI_4.sin_cos();
    let half_x = ((100.0 * cos).powi(2) + (50.0 * sin).powi(2)).sqrt();
    let drop = (100.0f64.powi(2) - 50.0f64.powi(2)) * sin * cos / half_x;
    assert!((line.start.y - (50.0 - drop)).abs() < 1e-9, "{line:?}");
    assert_eq!(line.end.y, 400.0);
}

#[test]
fn alignment_guide_cuts_a_circle_at_its_chord() {
    // A circle standing in the way, left of its centre: the line is cut at
    // the chord of the outline, not at the box drawn around the circle.
    let circle = ellipse(2, 50.0, 100.0, 100.0, 100.0, 0.0);
    let others = [rect(1, 75.0, 0.0, 20.0, 20.0), circle];
    let resolution = solve(Rect::new(75.0, 800.0, 20.0, 20.0), &others);
    let segments = y_segments(&resolution);
    let reach = (50.0f64 * 50.0 - 25.0 * 25.0).sqrt();
    assert_eq!(segments.len(), 2);
    assert!(
        (segments[0].0 - 20.0).abs() < 1e-9 && (segments[0].1 - (150.0 - reach)).abs() < 1e-9,
        "{segments:?}"
    );
    assert!(
        (segments[1].0 - (150.0 + reach)).abs() < 1e-9 && (segments[1].1 - 800.0).abs() < 1e-9,
        "{segments:?}"
    );
}

#[test]
fn alignment_guide_cuts_a_circle_far_from_the_origin_at_its_chord() {
    // The same cut, two million units out, where the viewport has followed:
    // the span must stay the chord's width, not collapse to a point because
    // of where the world origin happens to be.
    let circle = ellipse(2, 50.0, 2_000_100.0, 100.0, 100.0, 0.0);
    let others = [rect(1, 75.0, 2_000_000.0, 20.0, 20.0), circle];
    let visible = Rect::new(-1_000.0, 1_999_000.0, 2_000.0, 2_000.0);
    let resolution = resolve(
        Rect::new(75.0, 2_000_800.0, 20.0, 20.0),
        &others,
        visible,
        1.0,
        &SmartGuideState::default(),
    );
    let segments = y_segments(&resolution);
    let reach = (50.0f64 * 50.0 - 25.0 * 25.0).sqrt();
    assert_eq!(segments.len(), 2);
    assert!(
        (segments[0].0 - 2_000_020.0).abs() < 1e-9
            && (segments[0].1 - (2_000_150.0 - reach)).abs() < 1e-9,
        "{segments:?}"
    );
    assert!(
        (segments[1].0 - (2_000_150.0 + reach)).abs() < 1e-9
            && (segments[1].1 - 2_000_800.0).abs() < 1e-9,
        "{segments:?}"
    );
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
