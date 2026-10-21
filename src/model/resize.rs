use super::elements::Element;
use super::{ElementData, Point};

pub const MIN_ELEMENT_SIZE: f64 = 5.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ResizeHandle {
    Nw,
    N,
    Ne,
    W,
    E,
    Sw,
    S,
    Se,
}

impl From<usize> for ResizeHandle {
    fn from(i: usize) -> Self {
        match i {
            0 => ResizeHandle::Nw,
            1 => ResizeHandle::N,
            2 => ResizeHandle::Ne,
            3 => ResizeHandle::W,
            4 => ResizeHandle::E,
            5 => ResizeHandle::Sw,
            6 => ResizeHandle::S,
            7 => ResizeHandle::Se,
            _ => ResizeHandle::Nw,
        }
    }
}

impl ResizeHandle {
    pub fn css_cursor(self) -> &'static str {
        match self {
            ResizeHandle::Nw => "nw-resize",
            ResizeHandle::N => "n-resize",
            ResizeHandle::Ne => "ne-resize",
            ResizeHandle::W => "w-resize",
            ResizeHandle::E => "e-resize",
            ResizeHandle::Sw => "sw-resize",
            ResizeHandle::S => "s-resize",
            ResizeHandle::Se => "se-resize",
        }
    }

    pub fn free_axes(self) -> (bool, bool) {
        match self {
            ResizeHandle::Nw | ResizeHandle::Ne | ResizeHandle::Sw | ResizeHandle::Se => {
                (true, true)
            }
            ResizeHandle::N | ResizeHandle::S => (false, true),
            ResizeHandle::W | ResizeHandle::E => (true, false),
        }
    }

    pub fn opposite_anchor(self, el: &ElementData) -> Point {
        let x = el.world_point.x;
        let y = el.world_point.y;
        let w = el.width;
        let h = el.height;
        match self {
            ResizeHandle::Nw => Point::new(x + w, y + h),
            ResizeHandle::N => Point::new(x, y + h),
            ResizeHandle::Ne => Point::new(x, y + h),
            ResizeHandle::W => Point::new(x + w, y),
            ResizeHandle::E => Point::new(x, y),
            ResizeHandle::Sw => Point::new(x + w, y),
            ResizeHandle::S => Point::new(x, y),
            ResizeHandle::Se => Point::new(x, y),
        }
    }
}

pub fn rotate_point_around(p: Point, center: Point, angle: f64) -> Point {
    let dx = p.x - center.x;
    let dy = p.y - center.y;
    let (cos_a, sin_a) = (angle.cos(), angle.sin());
    Point::new(
        center.x + dx * cos_a - dy * sin_a,
        center.y + dx * sin_a + dy * cos_a,
    )
}

