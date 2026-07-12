use crate::model::resize::ResizeHandle;
use crate::model::ShapeColor;
use crate::model::{Bounds, Element, ElementId, HitTest, Point, Scene};
use crate::ui::dock::Tool;
use crate::canvas::settings::CanvasSettings;
use leptos::*;
use std::rc::Rc;

const HIT_MARGIN: f64 = 6.0;
const CLICK_MARGIN: f64 = 12.0;

/// Interaction mode of the canvas.
#[derive(Clone, Copy, PartialEq)]
pub enum CanvasMode {
    /// Select, move, resize, and edit existing elements.
    Select,
    /// Pan the viewport by dragging.
    Hand,
    /// Draw a new element based on the active tool.
    Draw,
}

/// Which handle a drag operation is acting on.
#[derive(Clone, Copy, PartialEq)]
pub enum Handle {
    /// Resize corner or edge handle on the bounding box.
    Resize(ResizeHandle),
    /// Move via the centre crosshair.
    Move,
    /// Rotate via the top handle.
    Rotate,
    /// Dragging an existing path point at this index.
    PathPoint(usize),
    /// Grabbed the ghost handle between points[i] and points[i+1].
    /// Resolved to PathPoint once the drag exceeds MIN_DRAG_DIST.
    PathMidpoint(usize),
}

pub type CropExportCallback = Rc<dyn Fn((f64, f64, f64, f64))>;

/// Tracks an in-progress draw operation.
#[derive(Clone)]
pub struct DrawingState {
    /// World-space point where the drag started.
    pub anchor: (f64, f64),
    /// Which tool is being used.
    pub tool: Tool,
    /// Colour to apply to the new element.
    pub color: ShapeColor,
}

/// All interior mutable state owned by the canvas component.
#[derive(Clone, Copy)]
pub struct CanvasState {
    pub screen_size: RwSignal<(f64, f64)>,
    pub svg_ref: NodeRef<leptos::svg::Svg>,
    pub drawing: RwSignal<Option<DrawingState>>,
    pub shift_pressed: RwSignal<bool>,
    pub alt_pressed: RwSignal<bool>,
    pub pan_anchor: RwSignal<Option<(f64, f64)>>,
    pub select_anchor: RwSignal<Option<(f64, f64)>>,
    pub erasing: RwSignal<bool>,
    pub selected_ids: RwSignal<Vec<ElementId>>,
    pub moving_anchor: RwSignal<Option<(f64, f64)>>,
    pub drag_action: RwSignal<Option<Handle>>,
    pub drag_bounds: RwSignal<Option<(f64, f64, f64, f64)>>,
    pub drag_angle: RwSignal<Option<f64>>,
    pub last_world: RwSignal<Option<(f64, f64)>>,
    pub drag_originals: RwSignal<Vec<Element>>,
    /// Frozen overlay bounds, set at rotate drag start, cleared on pointer-up.
    pub overlay_freeze: RwSignal<Option<(f64, f64, f64, f64)>>,
    /// Cumulative rotation delta during a rotate drag (0 when not rotating).
    pub rotation_delta: RwSignal<f64>,
    pub editing_id: RwSignal<Option<ElementId>>,
    pub edit_text: RwSignal<String>,
    pub textarea_ref: NodeRef<leptos::html::Textarea>,
}

impl CanvasState {
    pub fn new() -> Self {
        Self {
            screen_size: create_rw_signal((0.0, 0.0)),
            svg_ref: create_node_ref(),
            drawing: create_rw_signal(None),
            shift_pressed: create_rw_signal(false),
            alt_pressed: create_rw_signal(false),
            pan_anchor: create_rw_signal(None),
            select_anchor: create_rw_signal(None),
            erasing: create_rw_signal(false),
            selected_ids: create_rw_signal(Vec::new()),
            moving_anchor: create_rw_signal(None),
            drag_action: create_rw_signal(None),
            drag_bounds: create_rw_signal(None),
            drag_angle: create_rw_signal(None),
            last_world: create_rw_signal(None),
            drag_originals: create_rw_signal(Vec::new()),
            overlay_freeze: create_rw_signal(None),
            rotation_delta: create_rw_signal(0.0),
            editing_id: create_rw_signal(None),
            edit_text: create_rw_signal(String::new()),
            textarea_ref: create_node_ref(),
        }
    }
}

