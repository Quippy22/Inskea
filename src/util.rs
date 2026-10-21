use crate::model::Point;

mod expandable_array;
pub use expandable_array::ExpandableArray;

pub fn window_size() -> Point {
    let window = web_sys::window().expect("no global `window` exists");
    let w = window
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let h = window
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    Point::new(w, h)
}

/// Round `value` to the nearest multiple of `step`; a non-positive step leaves
/// the value untouched.
pub fn snap_value(value: f64, step: f64) -> f64 {
    if step <= 0.0 {
        value
    } else {
        (value / step).round() * step
    }
}
