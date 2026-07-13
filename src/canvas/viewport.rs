/// The single source of truth for "what part of the infinite world is
/// currently visible, and at what scale."
///
/// Convention: `offset_x`/`offset_y` is the WORLD-space point currently
/// centered on screen. World (0,0) is centered on screen when
/// offset is (0.0, 0.0) and zoom is 1.0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    /// World-space X coordinate currently centered on screen.
    pub offset_x: f64,
    /// World-space Y coordinate currently centered on screen.
    pub offset_y: f64,
    /// Scale multiplier. 1.0 = 100%, 2.0 = 200% zoomed in.
    pub zoom: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            zoom: 1.0,
        }
    }
}

impl Viewport {
    /// Compute the SVG `viewBox` attribute string for a given screen size
    /// (the current pixel width/height of the `<svg>` element).
    ///
    /// This is what actually shifts world (0,0) to the visual center of
    /// the screen, since SVG's native viewBox convention is top-left-origin.
    pub fn to_view_box(self, screen_width: f64, screen_height: f64) -> String {
        let view_width = screen_width / self.zoom;
        let view_height = screen_height / self.zoom;
        let min_x = self.offset_x - view_width / 2.0;
        let min_y = self.offset_y - view_height / 2.0;

        format!("{min_x} {min_y} {view_width} {view_height}")
    }

    /// Convert a screen-space point (e.g. from a pointer event, relative to
    /// the SVG element's top-left corner) into a world-space point.
    pub fn screen_to_world(&self, screen: (f64, f64), screen_size: (f64, f64)) -> (f64, f64) {
        let (sx, sy) = screen;
        let (sw, sh) = screen_size;
        (
            self.offset_x + (sx - sw / 2.0) / self.zoom,
            self.offset_y + (sy - sh / 2.0) / self.zoom,
        )
    }

    /// Convert a world-space point into a screen-space point (e.g. to
    /// position an HTML overlay like the text-edit textarea on top of a
    /// world-space element).
    pub fn world_to_screen(&self, world: (f64, f64), screen_size: (f64, f64)) -> (f64, f64) {
        let (wx, wy) = world;
        let (sw, sh) = screen_size;
        (
            (wx - self.offset_x) * self.zoom + sw / 2.0,
            (wy - self.offset_y) * self.zoom + sh / 2.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_viewport_centers_world_origin_on_screen_center() {
        let vp = Viewport::default();
        let screen_size = (1000.0, 800.0);

        let screen_center = (500.0, 400.0);
        let world = vp.screen_to_world(screen_center, screen_size);
        assert!((world.0 - 0.0).abs() < f64::EPSILON);
        assert!((world.1 - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn round_trip_screen_to_world_to_screen() {
        let vp = Viewport {
            offset_x: 120.0,
            offset_y: -40.0,
            zoom: 1.75,
        };
        let screen_size = (1024.0, 768.0);
        let original_screen = (300.0, 650.0);

        let world = vp.screen_to_world(original_screen, screen_size);
        let back_to_screen = vp.world_to_screen(world, screen_size);

        assert!((original_screen.0 - back_to_screen.0).abs() < 1e-9);
        assert!((original_screen.1 - back_to_screen.1).abs() < 1e-9);
    }

    #[test]
    fn view_box_is_centered_on_offset() {
        let vp = Viewport {
            offset_x: 0.0,
            offset_y: 0.0,
            zoom: 1.0,
        };
        let view_box = vp.to_view_box(1000.0, 800.0);
        assert_eq!(view_box, "-500 -400 1000 800");
    }
}
