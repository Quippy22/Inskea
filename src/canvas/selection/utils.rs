use crate::canvas::combined_bounds;
use crate::model::interaction::{layout_bounds, visual_bounds, visual_envelope};
use crate::model::{Element, ElementId, Point, Rect};

pub const HANDLE_RESIZE_RADIUS: f64 = 5.0;
pub const HANDLE_MOVE_RADIUS: f64 = 6.0;
pub const HANDLE_ROTATE_RADIUS: f64 = 7.0;
pub const ROTATE_HANDLE_OFFSET: f64 = 25.0;
pub const PATH_MERGE_DIST: f64 = 3.0;
pub const SELECT_PADDING: f64 = 5.0;

/// Round `value` to the nearest multiple of `step`; a non-positive step leaves
/// the value untouched.
pub fn snap_value(value: f64, step: f64) -> f64 {
    if step <= 0.0 {
        value
    } else {
        (value / step).round() * step
    }
}

pub fn selection_bounds(ids: &[ElementId], els: &[Element]) -> Option<Rect> {
    if ids.len() == 1 {
        els.iter()
            .find(|e| e.id() == ids[0])
            .map(visual_bounds)
            .or_else(|| combined_bounds(ids, els))
    } else {
        els.iter()
            .filter(|el| ids.contains(&el.id()))
            .map(visual_envelope)
            .reduce(|left, right| left.union(&right))
    }
}

pub fn handle_positions(bx: f64, by: f64, bw: f64, bh: f64) -> [Point; 10] {
    [
        Point::new(bx, by),
        Point::new(bx + bw / 2.0, by),
        Point::new(bx + bw, by),
        Point::new(bx, by + bh / 2.0),
        Point::new(bx + bw, by + bh / 2.0),
        Point::new(bx, by + bh),
        Point::new(bx + bw / 2.0, by + bh),
        Point::new(bx + bw, by + bh),
        Point::new(bx + bw / 2.0, by + bh / 2.0),
        Point::new(bx + bw / 2.0, by - 25.0),
    ]
}

pub fn unrotate_for_element(point: Point, el: &Element) -> Point {
    let rot = el.data().rotation;
    if rot == 0.0 {
        return point;
    }
    let b = layout_bounds(el);
    let cx = b.x + b.w / 2.0;
    let cy = b.y + b.h / 2.0;
    Point::unrotate(point, cx, cy, rot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::elements::rect::Rectangle;
    use crate::model::ElementData;

    #[test]
    fn unrotate_handle_hit_lands_in_element_frame() {
        let el = Element::Rectangle(Rectangle {
            data: ElementData {
                world_point: Point::new(100.0, 100.0),
                width: 100.0,
                height: 100.0,
                rotation: std::f64::consts::FRAC_PI_2,
                ..ElementData::new(0)
            },
        });
        let local = Point::new(150.0, 50.0);
        let world = Point::new(250.0, 150.0);
        let un = unrotate_for_element(world, &el);
        assert!((un.x - local.x).abs() < 1e-9);
        assert!((un.y - local.y).abs() < 1e-9);
    }

    #[test]
    fn snap_value_rounds_to_the_nearest_step() {
        assert_eq!(snap_value(0.0, 30.0), 0.0);
        assert_eq!(snap_value(14.9, 30.0), 0.0);
        assert_eq!(snap_value(15.0, 30.0), 30.0);
        assert_eq!(snap_value(44.0, 30.0), 30.0);
        assert_eq!(snap_value(-31.0, 30.0), -30.0);
    }

    #[test]
    fn snap_value_ignores_a_non_positive_step() {
        assert_eq!(snap_value(17.3, 0.0), 17.3);
        assert_eq!(snap_value(17.3, -5.0), 17.3);
    }

    #[test]
    fn multi_selection_envelope_covers_rotated_elements() {
        let rotated = Element::Rectangle(Rectangle {
            data: ElementData {
                world_point: Point::new(0.0, 0.0),
                width: 100.0,
                height: 100.0,
                rotation: std::f64::consts::FRAC_PI_4,
                ..ElementData::new(1)
            },
        });
        let straight = Element::Rectangle(Rectangle {
            data: ElementData {
                world_point: Point::new(200.0, 0.0),
                width: 100.0,
                height: 100.0,
                ..ElementData::new(2)
            },
        });

        let bounds = selection_bounds(&[1, 2], &[rotated, straight]).unwrap();
        let reach = 50.0 * 2.0f64.sqrt();
        assert!((bounds.x - (50.0 - reach)).abs() < 1e-9);
        assert!((bounds.y - (50.0 - reach)).abs() < 1e-9);
        assert!((bounds.x + bounds.w - 300.0).abs() < 1e-9);
        assert!((bounds.y + bounds.h - (50.0 + reach)).abs() < 1e-9);
    }
}
