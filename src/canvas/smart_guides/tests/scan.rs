//! How the scan is bounded: screen-space tolerances, hysteresis against the
//! previous frame, the visible window, and the reach.

use super::*;

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
