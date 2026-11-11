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

pub const EDGE_GRACE_PX: f64 = 40.0;

/// Compute the continuous camera follow delta when moving a shape.
///
/// Triggered when the cursor enters the edge grace space, touches the canvas edge,
/// or when the moving shape is out of view.
/// Uses the distance (`delta`) between the center of the view and the edge where the shape is,
/// moving the camera continuously to follow the shape.
pub fn edge_follow_delta(
    cursor_screen: Point,
    screen_size: Point,
    shape_bounds: Option<Rect>,
    viewport: Viewport,
) -> Option<Point> {
    if screen_size.x <= 0.0 || screen_size.y <= 0.0 {
        return None;
    }
    let zoom = viewport.effective_zoom();
    let delta_x = (screen_size.x / zoom) / 2.0;
    let delta_y = (screen_size.y / zoom) / 2.0;

    let view_left = viewport.offset_x - delta_x;
    let view_right = viewport.offset_x + delta_x;
    let view_top = viewport.offset_y - delta_y;
    let view_bottom = viewport.offset_y + delta_y;

    let (shape_left, shape_right, shape_top, shape_bottom) = if let Some(b) = shape_bounds {
        (b.x, b.x + b.w, b.y, b.y + b.h)
    } else {
        let world = viewport.screen_to_world(cursor_screen, screen_size);
        (world.x, world.x, world.y, world.y)
    };

    let mut scroll_dx = 0.0;
    let mut scroll_dy = 0.0;

    // Horizontal check: right edge (grace space, touching edge, or shape out of view)
    if cursor_screen.x >= screen_size.x - EDGE_GRACE_PX || shape_right > view_right {
        let factor =
            ((cursor_screen.x - (screen_size.x - EDGE_GRACE_PX)) / EDGE_GRACE_PX).clamp(0.2, 3.0);
        scroll_dx = (8.0 + factor * 10.0) / zoom;
    } else if cursor_screen.x <= EDGE_GRACE_PX || shape_left < view_left {
        let factor = ((EDGE_GRACE_PX - cursor_screen.x) / EDGE_GRACE_PX).clamp(0.2, 3.0);
        scroll_dx = -(8.0 + factor * 10.0) / zoom;
    }

    // Vertical check: bottom edge (grace space, touching edge, or shape out of view)
    if cursor_screen.y >= screen_size.y - EDGE_GRACE_PX || shape_bottom > view_bottom {
        let factor =
            ((cursor_screen.y - (screen_size.y - EDGE_GRACE_PX)) / EDGE_GRACE_PX).clamp(0.2, 3.0);
        scroll_dy = (8.0 + factor * 10.0) / zoom;
    } else if cursor_screen.y <= EDGE_GRACE_PX || shape_top < view_top {
        let factor = ((EDGE_GRACE_PX - cursor_screen.y) / EDGE_GRACE_PX).clamp(0.2, 3.0);
        scroll_dy = -(8.0 + factor * 10.0) / zoom;
    }

    if scroll_dx != 0.0 || scroll_dy != 0.0 {
        Some(Point::new(scroll_dx, scroll_dy))
    } else {
        None
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

    #[test]
    fn edge_follow_delta_triggers_on_grace_space_and_edges() {
        let vp = Viewport {
            offset_x: 0.0,
            offset_y: 0.0,
            zoom: 1.0,
        };
        let screen_size = Point::new(1000.0, 800.0);

        // Center of screen: no scroll
        let center = Point::new(500.0, 400.0);
        assert_eq!(edge_follow_delta(center, screen_size, None, vp), None);

        // Within right edge grace space (cx = 970 >= 1000 - 40)
        let right_edge = Point::new(970.0, 400.0);
        let delta = edge_follow_delta(right_edge, screen_size, None, vp);
        assert!(delta.is_some());
        assert!(delta.unwrap().x > 0.0);
        assert_eq!(delta.unwrap().y, 0.0);

        // Within left edge grace space (cx = 30 <= 40)
        let left_edge = Point::new(30.0, 400.0);
        let delta = edge_follow_delta(left_edge, screen_size, None, vp);
        assert!(delta.is_some());
        assert!(delta.unwrap().x < 0.0);
        assert_eq!(delta.unwrap().y, 0.0);

        // Within bottom edge grace space (cy = 770 >= 800 - 40)
        let bottom_edge = Point::new(500.0, 770.0);
        let delta = edge_follow_delta(bottom_edge, screen_size, None, vp);
        assert!(delta.is_some());
        assert_eq!(delta.unwrap().x, 0.0);
        assert!(delta.unwrap().y > 0.0);

        // Within top edge grace space (cy = 20 <= 40)
        let top_edge = Point::new(500.0, 20.0);
        let delta = edge_follow_delta(top_edge, screen_size, None, vp);
        assert!(delta.is_some());
        assert_eq!(delta.unwrap().x, 0.0);
        assert!(delta.unwrap().y < 0.0);

        // Shape out of view on right even if cursor is at center
        let shape_past_right = Some(Rect::new(510.0, 0.0, 50.0, 50.0));
        let delta = edge_follow_delta(center, screen_size, shape_past_right, vp);
        assert!(delta.is_some());
        assert!(delta.unwrap().x > 0.0);

        // Shape out of view on left
        let shape_past_left = Some(Rect::new(-560.0, 0.0, 50.0, 50.0));
        let delta = edge_follow_delta(center, screen_size, shape_past_left, vp);
        assert!(delta.is_some());
        assert!(delta.unwrap().x < 0.0);

        // Shape out of view on bottom
        let shape_past_bottom = Some(Rect::new(0.0, 410.0, 50.0, 50.0));
        let delta = edge_follow_delta(center, screen_size, shape_past_bottom, vp);
        assert!(delta.is_some());
        assert!(delta.unwrap().y > 0.0);

        // Shape out of view on top
        let shape_past_top = Some(Rect::new(0.0, -460.0, 50.0, 50.0));
        let delta = edge_follow_delta(center, screen_size, shape_past_top, vp);
        assert!(delta.is_some());
        assert!(delta.unwrap().y < 0.0);
    }
}
