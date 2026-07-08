use leptos::IntoView;
use super::ElementData;
use super::{Bounds, FromDrag, HitTest, Offset, Render, Resize, ResizeContext, Rotate, SnapToGrid, UpdateDrag};

use super::rect::{self, MIN_ELEMENT_SIZE};

/// An ellipse (oval) shape defined by its bounding-box top-left, width, and height.
#[derive(Clone, Debug)]
pub struct Ellipse {
    /// Position, size, and appearance (stroke/fill/rotation).
    pub data: ElementData,
}

impl Ellipse {
    fn fill_paint(fill: &Option<super::ShapeColor>) -> String {
        match fill {
            Some(c) => c.to_hex().to_string(),
            None => "none".to_string(),
        }
    }
}

impl FromDrag for Ellipse {
    fn from_drag(
        anchor: (f64, f64),
        current: (f64, f64),
        color: super::ShapeColor,
        shift: bool,
    ) -> Self {
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
        if w < rect::MIN_DIMENSION {
            w = rect::MIN_DIMENSION;
        }
        if h < rect::MIN_DIMENSION {
            h = rect::MIN_DIMENSION;
        }
        Self {
            data: ElementData {
                x,
                y,
                width: w,
                height: h,
                stroke_color: color,
                ..ElementData::new(0)
            },
        }
    }
}

impl UpdateDrag for Ellipse {
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
        if w < rect::MIN_DIMENSION {
            w = rect::MIN_DIMENSION;
        }
        if h < rect::MIN_DIMENSION {
            h = rect::MIN_DIMENSION;
        }
        self.data.x = x;
        self.data.y = y;
        self.data.width = w;
        self.data.height = h;
    }
}

impl Render for Ellipse {
    fn render(&self, _zoom: f64) -> leptos::View {
        let cx = self.data.x + self.data.width / 2.0;
        let cy = self.data.y + self.data.height / 2.0;
        let rx = self.data.width / 2.0;
        let ry = self.data.height / 2.0;
        let sw = self.data.stroke_width;
        let fill = Self::fill_paint(&self.data.fill_color);
        let stroke = super::ShapeColor::to_hex(self.data.stroke_color);
        if self.data.rotation == 0.0 {
            leptos::view! {
                <ellipse cx=cx cy=cy rx=rx ry=ry fill=fill stroke=stroke stroke-width=sw />
            }
            .into_view()
        } else {
            let deg = self.data.rotation.to_degrees();
            leptos::view! {
                <g transform={format!("rotate({} {} {})", deg, cx, cy)}>
                    <ellipse cx=cx cy=cy rx=rx ry=ry fill=fill stroke=stroke stroke-width=sw />
                </g>
            }
            .into_view()
        }
    }
}

impl HitTest for Ellipse {
    fn hit_test(&self, point: (f64, f64), margin: f64) -> bool {
        let (px, py) = point;
        let has_fill = self.data.fill_color.is_some();
        if has_fill {
            px >= self.data.x - margin
                && px <= self.data.x + self.data.width + margin
                && py >= self.data.y - margin
                && py <= self.data.y + self.data.height + margin
        } else {
            let cx = self.data.x + self.data.width / 2.0;
            let cy = self.data.y + self.data.height / 2.0;
            let hw = self.data.width / 2.0;
            let hh = self.data.height / 2.0;
            let dx = (px - cx) / hw.max(1.0);
            let dy = (py - cy) / hh.max(1.0);
            let dist = (dx * dx + dy * dy).sqrt();
            let edge_dist = (dist - 1.0).abs() * hw.min(hh).max(1.0);
            edge_dist <= margin + self.data.stroke_width
        }
    }
}

impl Bounds for Ellipse {
    fn bounds(&self) -> (f64, f64, f64, f64) {
        (self.data.x, self.data.y, self.data.width, self.data.height)
    }
}

impl Offset for Ellipse {
    fn offset(&mut self, dx: f64, dy: f64) {
        self.data.x += dx;
        self.data.y += dy;
    }
}

impl SnapToGrid for Ellipse {
    fn snap_to_grid(&mut self, grid: f64) {
        let cx = self.data.x + self.data.width / 2.0;
        let cy = self.data.y + self.data.height / 2.0;
        let snapped_cx = (cx / grid).round() * grid;
        let snapped_cy = (cy / grid).round() * grid;
        self.data.x += snapped_cx - cx;
        self.data.y += snapped_cy - cy;
    }
}

impl Rotate for Ellipse {
    fn rotate_around(&mut self, _cx: f64, _cy: f64, delta: f64) {
        self.data.rotation += delta;
    }
}

impl Resize for Ellipse {
    fn resize(&mut self, ctx: &ResizeContext) {
        let rctx = ctx;
        let (mut nx, mut ny, mut nw, mut nh) = match rctx.handle {
            0 => (rctx.bx + rctx.dx, rctx.by + rctx.dy, rctx.bw - rctx.dx, rctx.bh - rctx.dy),
            1 => (rctx.bx, rctx.by + rctx.dy, rctx.bw, rctx.bh - rctx.dy),
            2 => (rctx.bx, rctx.by + rctx.dy, rctx.bw + rctx.dx, rctx.bh - rctx.dy),
            3 => (rctx.bx + rctx.dx, rctx.by, rctx.bw - rctx.dx, rctx.bh),
            4 => (rctx.bx, rctx.by, rctx.bw + rctx.dx, rctx.bh),
            5 => (rctx.bx + rctx.dx, rctx.by, rctx.bw - rctx.dx, rctx.bh + rctx.dy),
            6 => (rctx.bx, rctx.by, rctx.bw, rctx.bh + rctx.dy),
            7 => (rctx.bx, rctx.by, rctx.bw + rctx.dx, rctx.bh + rctx.dy),
            _ => return,
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
                0 => { nx = rctx.bx + rctx.bw - nw; ny = rctx.by + rctx.bh - nh; }
                1 => { ny = rctx.by + rctx.bh - nh; }
                2 => { ny = rctx.by + rctx.bh - nh; }
                3 => { nx = rctx.bx + rctx.bw - nw; }
                5 => { nx = rctx.bx + rctx.bw - nw; }
                _ => {}
            }
        }
        if nw < MIN_ELEMENT_SIZE || nh < MIN_ELEMENT_SIZE {
            return;
        }
        if rctx.multi {
            if let super::Element::Ellipse(orig) = rctx.orig {
                let obw = rctx.bw.max(MIN_ELEMENT_SIZE);
                let obh = rctx.bh.max(MIN_ELEMENT_SIZE);
                let sx = nw / obw;
                let sy = nh / obh;
                self.data.x = (orig.data.x - rctx.bx) * sx + nx;
                self.data.y = (orig.data.y - rctx.by) * sy + ny;
                self.data.width = (orig.data.width * sx).max(MIN_ELEMENT_SIZE);
                self.data.height = (orig.data.height * sy).max(MIN_ELEMENT_SIZE);
            }
        } else {
            self.data.x = nx;
            self.data.y = ny;
            self.data.width = nw;
            self.data.height = nh;
        }
    }
}
