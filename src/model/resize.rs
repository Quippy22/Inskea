use super::elements::Element;
use super::ElementData;
use super::Point;

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
    /// Which axes are free to change when dragging this handle.
    pub fn free_axes(self) -> (bool, bool) {
        match self {
            ResizeHandle::Nw | ResizeHandle::Ne | ResizeHandle::Sw | ResizeHandle::Se => (true, true),
            ResizeHandle::N | ResizeHandle::S => (false, true),
            ResizeHandle::W | ResizeHandle::E => (true, false),
        }
    }

    /// The local-space anchor point (corner/edge opposite the handle)
    /// that stays fixed during a non-symmetric resize.
    pub fn opposite_anchor(self, el: &ElementData) -> (f64, f64) {
        let x = el.world_point.x;
        let y = el.world_point.y;
        let w = el.width;
        let h = el.height;
        match self {
            ResizeHandle::Nw => (x + w, y + h),
            ResizeHandle::N => (x, y + h),
            ResizeHandle::Ne => (x, y + h),
            ResizeHandle::W => (x + w, y),
            ResizeHandle::E => (x, y),
            ResizeHandle::Sw => (x + w, y),
            ResizeHandle::S => (x, y),
            ResizeHandle::Se => (x, y),
        }
    }
}

/// Rotate a point `p` around `center` by `angle` radians (counter-clockwise).
pub fn rotate_point_around(p: (f64, f64), center: (f64, f64), angle: f64) -> (f64, f64) {
    let (px, py) = (p.0 - center.0, p.1 - center.1);
    let (cos_a, sin_a) = (angle.cos(), angle.sin());
    (
        center.0 + px * cos_a - py * sin_a,
        center.1 + px * sin_a + py * cos_a,
    )
}

/// Axis-aligned bounding box that encloses all given elements,
/// accounting for their individual rotations.
pub fn common_bounds(elements: &[&ElementData]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    for el in elements {
        for c in el.world_point.rect_corners(el.width, el.height, el.rotation) {
            min_x = min_x.min(c.x);
            min_y = min_y.min(c.y);
            max_x = max_x.max(c.x);
            max_y = max_y.max(c.y);
        }
    }
    (min_x, min_y, max_x - min_x, max_y - min_y)
}

/// Resize an element using the local-frame approach.
///
/// 1. Un-rotate the pointer into the element's local (unrotated) frame.
/// 2. Compute new width/height anchored on the opposite corner/edge.
/// 3. Rotate the new center back into world space.
///
/// `shift` locks aspect ratio; `alt` resizes symmetrically around center.
pub fn resize_from_handle(
    original: &ElementData,
    handle: ResizeHandle,
    pointer_world: (f64, f64),
    shift: bool,
    alt: bool,
) -> ElementData {
    let cx = original.world_point.x + original.width / 2.0;
    let cy = original.world_point.y + original.height / 2.0;
    let center = (cx, cy);

    let local_pointer = rotate_point_around(pointer_world, center, -original.rotation);

    let anchor = handle.opposite_anchor(original);
    let (free_x, free_y) = handle.free_axes();

    let mut new_w = original.width;
    let mut new_h = original.height;
    let mut new_local_x = original.world_point.x;
    let mut new_local_y = original.world_point.y;

    if alt {
        if free_x {
            let half_w = (local_pointer.0 - cx).abs().max(MIN_ELEMENT_SIZE / 2.0);
            new_w = 2.0 * half_w;
        }
        if free_y {
            let half_h = (local_pointer.1 - cy).abs().max(MIN_ELEMENT_SIZE / 2.0);
            new_h = 2.0 * half_h;
        }
    } else {
        if free_x {
            new_w = (local_pointer.0 - anchor.0).abs().max(MIN_ELEMENT_SIZE);
            new_local_x = local_pointer.0.min(anchor.0);
        }
        if free_y {
            new_h = (local_pointer.1 - anchor.1).abs().max(MIN_ELEMENT_SIZE);
            new_local_y = local_pointer.1.min(anchor.1);
        }
    }

        if shift {
            let ratio = original.width / original.height;
            let nratio = new_w / new_h;
            if nratio > ratio {
                new_h = new_w / ratio;
                if !alt && free_y && handle == ResizeHandle::N {
                    new_local_y = anchor.1 - new_h;
                }
            } else {
                new_w = new_h * ratio;
                if !alt && free_x && handle == ResizeHandle::W {
                    new_local_x = anchor.0 - new_w;
                }
            }
        }

    // For edge handles (non-corner), the constrained axis keeps its original size.
    // But for corner handles where alt is active, we already computed both from center.
    if !alt {
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
        let new_local_center = (new_local_x + new_w / 2.0, new_local_y + new_h / 2.0);
        rotate_point_around(new_local_center, center, original.rotation)
    };

    ElementData {
        world_point: Point {
            x: new_cx - new_w / 2.0,
            y: new_cy - new_h / 2.0,
        },
        width: new_w,
        height: new_h,
        rotation: original.rotation,
        ..*original
    }
}

