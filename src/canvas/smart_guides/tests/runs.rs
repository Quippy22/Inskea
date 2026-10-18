//! Run highlighting: when a row's rhythm is shown, and when it is noise.

use super::super::candidates::AxisScan;
use super::*;

#[test]
fn runs_are_highlighted_while_an_alignment_is_active() {
    let others = [
        rect(1, 0.0, 0.0, 20.0, 20.0),
        rect(2, 100.0, 0.0, 20.0, 20.0),
        rect(3, 200.0, 0.0, 20.0, 20.0),
        rect(4, 300.0, 0.0, 20.0, 20.0),
    ];
    // The moving box rides just below the row, inside its lane.
    let resolution = solve(Rect::new(106.0, 30.0, 30.0, 20.0), &others);
    assert_eq!(resolution.snap_dx, -6.0);
    assert_eq!(resolution.state.guides.len(), 4);
    assert_eq!(
        resolution.state.guides[0],
        GuideLine {
            start: Point::new(100.0, 20.0),
            end: Point::new(100.0, 30.0),
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
        resolution.state.guides[2],
        GuideLine {
            start: Point::new(120.0, -9.0),
            end: Point::new(200.0, -9.0),
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
fn runs_hide_when_the_mover_leaves_the_lane() {
    // The same row with the dragged box parked below it, aligned on the axis
    // only: the row's rhythm says nothing about the drag, so its gaps stay
    // unhighlighted while the alignment line still connects the two.
    let others = [
        rect(1, 0.0, 0.0, 20.0, 20.0),
        rect(2, 100.0, 0.0, 20.0, 20.0),
        rect(3, 200.0, 0.0, 20.0, 20.0),
        rect(4, 300.0, 0.0, 20.0, 20.0),
    ];
    let resolution = solve(Rect::new(106.0, 250.0, 20.0, 20.0), &others);
    assert_eq!(resolution.snap_dx, -6.0);
    assert_eq!(
        resolution.state.guides,
        vec![GuideLine {
            start: Point::new(100.0, 20.0),
            end: Point::new(100.0, 250.0),
            stops: [100.0, 100.0],
            kind: GuideKind::Alignment,
        }]
    );
}

#[test]
fn runs_need_two_matching_gaps() {
    let boxes = [
        guide(1, Rect::new(0.0, 0.0, 20.0, 20.0)),
        guide(2, Rect::new(100.0, 0.0, 20.0, 20.0)),
        guide(3, Rect::new(600.0, 0.0, 20.0, 20.0)),
    ];
    let scan = AxisScan::new(GuideAxis::Vertical, &boxes, GUIDE_LANE_PX);
    assert!(equal_spacing_run_lines(
        Rect::new(0.0, 0.0, 20.0, 20.0),
        &scan,
        &Tolerances::for_zoom(1.0)
    )
    .is_empty());
}

#[test]
fn runs_skip_a_gap_crossed_by_an_object() {
    let run_count = |boxes: &[GuideBox]| {
        let scan = AxisScan::new(GuideAxis::Vertical, boxes, GUIDE_LANE_PX);
        equal_spacing_run_lines(
            Rect::new(0.0, 0.0, 20.0, 20.0),
            &scan,
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
