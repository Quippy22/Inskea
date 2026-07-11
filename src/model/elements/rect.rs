use super::{
    Bounds, FromDrag, HitTest, Offset, Render, Resize, Rotate, SnapToGrid, UpdateDrag,
};
use super::{ElementData, ShapeColor};
use crate::model::resize::{resize_bbox, ResizeContext, ResizeHandle};
use crate::model::Point;
use leptos::IntoView;

pub(crate) const MIN_DIMENSION: f64 = 1.0;

/// A rectangle shape defined by its top-left corner, width, and height.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Rectangle {
    /// Position, size, and appearance (stroke/fill/rotation).
    pub data: ElementData,
}

impl Rectangle {
    pub(crate) fn fill_paint(fill: &Option<ShapeColor>) -> String {
        match fill {
            Some(c) => c.to_hex().to_string(),
            None => "none".to_string(),
        }
    }

    pub(crate) fn stroke_hex(stroke: ShapeColor) -> &'static str {
        stroke.to_hex()
    }
}

impl FromDrag for Rectangle {
    fn from_drag(anchor: (f64, f64), current: (f64, f64), color: ShapeColor, shift: bool) -> Self {
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
        Self {
            data: ElementData {
                world_point: Point { x, y },
                width: w,
                height: h,
                stroke_color: color,
                ..ElementData::new(0)
            },
        }
    }
}

impl UpdateDrag for Rectangle {
    fn update_drag(&mut self, current: (f64, f64), anchor: (f64, f64), shift: bool) {
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
        self.data.world_point.x = x;
        self.data.world_point.y = y;
        self.data.width = w;
        self.data.height = h;
    }
}

impl Render for Rectangle {
    fn render(&self, _zoom: f64) -> leptos::View {
        let x = self.data.world_point.x;
        let y = self.data.world_point.y;
        let w = self.data.width;
        let h = self.data.height;
        let sw = self.data.stroke_width;
        let fill = Self::fill_paint(&self.data.fill_color);
        let stroke = Self::stroke_hex(self.data.stroke_color);
        if self.data.rotation == 0.0 {
            leptos::view! {
                <rect x=x y=y width=w height=h fill=fill stroke=stroke stroke-width=sw />
            }
            .into_view()
        } else {
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            let deg = self.data.rotation.to_degrees();
            leptos::view! {
                <g transform={format!("rotate({} {} {})", deg, cx, cy)}>
                    <rect x=x y=y width=w height=h fill=fill stroke=stroke stroke-width=sw />
                </g>
            }
            .into_view()
        }
    }
}

impl HitTest for Rectangle {
    fn hit_test(&self, point: Point, margin: f64) -> bool {
        let (px, py) = if self.data.rotation != 0.0 {
            let cx = self.data.world_point.x + self.data.width / 2.0;
            let cy = self.data.world_point.y + self.data.height / 2.0;
            let cos = (-self.data.rotation).cos();
            let sin = (-self.data.rotation).sin();
            let dx = point.x - cx;
            let dy = point.y - cy;
            (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
        } else {
            (point.x, point.y)
        };
        let has_fill = self.data.fill_color.is_some();
        if has_fill {
            px >= self.data.world_point.x - margin
                && px <= self.data.world_point.x + self.data.width + margin
                && py >= self.data.world_point.y - margin
                && py <= self.data.world_point.y + self.data.height + margin
        } else {
            let dl = (px - self.data.world_point.x).abs();
            let dr = (px - (self.data.world_point.x + self.data.width)).abs();
            let dt = (py - self.data.world_point.y).abs();
            let db = (py - (self.data.world_point.y + self.data.height)).abs();
            let near_edge = dl.min(dr).min(dt).min(db);
            near_edge <= margin + self.data.stroke_width
                && px >= self.data.world_point.x - margin
                && px <= self.data.world_point.x + self.data.width + margin
                && py >= self.data.world_point.y - margin
                && py <= self.data.world_point.y + self.data.height + margin
        }
    }
}

impl Bounds for Rectangle {
    fn bounds(&self) -> (f64, f64, f64, f64) {
        (self.data.world_point.x, self.data.world_point.y, self.data.width, self.data.height)
    }
}

impl Offset for Rectangle {
    fn offset(&mut self, dx: f64, dy: f64) {
        self.data.world_point.offset(dx, dy);
    }
}

impl SnapToGrid for Rectangle {
    fn snap_to_grid(&mut self, grid: f64) {
        let cx = self.data.world_point.x + self.data.width / 2.0;
        let cy = self.data.world_point.y + self.data.height / 2.0;
        let snapped_cx = (cx / grid).round() * grid;
        let snapped_cy = (cy / grid).round() * grid;
        self.data.world_point.x += snapped_cx - cx;
        self.data.world_point.y += snapped_cy - cy;
    }
}

impl Rotate for Rectangle {
    fn rotate_around(&mut self, _point: Point, delta: f64) {
        self.data.rotation += delta;
    }
}

impl Resize for Rectangle {
    fn resize(&mut self, ctx: &ResizeContext) {
        use crate::model::resize::MIN_ELEMENT_SIZE;
        let rctx = ctx;
        let (mut nx, mut ny, mut nw, mut nh) = match resize_bbox(
            rctx.bx, rctx.by, rctx.bw, rctx.bh,
            rctx.dx, rctx.dy,
            rctx.handle,
        ) {
            Some(v) => v,
            None => return,
        };
        if rctx.shift {
            let ratio = rctx.bw / rctx.bh;
            let nratio = nw / nh;
            if nratio > ratio {
                nh = nw / ratio;
            } else {
                nw = nh * ratio;
            }
            match rctx.handle {
                ResizeHandle::Nw => {
                    nx = rctx.bx + rctx.bw - nw;
                    ny = rctx.by + rctx.bh - nh;
                }
                ResizeHandle::N | ResizeHandle::Ne => {
                    ny = rctx.by + rctx.bh - nh;
                }
                ResizeHandle::W | ResizeHandle::Sw => {
                    nx = rctx.bx + rctx.bw - nw;
                }
                _ => {}
            }
        }
        if nw < MIN_ELEMENT_SIZE || nh < MIN_ELEMENT_SIZE {
            return;
        }
        if rctx.multi {
            if let super::Element::Rectangle(orig) = rctx.orig {
                let obw = rctx.bw.max(MIN_ELEMENT_SIZE);
                let obh = rctx.bh.max(MIN_ELEMENT_SIZE);
                let sx = nw / obw;
                let sy = nh / obh;
                self.data.world_point.x = (orig.data.world_point.x - rctx.bx) * sx + nx;
                self.data.world_point.y = (orig.data.world_point.y - rctx.by) * sy + ny;
                self.data.width = (orig.data.width * sx).max(MIN_ELEMENT_SIZE);
                self.data.height = (orig.data.height * sy).max(MIN_ELEMENT_SIZE);
            }
        } else {
            self.data.world_point.x = nx;
            self.data.world_point.y = ny;
            self.data.width = nw;
            self.data.height = nh;
        }
    }
}
