use crate::model::elements::utils::{resize_box, rotate_bbox};
use crate::model::resize::ResizeContext;
use crate::model::*;
use leptos::IntoView;
use std::rc::Rc;

/// Largest world-space side a freshly imported image is scaled down to.
/// Smaller images keep their natural size.
pub const MAX_IMPORT_SIZE: f64 = 512.0;

/// A raster or vector image referenced by a data URL.
///
/// The image bytes live in the document itself (base64-encoded in `src`), so a
/// `.skea` file is self-contained and does not depend on the original file
/// staying on disk. The URL is shared through an [`Rc`] so undo snapshots and
/// scene clones do not copy the encoded bytes.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Image {
    /// Position, size, rotation, and opacity.
    pub data: ElementData,
    /// Full data URL (`data:image/png;base64,...`) used as the SVG `href`.
    pub src: Rc<str>,
}

impl Image {
    /// Create an image centred on `center`, scaled down to fit
    /// [`MAX_IMPORT_SIZE`] while preserving its aspect ratio.
    pub fn new(src: Rc<str>, natural_width: f64, natural_height: f64, center: Point) -> Self {
        let (width, height) = fit_size(natural_width, natural_height);
        let mut data = ElementData::new(0);
        data.world_point = Point::new(center.x - width / 2.0, center.y - height / 2.0);
        data.width = width;
        data.height = height;
        Self { data, src }
    }
}

/// Fit an image's natural size into [`MAX_IMPORT_SIZE`] without enlarging it.
/// Invalid or missing intrinsic dimensions fall back to a square.
fn fit_size(natural_width: f64, natural_height: f64) -> (f64, f64) {
    let valid = |v: f64| v.is_finite() && v > 0.0;
    if !valid(natural_width) || !valid(natural_height) {
        return (MAX_IMPORT_SIZE, MAX_IMPORT_SIZE);
    }
    let scale = (MAX_IMPORT_SIZE / natural_width.max(natural_height)).min(1.0);
    (
        (natural_width * scale).max(1.0),
        (natural_height * scale).max(1.0),
    )
}

impl Render for Image {
    fn render(&self, _zoom: f64) -> leptos::View {
        let x = self.data.world_point.x;
        let y = self.data.world_point.y;
        let w = self.data.width;
        let h = self.data.height;
        let href = Rc::clone(&self.src);
        let opacity = self.data.style.opacity;
        let inner = leptos::view! {
            <image
                x=x
                y=y
                width=w
                height=h
                href=href
                preserveAspectRatio="none"
                opacity=opacity
            />
        }
        .into_view();
        if self.data.rotation == 0.0 {
            inner
        } else {
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            let deg = self.data.rotation.to_degrees();
            leptos::view! {
                <g transform=format!("rotate({} {} {})", deg, cx, cy)>{inner}</g>
            }
            .into_view()
        }
    }
}

impl HitTest for Image {
    fn hit_test(&self, point: Point, margin: f64) -> bool {
        let cx = self.data.world_point.x + self.data.width / 2.0;
        let cy = self.data.world_point.y + self.data.height / 2.0;
        let pt = Point::unrotate(point, cx, cy, self.data.rotation);
        pt.x >= self.data.world_point.x - margin
            && pt.x <= self.data.world_point.x + self.data.width + margin
            && pt.y >= self.data.world_point.y - margin
            && pt.y <= self.data.world_point.y + self.data.height + margin
    }
}

impl Bounds for Image {
    fn bounds(&self) -> Rect {
        Rect::new(
            self.data.world_point.x,
            self.data.world_point.y,
            self.data.width,
            self.data.height,
        )
    }
}

impl Offset for Image {
    fn offset(&mut self, dx: f64, dy: f64) {
        self.data.world_point.offset(dx, dy);
    }
}

impl Rotate for Image {
    fn rotate_around(&mut self, point: Point, delta: f64) {
        rotate_bbox(&mut self.data, point, delta);
    }
}

impl Resize for Image {
    fn resize(&mut self, ctx: &ResizeContext) {
        resize_box(&mut self.data, ctx);
    }
}

impl UpdateDrag for Image {
    fn update_drag(&mut self, _current: Point, _anchor: Point, _shift: bool, _zoom: f64) {
        // Images are imported, not dragged to size.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_fits_large_images_and_keeps_small_ones() {
        assert_eq!(fit_size(2048.0, 1024.0), (512.0, 256.0));
        assert_eq!(fit_size(100.0, 50.0), (100.0, 50.0));
        assert_eq!(fit_size(0.0, 0.0), (MAX_IMPORT_SIZE, MAX_IMPORT_SIZE));
    }

    #[test]
    fn new_image_is_centred_and_keeps_the_aspect_ratio() {
        let image = Image::new(
            "data:image/png;base64,AA".into(),
            200.0,
            100.0,
            Point::new(50.0, 60.0),
        );
        assert_eq!(image.data.width, 200.0);
        assert_eq!(image.data.height, 100.0);
        assert_eq!(image.data.world_point, Point::new(-50.0, 10.0));
    }

    #[test]
    fn hit_test_follows_the_rotation() {
        let mut image = Image::new(
            "data:image/png;base64,AA".into(),
            100.0,
            100.0,
            Point::new(50.0, 50.0),
        );
        assert_eq!(image.data.world_point, Point::new(0.0, 0.0));
        image.data.rotation = std::f64::consts::FRAC_PI_2;
        // Inside the rotated image, not outside it.
        assert!(image.hit_test(Point::new(50.0, 10.0), 0.0));
        assert!(!image.hit_test(Point::new(-40.0, 50.0), 0.0));
    }

    #[test]
    fn resize_reuses_the_box_handles() {
        let mut image = Image::new(
            "data:image/png;base64,AA".into(),
            100.0,
            50.0,
            Point::new(50.0, 25.0),
        );
        let orig = Element::Image(image.clone());
        image.resize(&ResizeContext {
            orig: &orig,
            handle: crate::model::resize::ResizeHandle::Se,
            pointer_world: Point::new(200.0, 100.0),
            start_world: Point::new(100.0, 50.0),
            shift: false,
            alt: false,
            multi: false,
            bx: 0.0,
            by: 0.0,
            bw: 100.0,
            bh: 50.0,
        });
        assert_eq!((image.data.width, image.data.height), (200.0, 100.0));
    }
}
