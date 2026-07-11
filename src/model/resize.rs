use super::elements::Element;

/// Minimum allowed size for any dimension after a resize.
pub const MIN_ELEMENT_SIZE: f64 = 5.0;

/// Identifies which handle on the bounding box is being dragged.
///
/// Corners resize along two axes; edge midpoints resize along one axis.
/// The labels Nw/Ne/Sw/Se and N/E/S/W refer to the *axis-aligned bounding
/// box* slots, not the shape's visual orientation — after rotation the
/// "N" handle may appear anywhere.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ResizeHandle {
    /// Top-left corner.
    Nw,
    /// Top edge midpoint.
    N,
    /// Top-right corner.
    Ne,
    /// Left edge midpoint.
    W,
    /// Right edge midpoint.
    E,
    /// Bottom-left corner.
    Sw,
    /// Bottom edge midpoint.
    S,
    /// Bottom-right corner.
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

/// Parameters for resizing an element via drag handles.
#[derive(Clone, Copy)]
pub struct ResizeContext<'a> {
    /// Original (pre-drag) element data — used to prevent scale compounding.
    pub orig: &'a Element,
    /// Bounding box at drag start.
    pub bx: f64,
    pub by: f64,
    pub bw: f64,
    pub bh: f64,
    /// Total mouse delta from drag start.
    pub dx: f64,
    pub dy: f64,
    /// Which handle is being dragged.
    pub handle: ResizeHandle,
    /// Whether Shift is held (preserve aspect ratio).
    pub shift: bool,
    /// Whether multiple elements are being resized together.
    pub multi: bool,
}

/// Compute the new bounding-box position and size for a resize drag.
///
/// Returns `Some((nx, ny, nw, nh))` on success, or `None` if the result
/// would be smaller than [`MIN_ELEMENT_SIZE`].
pub fn resize_bbox(
    bx: f64,
    by: f64,
    bw: f64,
    bh: f64,
    dx: f64,
    dy: f64,
    handle: ResizeHandle,
) -> Option<(f64, f64, f64, f64)> {
    let (nx, ny, nw, nh) = match handle {
        ResizeHandle::Nw => (bx + dx, by + dy, bw - dx, bh - dy),
        ResizeHandle::N => (bx, by + dy, bw, bh - dy),
        ResizeHandle::Ne => (bx, by + dy, bw + dx, bh - dy),
        ResizeHandle::W => (bx + dx, by, bw - dx, bh),
        ResizeHandle::E => (bx, by, bw + dx, bh),
        ResizeHandle::Sw => (bx + dx, by, bw - dx, bh + dy),
        ResizeHandle::S => (bx, by, bw, bh + dy),
        ResizeHandle::Se => (bx, by, bw + dx, bh + dy),
    };
    if nw < MIN_ELEMENT_SIZE || nh < MIN_ELEMENT_SIZE {
        return None;
    }
    Some((nx, ny, nw, nh))
}