pub fn resize_from_handle(
    original: &ElementData,
    handle: ResizeHandle,
    pointer_world: Point,
    shift: bool,
    alt: bool,
) -> ElementData {
    let cx = original.world_point.x + original.width / 2.0;
    let cy = original.world_point.y + original.height / 2.0;
    let center = Point::new(cx, cy);

    let local_pointer = rotate_point_around(pointer_world, center, -original.rotation);

    let anchor = handle.opposite_anchor(original);
    let (free_x, free_y) = handle.free_axes();

    let mut new_w = original.width;
    let mut new_h = original.height;
    let mut new_local_x = original.world_point.x;
    let mut new_local_y = original.world_point.y;
    let mut flip_x = false;
    let mut flip_y = false;

    if alt {
        if free_x {
            let half_w = (local_pointer.x - cx).abs().max(MIN_ELEMENT_SIZE / 2.0);
            new_w = 2.0 * half_w;
        }
        if free_y {
            let half_h = (local_pointer.y - cy).abs().max(MIN_ELEMENT_SIZE / 2.0);
            new_h = 2.0 * half_h;
        }
    } else {
        if free_x {
            let delta = local_pointer.x - anchor.x;
            flip_x = delta >= 0.0;
            new_w = delta.abs().max(MIN_ELEMENT_SIZE);
        }
        if free_y {
            let delta = local_pointer.y - anchor.y;
            flip_y = delta >= 0.0;
            new_h = delta.abs().max(MIN_ELEMENT_SIZE);
        }

        if shift {
            let ratio = original.width / original.height;
            let nratio = new_w / new_h;
            if nratio > ratio {
                new_h = new_w / ratio;
            } else {
                new_w = new_h * ratio;
            }
        }

        // Keep the opposite edge fixed. Once the pointer crosses it, the
        // box flips to the other side instead of dragging a tiny box along.
        if free_x {
            new_local_x = if flip_x { anchor.x } else { anchor.x - new_w };
        }
        if free_y {
            new_local_y = if flip_y { anchor.y } else { anchor.y - new_h };
        }

        match handle {
            ResizeHandle::N | ResizeHandle::S => {
                new_w = original.width;
                new_local_x = original.world_point.x;
            }
            ResizeHandle::W | ResizeHandle::E => {
                new_h = original.height;
                new_local_y = original.world_point.y;
            }
            _ => {}
        }
    }

    if new_w < MIN_ELEMENT_SIZE || new_h < MIN_ELEMENT_SIZE {
        return original.clone();
    }

    let (new_cx, new_cy) = if alt {
        (cx, cy)
    } else {
        let new_local_center = Point::new(new_local_x + new_w / 2.0, new_local_y + new_h / 2.0);
        let c = rotate_point_around(new_local_center, center, original.rotation);
        (c.x, c.y)
    };

    ElementData {
        world_point: Point::new(new_cx - new_w / 2.0, new_cy - new_h / 2.0),
        width: new_w,
        height: new_h,
        rotation: original.rotation,
        style: original.style.clone(),
        ..*original
    }
}

#[derive(Clone, Copy)]
pub struct ResizeContext<'a> {
    pub orig: &'a Element,
    pub handle: ResizeHandle,
    pub pointer_world: Point,
    /// World point where the resize drag started (the grab point). Deltas
    /// against it keep the size from jumping on pointer-down when the handle
    /// sits outside the element's layout box (text handles do).
    pub start_world: Point,
    pub shift: bool,
    pub alt: bool,
    pub multi: bool,
    pub bx: f64,
    pub by: f64,
    pub bw: f64,
    pub bh: f64,
}

