//! Smart guides for move drags.
//!
//! Resolution is a pure function of the proposed rectangle, the neighbouring
//! element bounds, the visible world rectangle, and the zoom: [`resolve_move`]
//! returns the snap delta plus the guides to draw. Targets are the elements
//! the viewport shows. Tolerances are specified in screen pixels and divided
//! by the zoom, so snapping feels the same at every zoom level. The previous
//! state is threaded back in for hysteresis: an active guide stays latched
//! until the pointer leaves a slightly wider release band.

mod candidates;
mod geometry;
mod guides;
mod overlay;
mod solver;
#[cfg(test)]
mod tests;

pub use overlay::guide_overlay;
pub use solver::resolve_move;

use crate::model::{ElementId, Point, Rect};

/// Distance at which a proposed position acquires a guide, in screen pixels.
pub const GUIDE_ACQUIRE_PX: f64 = 6.0;
/// Distance at which an already active guide releases, in screen pixels.
pub const GUIDE_RELEASE_PX: f64 = 10.0;
/// Perpendicular distance at which two boxes count as sharing a lane,
/// in screen pixels.
pub const GUIDE_LANE_PX: f64 = 12.0;
/// Objects further away than this from the moving box, in screen pixels, no
/// longer take part in the spacing scans. Alignment still reads everything on
/// screen: the spacing scans are the quadratic ones, so they get the bound.
pub const GUIDE_PAIR_REACH_PX: f64 = 600.0;
/// Upper bound on the boxes the spacing scans pair up, nearest first. Only a
/// pathological scene (thousands of boxes in one lane, zoomed out) reaches it.
const GUIDE_MAX_PAIR_TARGETS: usize = 96;
/// Distance from the boxes to a spacing marker's centre, in screen pixels:
/// the marker sits outside the shared band, clear of the feature lines an
/// alignment guide rides on.
pub const GUIDE_SPACING_OFFSET_PX: f64 = 9.0;
/// Height of the spacing marker band, in screen pixels.
pub const GUIDE_SPACING_BAND_PX: f64 = 8.0;
/// Length of the spacing marker's end ticks, in screen pixels.
pub const GUIDE_SPACING_CAP_PX: f64 = 15.0;
/// Gap between an end tick's far tip and the edge it reaches towards, in
/// screen pixels.
pub const GUIDE_SPACING_CLEARANCE_PX: f64 = 1.5;
/// Clearance between a spacing band and its measurement label, in screen px.
pub const GUIDE_LABEL_CLEARANCE_PX: f64 = 7.0;
/// Font size of a measurement label, in screen pixels.
pub const GUIDE_LABEL_FONT_PX: f64 = 11.0;
/// Bands shorter than this on screen get no label: there is no room to read
/// one without it spilling over the pieces the band measures.
pub const GUIDE_MIN_LABEL_SPAN_PX: f64 = 34.0;
const GUIDE_ALIGNMENT_COLOR: &str = "#7aa2f7";
const GUIDE_SPACING_COLOR: &str = "#9ece6a";
const EPSILON: f64 = 1e-9;
const FEATURES: [GuideFeature; 3] = [GuideFeature::Start, GuideFeature::Center, GuideFeature::End];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuideAxis {
    /// A vertical line, matching an x-coordinate.
    Vertical,
    /// A horizontal line, matching a y-coordinate.
    Horizontal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuideKind {
    Alignment,
    EqualSpacing,
    /// A gap the moving box closed against the end of an evenly spaced chain.
    ChainSpacing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GuideFeature {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuideKey {
    pub axis: GuideAxis,
    pub kind: GuideKind,
    pub moving_feature: GuideFeature,
    pub target_feature: GuideFeature,
    pub target_a: ElementId,
    pub target_b: ElementId,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuideLine {
    pub start: Point,
    pub end: Point,
    /// Cross coordinate each end tick may reach towards: the near edge of the
    /// piece beside that end, so ticks stay attached to pieces of different
    /// sizes. Alignment guides carry their own coordinate.
    pub stops: [f64; 2],
    pub kind: GuideKind,
}

/// Distance readout beside a spacing band: the free space the band covers, in
/// world units, drawn as a number of a fixed screen size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuideLabel {
    /// Point on the band the label hangs from: the band's midpoint along the
    /// axis. The renderer offsets it across the band, away from the pieces.
    pub at: Point,
    /// Distance between the two edges the band spans, in world units.
    pub distance: f64,
    /// Axis of the band's guide. The band runs across it and the label is
    /// offset along it, so the two never overlap.
    pub axis: GuideAxis,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SmartGuideState {
    pub guides: Vec<GuideLine>,
    pub labels: Vec<GuideLabel>,
    pub active: Vec<GuideKey>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GuideResolution {
    pub snap_dx: f64,
    pub snap_dy: f64,
    pub state: SmartGuideState,
}

#[derive(Clone, Copy)]
struct GuideBox {
    id: ElementId,
    bounds: Rect,
}

#[derive(Clone, Copy)]
struct GuideCandidate {
    key: GuideKey,
    snap_delta: f64,
}

/// Screen-pixel tolerances converted to world units for one zoom level.
#[derive(Clone, Copy)]
struct Tolerances {
    acquire: f64,
    release: f64,
    lane: f64,
    reach: f64,
    marker: f64,
}

impl Tolerances {
    fn for_zoom(zoom: f64) -> Self {
        let zoom = if zoom.is_finite() && zoom > 0.0 {
            zoom
        } else {
            1.0
        };
        Self {
            acquire: GUIDE_ACQUIRE_PX / zoom,
            release: GUIDE_RELEASE_PX / zoom,
            lane: GUIDE_LANE_PX / zoom,
            reach: GUIDE_PAIR_REACH_PX / zoom,
            marker: GUIDE_SPACING_OFFSET_PX / zoom,
        }
    }
}
