use super::{
    Bounds, FromDrag, HitTest, Offset, Render, Resize, Rotate, SnapToGrid, UpdateDrag,
};
use super::{ElementData, ShapeColor};
use super::utils::{rect_from_drag, rotate_bbox, snap_bbox_to_grid};
use crate::model::resize::{resize_bbox, resize_from_handle, ResizeContext};
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
    fn from_drag(anchor: Point, current: Point, color: ShapeColor, shift: bool) -> Self {
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

impl UpdateDrag for Rectangle {
    fn update_drag(&mut self, current: Point, anchor: Point, shift: bool) {
        let (pt, w, h) = rect_from_drag(anchor, current, shift);
        self.data.world_point.set(pt.x, pt.y);
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
        snap_bbox_to_grid(&mut self.data.world_point, self.data.width, self.data.height, grid);
    }
}

impl Rotate for Rectangle {
    fn rotate_around(&mut self, point: Point, delta: f64) {
        rotate_bbox(&mut self.data, point, delta);
    }
}

impl Resize for Rectangle {
    fn resize(&mut self, ctx: &ResizeContext) {
        if ctx.multi {
            use crate::model::resize::MIN_ELEMENT_SIZE;
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
            if let super::Element::Rectangle(orig) = ctx.orig {
                let obw = ctx.bw.max(MIN_ELEMENT_SIZE);
                let obh = ctx.bh.max(MIN_ELEMENT_SIZE);
                let sx = nw / obw;
                let sy = nh / obh;
                self.data.world_point.set(
                    (orig.data.world_point.x - ctx.bx) * sx + pos.x,
                    (orig.data.world_point.y - ctx.by) * sy + pos.y,
                );
                self.data.width = (orig.data.width * sx).max(MIN_ELEMENT_SIZE);
                self.data.height = (orig.data.height * sy).max(MIN_ELEMENT_SIZE);
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
