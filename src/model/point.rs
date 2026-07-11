/// A 2-D point in world space.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Point {
    /// Horizontal coordinate (positive right).
    pub x: f64,
    /// Vertical coordinate (positive down).
    pub y: f64,
}

impl Point {
    /// Create a new `Point` at `(x, y)`.
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Set both coordinates at once.
    pub fn set(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
    }

    /// Translate this point by `dx` horizontally and `dy` vertically.
    pub fn offset(&mut self, dx: f64, dy: f64) {
        self.x += dx;
        self.y += dy;
    }

    /// Snap this point to the nearest grid line with the given `grid` spacing.
    pub fn snap_to_grid(&mut self, grid: f64) {
        self.set(
            (self.x / grid).round() * grid,
            (self.y / grid).round() * grid,
        );
    }

    /// Rotate this point around `other` by `delta` radians.
    pub fn rotate_around(&mut self, other: Self, delta: f64) {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let cos = delta.cos();
        let sin = delta.sin();
        self.set(other.x + dx * cos - dy * sin, other.y + dx * sin + dy * cos);
    }

    /// Return the four world-space corners of a rectangle with this point
    /// as the top-left corner, rotated around its center by `rotation` radians.
    pub fn rect_corners(self, width: f64, height: f64, rotation: f64) -> [Point; 4] {
        let cx = self.x + width / 2.0;
        let cy = self.y + height / 2.0;
        let center = Point::new(cx, cy);
        let corners = [
            Point::new(self.x, self.y),
            Point::new(self.x + width, self.y),
            Point::new(self.x + width, self.y + height),
            Point::new(self.x, self.y + height),
        ];
        if rotation == 0.0 {
            corners
        } else {
            corners.map(|mut c| {
                c.rotate_around(center, rotation);
                c
            })
        }
    }
}

impl Default for Point {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}