/// Parameters for resizing an element via drag handles.
#[derive(Clone, Copy)]
pub struct ResizeContext<'a> {
    pub orig: &'a Element,
    pub handle: ResizeHandle,
    pub pointer_world: (f64, f64),
    pub shift: bool,
    pub alt: bool,
    pub multi: bool,
    pub bx: f64,
    pub by: f64,
    pub bw: f64,
    pub bh: f64,
}

/// Compute the axis-aligned bounding-box change for multi-element scaling.
pub fn resize_bbox(
    pos: Point,
    size: (f64, f64),
    pointer_world: (f64, f64),
    handle: ResizeHandle,
    shift: bool,
    alt: bool,
) -> Option<(Point, (f64, f64))> {
    let (bx, by) = (pos.x, pos.y);
    let (bw, bh) = size;
    let (free_x, free_y) = handle.free_axes();

    let mut nx = bx;
    let mut ny = by;
    let mut nw = bw;
    let mut nh = bh;

    if alt {
        let cx = bx + bw / 2.0;
        let cy = by + bh / 2.0;
        if free_x {
            let half_w = (pointer_world.0 - cx).abs().max(MIN_ELEMENT_SIZE / 2.0);
            nw = 2.0 * half_w;
        }
        if free_y {
            let half_h = (pointer_world.1 - cy).abs().max(MIN_ELEMENT_SIZE / 2.0);
            nh = 2.0 * half_h;
        }
    } else {
        match handle {
            ResizeHandle::Nw => { nx = pointer_world.0; ny = pointer_world.1; nw = (bx + bw) - pointer_world.0; nh = (by + bh) - pointer_world.1; }
            ResizeHandle::N => { ny = pointer_world.1; nh = (by + bh) - pointer_world.1; }
            ResizeHandle::Ne => { ny = pointer_world.1; nw = pointer_world.0 - bx; nh = (by + bh) - pointer_world.1; }
            ResizeHandle::W => { nx = pointer_world.0; nw = (bx + bw) - pointer_world.0; }
            ResizeHandle::E => { nw = pointer_world.0 - bx; }
            ResizeHandle::Sw => { nx = pointer_world.0; nw = (bx + bw) - pointer_world.0; nh = pointer_world.1 - by; }
            ResizeHandle::S => { nh = pointer_world.1 - by; }
            ResizeHandle::Se => { nw = pointer_world.0 - bx; nh = pointer_world.1 - by; }
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
                ResizeHandle::Nw => { nx = bx + bw - nw; ny = by + bh - nh; }
                ResizeHandle::N | ResizeHandle::Ne => { ny = by + bh - nh; }
                ResizeHandle::W | ResizeHandle::Sw => { nx = bx + bw - nw; }
                _ => {}
            }
        }

        match handle {
            ResizeHandle::N | ResizeHandle::S => { nx = bx; nw = bw; }
            ResizeHandle::W | ResizeHandle::E => { ny = by; nh = bh; }
            _ => {}
        }
    }

    if nw < MIN_ELEMENT_SIZE || nh < MIN_ELEMENT_SIZE {
        return None;
    }

    Some((Point { x: nx, y: ny }, (nw, nh)))
}

