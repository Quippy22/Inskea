use crate::canvas::combined_bounds;
use crate::model::resize::{common_bounds, rotate_point_around};
use crate::model::{Bounds, Element, ElementData, ElementId, Point};

pub const GRID_SIZE: f64 = 40.0;
pub const HANDLE_RESIZE_RADIUS: f64 = 5.0;
pub const HANDLE_MOVE_RADIUS: f64 = 6.0;
pub const HANDLE_ROTATE_RADIUS: f64 = 7.0;
pub const ROTATE_HANDLE_OFFSET: f64 = 25.0;
pub const PATH_MERGE_DIST: f64 = 3.0;
pub const SELECT_PADDING: f64 = 5.0;

pub fn selection_bounds(ids: &[ElementId], els: &[Element]) -> Option<(f64, f64, f64, f64)> {
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
                    let (bx, by, bw, bh) = el.bounds();
                    let mut d = el.data().clone();
                    d.world_point = Point { x: bx, y: by };
                    d.width = bw;
                    d.height = bh;
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

pub fn handle_positions(bx: f64, by: f64, bw: f64, bh: f64) -> [(f64, f64); 10] {
    [
        (bx, by),
        (bx + bw / 2.0, by),
        (bx + bw, by),
        (bx, by + bh / 2.0),
        (bx + bw, by + bh / 2.0),
        (bx, by + bh),
        (bx + bw / 2.0, by + bh),
        (bx + bw, by + bh),
        (bx + bw / 2.0, by + bh / 2.0),
        (bx + bw / 2.0, by - 25.0),
    ]
}

pub fn unrotate_for_element(point: (f64, f64), el: &Element) -> (f64, f64) {
    let rot = el.data().rotation;
    if rot == 0.0 {
        return point;
    }
    let (bx, by, bw, bh) = el.bounds();
    let cx = bx + bw / 2.0;
    let cy = by + bh / 2.0;
    rotate_point_around(point, (cx, cy), -rot)
}
