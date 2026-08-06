use crate::canvas::combined_bounds;
use crate::model::resize::common_bounds;
use crate::model::{Bounds, Element, ElementData, ElementId, Point, Rect};

pub const GRID_SIZE: f64 = 40.0;
pub const HANDLE_RESIZE_RADIUS: f64 = 5.0;
pub const HANDLE_MOVE_RADIUS: f64 = 6.0;
pub const HANDLE_ROTATE_RADIUS: f64 = 7.0;
pub const ROTATE_HANDLE_OFFSET: f64 = 25.0;
pub const PATH_MERGE_DIST: f64 = 3.0;
pub const SELECT_PADDING: f64 = 5.0;

pub fn selection_bounds(ids: &[ElementId], els: &[Element]) -> Option<Rect> {
    if ids.len() == 1 {
        els.iter()
            .find(|e| e.id() == ids[0])
            .map(|el| el.bounds())
            .or_else(|| combined_bounds(ids, els))
    } else {
        let data_refs: Vec<ElementData> = els
            .iter()
            .filter(|el| ids.contains(&el.id()))
            .map(|el| {
                if matches!(el, Element::Line(_) | Element::Freehand(_)) {
                    let b = el.bounds();
                    let mut d = el.data().clone();
                    d.world_point = Point::new(b.x, b.y);
                    d.width = b.w;
                    d.height = b.h;
                    d.rotation = 0.0;
                    d
                } else {
                    el.data().clone()
                }
            })
            .collect();
        if data_refs.is_empty() {
            None
        } else {
            let refs: Vec<&ElementData> = data_refs.iter().collect();
            Some(common_bounds(&refs))
        }
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
    let b = el.bounds();
    let cx = b.x + b.w / 2.0;
    let cy = b.y + b.h / 2.0;
    Point::unrotate(point, cx, cy, -rot)
}
