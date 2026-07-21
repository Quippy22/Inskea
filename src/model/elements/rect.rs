use crate::model::elements::utils::{rect_from_drag, rotate_bbox, snap_bbox_to_grid};
use crate::model::resize::{resize_bbox, resize_from_handle, resize_scale_element, ResizeContext};
use crate::model::*;
use leptos::IntoView;

pub(crate) const MIN_DIMENSION: f64 = 1.0;

/// A rectangle shape defined by its top-left corner, width, and height.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Rectangle {
    /// Position, size, and appearance (stroke/fill/rotation).
    pub data: ElementData,
}

impl Rectangle {
    pub(crate) fn fill_paint(fill: &Option<Color>) -> String {
        match fill {
            Some(c) => c.to_hex().to_string(),
            None => "none".to_string(),
        }
    }

    pub(crate) fn stroke_hex(stroke: &Color) -> String {
        stroke.to_hex()
    }
}

impl FromDrag for Rectangle {
    fn from_drag(anchor: Point, current: Point, color: Color, shift: bool) -> Self {
        let (pt, w, h) = rect_from_drag(anchor, current, shift);
        Self {
            data: ElementData {
                world_point: pt,
                width: w,
                height: h,
                style: super::ElementStyle {
                    stroke_color: color,
                    ..Default::default()
                },
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
        let sw = self.data.style.stroke_width;
        let fill = Self::fill_paint(&self.data.style.fill_color);
        let stroke = Self::stroke_hex(&self.data.style.stroke_color);
        let dash = self.data.style.stroke_style.stroke_dasharray();
        let linejoin = self.data.style.edge_style.stroke_linejoin();
        let (rx, ry) = match self.data.style.edge_style {
            super::EdgeStyle::Rounded => (self.data.style.roundness, self.data.style.roundness),
            super::EdgeStyle::Sharp => (0.0, 0.0),
        };
        let opacity = self.data.style.opacity;
        if self.data.rotation == 0.0 {
            leptos::view! {
                <rect
                    x=x
                    y=y
                    width=w
                    height=h
                    fill=fill
                    stroke=stroke
                    stroke-width=sw
                    stroke-dasharray=dash
                    stroke-linejoin=linejoin
                    rx=rx
                    ry=ry
                    opacity=opacity
                />
            }
            .into_view()
        } else {
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            let deg = self.data.rotation.to_degrees();
            leptos::view! {
                <g transform=format!("rotate({} {} {})", deg, cx, cy)>
                    <rect
                        x=x
                        y=y
                        width=w
                        height=h
                        fill=fill
                        stroke=stroke
                        stroke-width=sw
                        stroke-dasharray=dash
                        stroke-linejoin=linejoin
                        rx=rx
                        ry=ry
                        opacity=opacity
                    />
                </g>
            }
            .into_view()
        }
    }
}

impl HitTest for Rectangle {
    fn hit_test(&self, point: Point, margin: f64) -> bool {
        let cx = self.data.world_point.x + self.data.width / 2.0;
        let cy = self.data.world_point.y + self.data.height / 2.0;
        let pt = Point::unrotate(point, cx, cy, self.data.rotation);
        let px = pt.x;
        let py = pt.y;
        let has_fill = self.data.style.fill_color.is_some();
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
            near_edge <= margin + self.data.style.stroke_width
                && px >= self.data.world_point.x - margin
                && px <= self.data.world_point.x + self.data.width + margin
                && py >= self.data.world_point.y - margin
                && py <= self.data.world_point.y + self.data.height + margin
        }
    }
}

impl Bounds for Rectangle {
    fn bounds(&self) -> (f64, f64, f64, f64) {
        (
            self.data.world_point.x,
            self.data.world_point.y,
            self.data.width,
            self.data.height,
        )
    }
}

impl Offset for Rectangle {
    fn offset(&mut self, dx: f64, dy: f64) {
        self.data.world_point.offset(dx, dy);
    }
}

impl SnapToGrid for Rectangle {
    fn snap_to_grid(&mut self, grid: f64) {
        snap_bbox_to_grid(
            &mut self.data.world_point,
            self.data.width,
            self.data.height,
            grid,
        );
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
            let (pos, (nw, nh)) = match resize_bbox(
                Point {
                    x: ctx.bx,
                    y: ctx.by,
                },
                (ctx.bw, ctx.bh),
                ctx.pointer_world,
                ctx.handle,
                ctx.shift,
                ctx.alt,
            ) {
                Some(v) => v,
                None => return,
            };
            resize_scale_element(
                &mut self.data,
                ctx.orig.data(),
                pos,
                nw,
                nh,
                ctx.bx,
                ctx.by,
                ctx.bw,
                ctx.bh,
                true,
            );
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