pub fn resize_bbox(
    pos: Point,
    size: Point,
    pointer_world: Point,
    handle: ResizeHandle,
    shift: bool,
    alt: bool,
) -> Option<(Point, Point)> {
    let (bx, by) = (pos.x, pos.y);
    let (bw, bh) = (size.x, size.y);
    let (free_x, free_y) = handle.free_axes();

    let mut nx = bx;
    let mut ny = by;
    let mut nw = bw;
    let mut nh = bh;

    if alt {
        let cx = bx + bw / 2.0;
        let cy = by + bh / 2.0;
        if free_x {
            let half_w = (pointer_world.x - cx).abs().max(MIN_ELEMENT_SIZE / 2.0);
            nw = 2.0 * half_w;
        }
        if free_y {
            let half_h = (pointer_world.y - cy).abs().max(MIN_ELEMENT_SIZE / 2.0);
            nh = 2.0 * half_h;
        }
    } else {
        match handle {
            ResizeHandle::Nw => {
                nx = pointer_world.x;
                ny = pointer_world.y;
                nw = (bx + bw) - pointer_world.x;
                nh = (by + bh) - pointer_world.y;
            }
            ResizeHandle::N => {
                ny = pointer_world.y;
                nh = (by + bh) - pointer_world.y;
            }
            ResizeHandle::Ne => {
                ny = pointer_world.y;
                nw = pointer_world.x - bx;
                nh = (by + bh) - pointer_world.y;
            }
            ResizeHandle::W => {
                nx = pointer_world.x;
                nw = (bx + bw) - pointer_world.x;
            }
            ResizeHandle::E => {
                nw = pointer_world.x - bx;
            }
            ResizeHandle::Sw => {
                nx = pointer_world.x;
                nw = (bx + bw) - pointer_world.x;
                nh = pointer_world.y - by;
            }
            ResizeHandle::S => {
                nh = pointer_world.y - by;
            }
            ResizeHandle::Se => {
                nw = pointer_world.x - bx;
                nh = pointer_world.y - by;
            }
        }

        if nw < MIN_ELEMENT_SIZE || nh < MIN_ELEMENT_SIZE {
            return None;
        }

        if shift {
            let ratio = bw / bh;
            let nratio = nw / nh;
            if nratio > ratio {
                nh = nw / ratio;
            } else {
                nw = nh * ratio;
            }
            match handle {
                ResizeHandle::Nw => {
                    nx = bx + bw - nw;
                    ny = by + bh - nh;
                }
                ResizeHandle::N | ResizeHandle::Ne => {
                    ny = by + bh - nh;
                }
                ResizeHandle::W | ResizeHandle::Sw => {
                    nx = bx + bw - nw;
                }
                _ => {}
            }
        }

        match handle {
            ResizeHandle::N | ResizeHandle::S => {
                nx = bx;
                nw = bw;
            }
            ResizeHandle::W | ResizeHandle::E => {
                ny = by;
                nh = bh;
            }
            _ => {}
        }
    }

    if nw < MIN_ELEMENT_SIZE || nh < MIN_ELEMENT_SIZE {
        return None;
    }

    Some((Point::new(nx, ny), Point::new(nw, nh)))
}

#[derive(Clone, Copy, Debug)]
pub struct ResizeScale {
    pub position: Point,
    pub size: Point,
    pub original_bounds: super::Rect,
    /// Text recalculates its height from the scaled font and wrapping.
    pub scale_height: bool,
}

pub fn resize_scale_element(data: &mut ElementData, orig: &ElementData, scale: ResizeScale) {
    let bounds = scale.original_bounds;
    let sx = scale.size.x / bounds.w.max(MIN_ELEMENT_SIZE);
    let sy = scale.size.y / bounds.h.max(MIN_ELEMENT_SIZE);
    data.world_point.set(
        (orig.world_point.x - bounds.x) * sx + scale.position.x,
        (orig.world_point.y - bounds.y) * sy + scale.position.y,
    );
    data.width = (orig.width * sx).max(MIN_ELEMENT_SIZE);
    if scale.scale_height {
        data.height = (orig.height * sy).max(MIN_ELEMENT_SIZE);
    }
}

#[cfg(test)]
mod tests {
    use super::super::elements::{EdgeStyle, ElementStyle, StrokeStyle};
    use super::*;

    fn default_data() -> ElementData {
        ElementData {
            id: 1,
            world_point: Point { x: 0.0, y: 0.0 },
            width: 100.0,
            height: 100.0,
            rotation: 0.0,
            style: ElementStyle {
                font_size: 24.0,
                stroke_color: super::super::Color::new(super::super::Color::BLUE),
                fill_color: None,
                stroke_width: 2.0,
                stroke_style: StrokeStyle::Solid,
                edge_style: EdgeStyle::Sharp,
                roundness: 6.0,
                opacity: 1.0,
            },
        }
    }

    #[test]
    fn rotate_point_around_identity() {
        let p = rotate_point_around(Point::new(10.0, 20.0), Point::new(5.0, 5.0), 0.0);
        assert_eq!(p, Point::new(10.0, 20.0));
    }

