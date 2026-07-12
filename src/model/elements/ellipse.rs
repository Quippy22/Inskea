use super::ElementData;
use super::{
    Bounds, FromDrag, HitTest, Offset, Render, Resize, Rotate, SnapToGrid, UpdateDrag,
};
use super::utils::{rect_from_drag, rotate_bbox, snap_bbox_to_grid};
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
    fn from_drag(anchor: Point, current: Point, color: super::ShapeColor, shift: bool) -> Self {
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
    fn update_drag(&mut self, current: Point, anchor: Point, shift: bool) {
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
        let cx = self.data.world_point.x + self.data.width / 2.0;
        let cy = self.data.world_point.y + self.data.height / 2.0;
        let pt = Point::unrotate(point, cx, cy, self.data.rotation);
        let px = pt.x;
        let py = pt.y;
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
        snap_bbox_to_grid(&mut self.data.world_point, self.data.width, self.data.height, grid);
    }
}

impl Rotate for Ellipse {
    fn rotate_around(&mut self, point: Point, delta: f64) {
        rotate_bbox(&mut self.data, point, delta);
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
