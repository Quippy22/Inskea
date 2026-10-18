//! Smart-guide tests, one file per behaviour. This one carries the fixtures
//! every suite builds on.

use super::guides::*;
use super::*;
use crate::model::elements::ellipse::Ellipse;
use crate::model::elements::rect::Rectangle;
use crate::model::elements::ElementData;
use crate::model::interaction::{guide_bounds, guide_frame, FrameShape, GuideFrame};
use crate::model::{Element, ElementId, Point, Rect};

mod alignment;
mod fuzz;
mod runs;
mod scan;
mod spacing;

fn rect(id: ElementId, x: f64, y: f64, w: f64, h: f64) -> GuideBox {
    let bounds = Rect::new(x, y, w, h);
    GuideBox {
        id,
        bounds,
        frame: GuideFrame::rect(bounds),
    }
}

/// A target whose frame is its own envelope (an element that does not rotate).
fn guide(id: ElementId, bounds: Rect) -> GuideBox {
    rect(id, bounds.x, bounds.y, bounds.w, bounds.h)
}

fn placed_data(id: ElementId, x: f64, y: f64, w: f64, h: f64, rotation: f64) -> ElementData {
    let mut data = ElementData::new(id);
    data.world_point = Point::new(x, y);
    data.width = w;
    data.height = h;
    data.rotation = rotation;
    data
}

/// A target taken from a real element, so its frame comes from the same
/// helpers the canvas uses.
fn shaped(id: ElementId, element: Element) -> GuideBox {
    GuideBox {
        id,
        bounds: guide_bounds(&element),
        frame: guide_frame(&element),
    }
}

fn element(id: ElementId, x: f64, y: f64, w: f64, h: f64, rotation: f64) -> GuideBox {
    shaped(
        id,
        Element::Rectangle(Rectangle {
            data: placed_data(id, x, y, w, h, rotation),
        }),
    )
}

/// The same, for the shape whose guides end on its outline.
fn ellipse(id: ElementId, x: f64, y: f64, w: f64, h: f64, rotation: f64) -> GuideBox {
    shaped(
        id,
        Element::Ellipse(Ellipse {
            data: placed_data(id, x, y, w, h, rotation),
        }),
    )
}

/// Cross intervals of the vertical guide segments, in draw order.
fn y_segments(resolution: &GuideResolution) -> Vec<(f64, f64)> {
    resolution
        .state
        .guides
        .iter()
        .map(|line| (line.start.y, line.end.y))
        .collect()
}

/// World rectangle the tests treat as on screen: wide enough that small
/// fixtures are never filtered or trimmed.
fn view() -> Rect {
    Rect::new(-10_000.0, -10_000.0, 20_000.0, 20_000.0)
}

/// Solve as if the moving box were unrotated, so its frame is its envelope.
fn resolve(
    proposed: Rect,
    others: &[GuideBox],
    visible: Rect,
    zoom: f64,
    previous: &SmartGuideState,
) -> GuideResolution {
    resolve_move(
        proposed,
        GuideFrame::rect(proposed),
        others,
        visible,
        zoom,
        previous,
    )
}

fn solve(proposed: Rect, others: &[GuideBox]) -> GuideResolution {
    resolve(proposed, others, view(), 1.0, &SmartGuideState::default())
}