    #[test]
    fn rotate_point_around_180() {
        let p = rotate_point_around(
            Point::new(10.0, 0.0),
            Point::new(0.0, 0.0),
            std::f64::consts::PI,
        );
        assert!((p.x + 10.0).abs() < 1e-10);
        assert!((p.y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn rotate_point_around_90() {
        let p = rotate_point_around(
            Point::new(10.0, 0.0),
            Point::new(0.0, 0.0),
            std::f64::consts::FRAC_PI_2,
        );
        assert!((p.x - 0.0).abs() < 1e-10);
        assert!((p.y - 10.0).abs() < 1e-10);
    }

    #[test]
    fn resizing_unrotated_rectangle_from_se_anchors_nw() {
        let original = default_data();
        let result = resize_from_handle(
            &original,
            ResizeHandle::Se,
            Point::new(150.0, 120.0),
            false,
            false,
        );
        assert!((result.world_point.x - 0.0).abs() < 0.01);
        assert!((result.world_point.y - 0.0).abs() < 0.01);
        assert!((result.width - 150.0).abs() < 0.01);
        assert!((result.height - 120.0).abs() < 0.01);
    }

    #[test]
    fn resizing_45_degree_rotated_rectangle_does_not_skew() {
        let original = ElementData {
            width: 100.0,
            height: 100.0,
            rotation: std::f64::consts::FRAC_PI_4,
            ..default_data()
        };
        let center = Point::new(
            original.world_point.x + original.width / 2.0,
            original.world_point.y + original.height / 2.0,
        );
        // drag a point that, in LOCAL space, is a pure diagonal extension
        let local_target = Point::new(150.0, 150.0);
        let world_target = rotate_point_around(local_target, center, original.rotation);
        let result = resize_from_handle(&original, ResizeHandle::Se, world_target, false, false);
        // width and height should scale evenly, not skew into an irregular shape
        assert!((result.width - result.height).abs() < 0.01);
    }

    #[test]
    fn rotate_point_around_works() {
        let p = rotate_point_around(
            Point::new(1.0, 0.0),
            Point::new(0.0, 0.0),
            std::f64::consts::FRAC_PI_2,
        );
        assert!((p.x - 0.0).abs() < 1e-10);
        assert!((p.y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn resize_north_handle_only_changes_height() {
        let original = default_data();
        // Drag N handle to y=80 (20px above bottom at y=100) → height = 20, top at y=80
        let result = resize_from_handle(
            &original,
            ResizeHandle::N,
            Point::new(50.0, 80.0),
            false,
            false,
        );
        assert!((result.width - 100.0).abs() < 0.01);
        assert!((result.height - 20.0).abs() < 0.01);
        assert!((result.world_point.y - 80.0).abs() < 0.01);
        assert!((result.world_point.x - 0.0).abs() < 0.01);
    }

    #[test]
    fn resize_east_handle_only_changes_width() {
        let original = default_data();
        let result = resize_from_handle(
            &original,
            ResizeHandle::E,
            Point::new(150.0, 50.0),
            false,
            false,
        );
        assert!((result.height - 100.0).abs() < 0.01);
        assert!((result.width - 150.0).abs() < 0.01);
        assert!((result.world_point.x - 0.0).abs() < 0.01);
    }

    #[test]
    fn resize_west_keeps_the_east_edge_at_the_minimum_size() {
        let original = default_data();
        let result = resize_from_handle(
            &original,
            ResizeHandle::W,
            Point::new(98.0, 50.0),
            false,
            false,
        );
        assert!((result.world_point.x - 95.0).abs() < 0.01);
        assert!((result.width - MIN_ELEMENT_SIZE).abs() < 0.01);
    }

    #[test]
    fn resize_west_past_the_east_edge_flips_to_the_right() {
        let original = default_data();
        let result = resize_from_handle(
            &original,
            ResizeHandle::W,
            Point::new(130.0, 50.0),
            false,
            false,
        );
        assert!((result.world_point.x - 100.0).abs() < 0.01);
        assert!((result.width - 30.0).abs() < 0.01);
    }

    #[test]
    fn resize_shift_locks_aspect_ratio() {
        let original = default_data();
        // Drag SE to (200, 150) — wider than tall
        let result = resize_from_handle(
            &original,
            ResizeHandle::Se,
            Point::new(200.0, 150.0),
            true,
            false,
        );
        // Aspect ratio should be 1:1 (100/100)
        assert!((result.width - result.height).abs() < 0.01);
    }

    #[test]
    fn resize_around_center_works() {
        let original = default_data();
        let result = resize_from_handle(
            &original,
            ResizeHandle::Se,
            Point::new(150.0, 120.0),
            false,
            true,
        );
        // Center should stay at (50, 50)
        let cx = result.world_point.x + result.width / 2.0;
        let cy = result.world_point.y + result.height / 2.0;
        assert!((cx - 50.0).abs() < 0.01);
        assert!((cy - 50.0).abs() < 0.01);
    }

    #[test]
    fn resize_scale_element_doubles_width() {
        let mut data = ElementData {
            id: 2,
            world_point: Point::new(0.0, 0.0),
            width: 100.0,
            height: 50.0,
            rotation: 0.0,
            style: ElementStyle {
                font_size: 24.0,
                stroke_color: super::super::Color::new(super::super::Color::BLUE),
                fill_color: None,
                stroke_width: 2.0,
                stroke_style: StrokeStyle::Solid,
                edge_style: EdgeStyle::Sharp,
                roundness: 6.0,
                opacity: 1.0,
            },
        };
        let orig = ElementData {
            id: 1,
            world_point: Point::new(0.0, 0.0),
            width: 100.0,
            height: 50.0,
            rotation: 0.0,
            style: ElementStyle {
                font_size: 24.0,
                stroke_color: super::super::Color::new(super::super::Color::BLUE),
                fill_color: None,
                stroke_width: 2.0,
                stroke_style: StrokeStyle::Solid,
                edge_style: EdgeStyle::Sharp,
                roundness: 6.0,
                opacity: 1.0,
            },
        };
        resize_scale_element(
            &mut data,
            &orig,
            ResizeScale {
                position: Point::new(0.0, 0.0),
                size: Point::new(200.0, 100.0),
                original_bounds: super::super::Rect::new(0.0, 0.0, 100.0, 50.0),
                scale_height: true,
            },
        );
        assert!((data.width - 200.0).abs() < 0.01);
        assert!((data.height - 100.0).abs() < 0.01);
        assert!((data.world_point.x - 0.0).abs() < 0.01);
        assert!((data.world_point.y - 0.0).abs() < 0.01);
    }

    #[test]
    fn resize_scale_element_skips_height_when_set_height_false() {
        let mut data = ElementData {
            id: 2,
            world_point: Point::new(0.0, 0.0),
            width: 100.0,
            height: 50.0,
            rotation: 0.0,
            style: ElementStyle {
                font_size: 24.0,
                stroke_color: super::super::Color::new(super::super::Color::BLUE),
                fill_color: None,
                stroke_width: 2.0,
                stroke_style: StrokeStyle::Solid,
                edge_style: EdgeStyle::Sharp,
                roundness: 6.0,
                opacity: 1.0,
            },
        };
        let orig = ElementData {
            id: 1,
            world_point: Point::new(0.0, 0.0),
            width: 100.0,
            height: 50.0,
            rotation: 0.0,
            style: ElementStyle {
                font_size: 24.0,
                stroke_color: super::super::Color::new(super::super::Color::BLUE),
                fill_color: None,
                stroke_width: 2.0,
                stroke_style: StrokeStyle::Solid,
                edge_style: EdgeStyle::Sharp,
                roundness: 6.0,
                opacity: 1.0,
            },
        };
        resize_scale_element(
            &mut data,
            &orig,
            ResizeScale {
                position: Point::new(0.0, 0.0),
                size: Point::new(200.0, 100.0),
                original_bounds: super::super::Rect::new(0.0, 0.0, 100.0, 50.0),
                scale_height: false,
            },
        );
        assert!((data.width - 200.0).abs() < 0.01);
        assert!((data.height - 50.0).abs() < 0.01); // unchanged
    }
}
