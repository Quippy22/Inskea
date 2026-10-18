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
    /// World rectangle the screen currently shows.
    pub fn visible_rect(self, screen_size: Point) -> Rect {
        let zoom = if self.zoom.is_finite() && self.zoom > 0.0 {
            self.zoom
        } else {
            1.0
        };
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
}
