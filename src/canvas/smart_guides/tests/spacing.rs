//! Equal-spacing and chain-spacing: snapping into a repeated gap, the bands
//! drawn for every match, and their labels.

use super::*;

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
