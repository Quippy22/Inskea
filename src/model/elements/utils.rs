use crate::model::Point;

use super::rect::MIN_DIMENSION;

pub(crate) fn rect_from_drag(
    anchor: (f64, f64),
    current: (f64, f64),
    shift: bool,
) -> (Point, f64, f64) {
    let (ax, ay) = anchor;
    let (cx, cy) = current;
    let mut x = ax.min(cx);
    let mut y = ay.min(cy);
    let mut w = (cx - ax).abs();
    let mut h = (cy - ay).abs();
    if shift {
        let s = w.max(h);
        w = s;
        h = s;
        if cx < ax {
            x = ax - s;
        }
        if cy < ay {
            y = ay - s;
        }
    }
    if w < MIN_DIMENSION {
        w = MIN_DIMENSION;
    }
    if h < MIN_DIMENSION {
        h = MIN_DIMENSION;
    }
    (Point { x, y }, w, h)
}
