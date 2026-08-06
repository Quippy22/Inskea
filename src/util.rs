use crate::model::Point;

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