/// Props passed into the Canvas component, bundled for easy forwarding to handlers.
#[derive(Clone)]
pub struct CanvasInputs {
    pub cursor_screen: RwSignal<(f64, f64)>,
    pub cursor_world: RwSignal<(f64, f64)>,
    pub viewport: RwSignal<super::viewport::Viewport>,
    pub selected_tool: RwSignal<Tool>,
    pub selected_color: RwSignal<ShapeColor>,
    pub canvas_mode: RwSignal<CanvasMode>,
    pub scene: RwSignal<Scene>,
    pub eraser_active: RwSignal<bool>,
    pub settings: RwSignal<CanvasSettings>,
    pub push_snapshot: Rc<dyn Fn()>,
    pub export_crop_active: RwSignal<bool>,
    pub on_crop_export: RwSignal<Option<CropExportCallback>>,
}

// ── Helper functions used by pointer event handlers ──────────────────────

/// Erase the topmost element at a world-space point.
pub fn hit_and_erase(point: (f64, f64), scene: RwSignal<Scene>) {
    let id = scene.with(|s| {
        s.elements()
            .iter()
            .rev()
            .find(|el| el.hit_test(Point { x: point.0, y: point.1 }, HIT_MARGIN))
            .map(|el| el.id())
    });
    if let Some(id) = id {
        scene.update(|s| s.remove_by_id(id));
    }
}

/// Check whether a rectangle fully contains an element's bounding box.
pub fn rect_fully_contains_element(rx: f64, ry: f64, rw: f64, rh: f64, el: &Element) -> bool {
    let (ex, ey, ew, eh) = el.bounds();
    ex >= rx && ey >= ry && (ex + ew) <= (rx + rw) && (ey + eh) <= (ry + rh)
}

/// Compute the combined axis-aligned bounding box of a set of element IDs.
pub fn combined_bounds(ids: &[ElementId], elements: &[Element]) -> Option<(f64, f64, f64, f64)> {
    let mut out: Option<(f64, f64, f64, f64)> = None;
    for el in elements {
        if ids.contains(&el.id()) {
            let (ex, ey, ew, eh) = el.bounds();
            let (x1, y1, x2, y2) = (ex, ey, ex + ew, ey + eh);
            match out {
                None => out = Some((x1, y1, x2, y2)),
                Some((min_x, min_y, max_x, max_y)) => {
                    out = Some((min_x.min(x1), min_y.min(y1), max_x.max(x2), max_y.max(y2)));
                }
            }
        }
    }
    out.map(|(x1, y1, x2, y2)| (x1, y1, x2 - x1, y2 - y1))
}

/// Find the topmost element under a point and return its ID.
pub fn hit_test_topmost(point: (f64, f64), elements: &[Element]) -> Option<ElementId> {
    elements
        .iter()
        .rev()
        .find(|el| el.hit_test(Point { x: point.0, y: point.1 }, HIT_MARGIN))
        .map(|el| el.id())
}

/// Returns true if the point is inside any element's bounding box (with margin).
pub fn point_inside_any_element(point: (f64, f64), elements: &[Element]) -> bool {
    elements.iter().any(|el| {
        let (ex, ey, ew, eh) = el.bounds();
        let (px, py) = point;
        px >= ex - CLICK_MARGIN
            && px <= ex + ew + CLICK_MARGIN
            && py >= ey - CLICK_MARGIN
            && py <= ey + eh + CLICK_MARGIN
    })
}
