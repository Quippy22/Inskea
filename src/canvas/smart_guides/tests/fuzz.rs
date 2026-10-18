//! Randomized resolution over mixed boxes, with snapping and guides kept
//! finite.

use super::*;

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
                let (x, y, w, h) = (
                    rng.next(),
                    rng.next(),
                    rng.next().abs() + 0.1,
                    rng.next().abs() + 0.1,
                );
                if i % 2 == 0 {
                    rect(i as ElementId + 1, x, y, w, h)
                } else {
                    // Ellipses put the tangent solve under the same fuzz.
                    ellipse(i as ElementId + 1, x, y, w, h, rng.next())
                }
            })
            .collect();
        let zoom = [0.25, 1.0, 4.0][round % 3];
        let proposed = Rect::new(
            rng.next(),
            rng.next(),
            rng.next().abs() + 0.1,
            rng.next().abs() + 0.1,
        );
        let resolution = if round % 4 == 3 {
            // And an ellipse on the moving side solves its half too.
            resolve_move(
                proposed,
                GuideFrame {
                    corners: proposed.corners(),
                    shape: FrameShape::Ellipse,
                },
                &others,
                view(),
                zoom,
                &state,
            )
        } else {
            resolve(proposed, &others, view(), zoom, &state)
        };
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