/// Scale an element's position and size proportionally to a new bounding-box.
///
/// `pos` is the new top-left corner of the bounds, `(nw, nh)` the new size.
/// `(bx, by, bw, bh)` is the original bounds box. The scaling is derived from
/// the ratio of `(nw, nh)` to the clamped bounds size.
///
/// When `set_height` is `false` (used by `Text`), the element's height is kept
/// unchanged — the caller manages it separately via `resize_text`.
#[allow(clippy::too_many_arguments)]
pub fn resize_scale_element(
    data: &mut ElementData,
    orig: &ElementData,
    pos: Point,
    nw: f64,
    nh: f64,
    bx: f64,
    by: f64,
    bw: f64,
    bh: f64,
    set_height: bool,
) {
    let obw = bw.max(MIN_ELEMENT_SIZE);
    let obh = bh.max(MIN_ELEMENT_SIZE);
    let sx = nw / obw;
    let sy = nh / obh;
    data.world_point.set(
        (orig.world_point.x - bx) * sx + pos.x,
        (orig.world_point.y - by) * sy + pos.y,
    );
    data.width = (orig.width * sx).max(MIN_ELEMENT_SIZE);
    if set_height {
        data.height = (orig.height * sy).max(MIN_ELEMENT_SIZE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::elements::{EdgeStyle, ElementStyle, StrokeStyle};

    fn default_data() -> ElementData {
        ElementData {
            id: 1,
            world_point: Point { x: 0.0, y: 0.0 },
            width: 100.0,
            height: 100.0,
            rotation: 0.0,
            style: ElementStyle {
                font_size: 24.0,
                stroke_color: super::super::ShapeColor::Blue,
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
        let p = rotate_point_around((10.0, 20.0), (5.0, 5.0), 0.0);
        assert_eq!(p, (10.0, 20.0));
    }

    #[test]
    fn rotate_point_around_180() {
        let p = rotate_point_around((10.0, 0.0), (0.0, 0.0), std::f64::consts::PI);
        assert!((p.0 + 10.0).abs() < 1e-10);
        assert!((p.1 - 0.0).abs() < 1e-10);
    }

    #[test]
    fn rotate_point_around_90() {
        let p = rotate_point_around((10.0, 0.0), (0.0, 0.0), std::f64::consts::FRAC_PI_2);
        assert!((p.0 - 0.0).abs() < 1e-10);
        assert!((p.1 - 10.0).abs() < 1e-10);
    }

    #[test]
    fn resizing_unrotated_rectangle_from_se_anchors_nw() {
        let original = default_data();
        let result = resize_from_handle(&original, ResizeHandle::Se, (150.0, 120.0), false, false);
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
        let center = (
            original.world_point.x + original.width / 2.0,
            original.world_point.y + original.height / 2.0,
        );
        // drag a point that, in LOCAL space, is a pure diagonal extension
        let local_target = (150.0, 150.0);
        let world_target = rotate_point_around(local_target, center, original.rotation);
        let result = resize_from_handle(&original, ResizeHandle::Se, world_target, false, false);
        // width and height should scale evenly, not skew into an irregular shape
        assert!((result.width - result.height).abs() < 0.01);
    }

    #[test]
    fn rotate_point_around_works() {
        let p = rotate_point_around((1.0, 0.0), (0.0, 0.0), std::f64::consts::FRAC_PI_2);
        assert!((p.0 - 0.0).abs() < 1e-10);
        assert!((p.1 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn rect_corners_rotated_90() {
        let p = Point::new(0.0, 0.0);
        let corners = p.rect_corners(100.0, 50.0, std::f64::consts::FRAC_PI_2);
        // Center at (50, 25), rotate 90° CW → top-left (-25, 50), etc.
        assert!((corners[0].x - 75.0).abs() < 0.01);
        assert!((corners[0].y - -25.0).abs() < 0.01);
    }

    #[test]
    fn rotated_corners_unrotated() {
        let el = default_data();
        let corners = el.world_point.rect_corners(el.width, el.height, el.rotation);
        assert_eq!(corners[0], Point { x: 0.0, y: 0.0 });
        assert_eq!(corners[1], Point { x: 100.0, y: 0.0 });
        assert_eq!(corners[2], Point { x: 100.0, y: 100.0 });
        assert_eq!(corners[3], Point { x: 0.0, y: 100.0 });
    }

    #[test]
    fn common_bounds_single_element() {
        let el = default_data();
        let (x, y, w, h) = common_bounds(&[&el]);
        assert!((x - 0.0).abs() < 0.01);
        assert!((y - 0.0).abs() < 0.01);
        assert!((w - 100.0).abs() < 0.01);
        assert!((h - 100.0).abs() < 0.01);
    }

    #[test]
    fn resize_north_handle_only_changes_height() {
        let original = default_data();
        // Drag N handle to y=80 (20px above bottom at y=100) → height = 20, top at y=80
        let result = resize_from_handle(&original, ResizeHandle::N, (50.0, 80.0), false, false);
        assert!((result.width - 100.0).abs() < 0.01);
        assert!((result.height - 20.0).abs() < 0.01);
        assert!((result.world_point.y - 80.0).abs() < 0.01);
        assert!((result.world_point.x - 0.0).abs() < 0.01);
    }

    #[test]
    fn resize_east_handle_only_changes_width() {
        let original = default_data();
        let result = resize_from_handle(&original, ResizeHandle::E, (150.0, 50.0), false, false);
        assert!((result.height - 100.0).abs() < 0.01);
        assert!((result.width - 150.0).abs() < 0.01);
        assert!((result.world_point.x - 0.0).abs() < 0.01);
    }

    #[test]
    fn resize_shift_locks_aspect_ratio() {
        let original = default_data();
        // Drag SE to (200, 150) — wider than tall
        let result = resize_from_handle(&original, ResizeHandle::Se, (200.0, 150.0), true, false);
        // Aspect ratio should be 1:1 (100/100)
        assert!((result.width - result.height).abs() < 0.01);
    }

    #[test]
    fn resize_around_center_works() {
        let original = default_data();
        let result = resize_from_handle(&original, ResizeHandle::Se, (150.0, 120.0), false, true);
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
                stroke_color: super::super::ShapeColor::Blue,
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
                stroke_color: super::super::ShapeColor::Blue,
                fill_color: None,
                stroke_width: 2.0,
                stroke_style: StrokeStyle::Solid,
                edge_style: EdgeStyle::Sharp,
                roundness: 6.0,
                opacity: 1.0,
            },
        };
        resize_scale_element(&mut data, &orig, Point::new(0.0, 0.0), 200.0, 100.0, 0.0, 0.0, 100.0, 50.0, true);
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
                stroke_color: super::super::ShapeColor::Blue,
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
                stroke_color: super::super::ShapeColor::Blue,
                fill_color: None,
                stroke_width: 2.0,
                stroke_style: StrokeStyle::Solid,
                edge_style: EdgeStyle::Sharp,
                roundness: 6.0,
                opacity: 1.0,
            },
        };
        resize_scale_element(&mut data, &orig, Point::new(0.0, 0.0), 200.0, 100.0, 0.0, 0.0, 100.0, 50.0, false);
        assert!((data.width - 200.0).abs() < 0.01);
        assert!((data.height - 50.0).abs() < 0.01); // unchanged
    }
}
