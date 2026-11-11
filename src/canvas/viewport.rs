use crate::model::{Point, Rect};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub offset_x: f64,
    pub offset_y: f64,
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
    /// The zoom clamped to a finite positive value.
    pub fn effective_zoom(self) -> f64 {
        if self.zoom.is_finite() && self.zoom > 0.0 {
            self.zoom
        } else {
            1.0
        }
    }

    /// World units covered by one screen pixel at the current zoom.
    pub fn world_per_px(self) -> f64 {
        1.0 / self.effective_zoom()
    }

    /// World rectangle the screen currently shows.
    pub fn visible_rect(self, screen_size: Point) -> Rect {
        let zoom = self.effective_zoom();
        let width = screen_size.x / zoom;
        let height = screen_size.y / zoom;
        Rect::new(
            self.offset_x - width / 2.0,
            self.offset_y - height / 2.0,
            width,
            height,
        )
    }

    pub fn to_view_box(self, screen_width: f64, screen_height: f64) -> String {
        let view = self.visible_rect(Point::new(screen_width, screen_height));
        format!("{} {} {} {}", view.x, view.y, view.w, view.h)
    }

    pub fn screen_to_world(&self, screen: Point, screen_size: Point) -> Point {
        Point::new(
            self.offset_x + (screen.x - screen_size.x / 2.0) / self.zoom,
            self.offset_y + (screen.y - screen_size.y / 2.0) / self.zoom,
        )
    }

    pub fn world_to_screen(&self, world: Point, screen_size: Point) -> Point {
        Point::new(
            (world.x - self.offset_x) * self.zoom + screen_size.x / 2.0,
            (world.y - self.offset_y) * self.zoom + screen_size.y / 2.0,
        )
    }

    /// Zoom the viewport by `factor` keeping the screen point `center_screen` stationary in world coordinates.
    pub fn zoom_at_point(
        &mut self,
        factor: f64,
        center_screen: Point,
        screen_size: Point,
        min_zoom: f64,
        max_zoom: f64,
    ) {
        let world = self.screen_to_world(center_screen, screen_size);
        self.zoom = (self.zoom * factor).clamp(min_zoom, max_zoom);
        self.offset_x = world.x - (center_screen.x - screen_size.x / 2.0) / self.zoom;
        self.offset_y = world.y - (center_screen.y - screen_size.y / 2.0) / self.zoom;
    }

    /// Zoom and pan the viewport from a two-finger pinch gesture.
    pub fn pinch_zoom(
        &mut self,
        factor: f64,
        prev_center: Point,
        new_center: Point,
        screen_size: Point,
        min_zoom: f64,
        max_zoom: f64,
    ) {
        let world = self.screen_to_world(prev_center, screen_size);
        self.zoom = (self.zoom * factor).clamp(min_zoom, max_zoom);
        self.offset_x = world.x - (new_center.x - screen_size.x / 2.0) / self.zoom;
        self.offset_y = world.y - (new_center.y - screen_size.y / 2.0) / self.zoom;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_viewport_centers_world_origin_on_screen_center() {
        let vp = Viewport::default();
        let screen_size = Point::new(1000.0, 800.0);
        let screen_center = Point::new(500.0, 400.0);
        let world = vp.screen_to_world(screen_center, screen_size);
        assert!((world.x - 0.0).abs() < f64::EPSILON);
        assert!((world.y - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn round_trip_screen_to_world_to_screen() {
        let vp = Viewport {
            offset_x: 120.0,
            offset_y: -40.0,
            zoom: 1.75,
        };
        let screen_size = Point::new(1024.0, 768.0);
        let original_screen = Point::new(300.0, 650.0);

        let world = vp.screen_to_world(original_screen, screen_size);
        let back_to_screen = vp.world_to_screen(world, screen_size);

        assert!((original_screen.x - back_to_screen.x).abs() < 1e-9);
        assert!((original_screen.y - back_to_screen.y).abs() < 1e-9);
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

    #[test]
    fn world_per_px_inverts_zoom() {
        let vp = Viewport {
            zoom: 2.0,
            ..Viewport::default()
        };
        assert!((vp.world_per_px() - 0.5).abs() < f64::EPSILON);

        let invalid = Viewport {
            zoom: 0.0,
            ..Viewport::default()
        };
        assert_eq!(invalid.world_per_px(), 1.0);
    }

    #[test]
    fn visible_rect_scales_with_zoom() {
        let vp = Viewport {
            offset_x: 100.0,
            offset_y: -50.0,
            zoom: 2.0,
        };
        assert_eq!(
            vp.visible_rect(Point::new(800.0, 400.0)),
            Rect::new(100.0 - 200.0, -50.0 - 100.0, 400.0, 200.0)
        );
    }

    #[test]
    fn zoom_at_point_preserves_world_position_under_cursor() {
        let mut vp = Viewport {
            offset_x: 50.0,
            offset_y: -30.0,
            zoom: 1.0,
        };
        let screen_size = Point::new(1000.0, 800.0);
        let cursor_screen = Point::new(350.0, 600.0);
        let world_before = vp.screen_to_world(cursor_screen, screen_size);

        vp.zoom_at_point(1.5, cursor_screen, screen_size, 0.1, 20.0);

        let screen_after = vp.world_to_screen(world_before, screen_size);
        assert!((screen_after.x - cursor_screen.x).abs() < 1e-9);
        assert!((screen_after.y - cursor_screen.y).abs() < 1e-9);
    }

    #[test]
    fn pinch_zoom_pans_and_zooms_around_gesture() {
        let mut vp = Viewport {
            offset_x: 0.0,
            offset_y: 0.0,
            zoom: 1.0,
        };
        let screen_size = Point::new(1000.0, 800.0);
        let prev_center = Point::new(400.0, 400.0);
        let new_center = Point::new(450.0, 420.0);
        let world_at_prev = vp.screen_to_world(prev_center, screen_size);

        vp.pinch_zoom(2.0, prev_center, new_center, screen_size, 0.1, 20.0);

        let screen_after = vp.world_to_screen(world_at_prev, screen_size);
        assert!((screen_after.x - new_center.x).abs() < 1e-9);
        assert!((screen_after.y - new_center.y).abs() < 1e-9);
        assert!((vp.zoom - 2.0).abs() < 1e-9);
    }
}
