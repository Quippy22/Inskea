use super::ElementData;
use super::{
    Bounds, FromDrag, HitTest, Offset, Render, Resize, Rotate, SnapToGrid, UpdateDrag,
};
use super::utils::rect_from_drag;
use crate::model::resize::{resize_bbox, resize_from_handle, ResizeContext};
use crate::model::Point;
use leptos::IntoView;

/// An ellipse (oval) shape defined by its bounding-box top-left, width, and height.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
        let (pt, w, h) = rect_from_drag(anchor, current, shift);
        Self {
            data: ElementData {
                world_point: pt,
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
        let (pt, w, h) = rect_from_drag(anchor, current, shift);
        self.data.world_point.set(pt.x, pt.y);
        self.data.width = w;
        self.data.height = h;
    }
}

impl Render for Ellipse {
    fn render(&self, _zoom: f64) -> leptos::View {
        let cx = self.data.world_point.x + self.data.width / 2.0;
        let cy = self.data.world_point.y + self.data.height / 2.0;
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
            let cx = self.data.world_point.x + self.data.width / 2.0;
            let cy = self.data.world_point.y + self.data.height / 2.0;
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
        (self.data.world_point.x, self.data.world_point.y, self.data.width, self.data.height)
    }
}

impl Offset for Ellipse {
    fn offset(&mut self, dx: f64, dy: f64) {
        self.data.world_point.offset(dx, dy);
    }
}

impl SnapToGrid for Ellipse {
    fn snap_to_grid(&mut self, grid: f64) {
        let cx = self.data.world_point.x + self.data.width / 2.0;
        let cy = self.data.world_point.y + self.data.height / 2.0;
        let snapped_cx = (cx / grid).round() * grid;
        let snapped_cy = (cy / grid).round() * grid;
        self.data.world_point.x += snapped_cx - cx;
        self.data.world_point.y += snapped_cy - cy;
    }
}

impl Rotate for Ellipse {
    fn rotate_around(&mut self, point: Point, delta: f64) {
        self.data.rotation += delta;
        let cx = self.data.world_point.x + self.data.width / 2.0;
        let cy = self.data.world_point.y + self.data.height / 2.0;
        let mut center = Point { x: cx, y: cy };
        center.rotate_around(point, delta);
        self.data.world_point.x = center.x - self.data.width / 2.0;
        self.data.world_point.y = center.y - self.data.height / 2.0;
    }
}

impl Resize for Ellipse {
    fn resize(&mut self, ctx: &ResizeContext) {
        if ctx.multi {
            let (pos, (nw, nh)) = match resize_bbox(
                Point { x: ctx.bx, y: ctx.by },
                (ctx.bw, ctx.bh),
                ctx.pointer_world,
                ctx.handle,
                ctx.shift,
                ctx.alt,
            ) {
                Some(v) => v,
                None => return,
            };
            if let super::Element::Ellipse(orig) = ctx.orig {
                let obw = ctx.bw.max(crate::model::resize::MIN_ELEMENT_SIZE);
                let obh = ctx.bh.max(crate::model::resize::MIN_ELEMENT_SIZE);
                let sx = nw / obw;
                let sy = nh / obh;
                self.data.world_point.set(
                    (orig.data.world_point.x - ctx.bx) * sx + pos.x,
                    (orig.data.world_point.y - ctx.by) * sy + pos.y,
                );
                self.data.width = (orig.data.width * sx).max(crate::model::resize::MIN_ELEMENT_SIZE);
                self.data.height = (orig.data.height * sy).max(crate::model::resize::MIN_ELEMENT_SIZE);
            }
        } else {
            let result = resize_from_handle(
                &self.data,
                ctx.handle,
                ctx.pointer_world,
                ctx.shift,
                ctx.alt,
            );
            self.data = result;
        }
    }
}
