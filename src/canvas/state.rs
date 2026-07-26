use crate::canvas::settings::CanvasSettings;
use crate::model::resize::ResizeHandle;
use crate::model::Color;
use crate::model::{
    Bounds, Element, ElementId, ElementStyle, HitTest, LineStyle, Point, Rect, Scene,
};
use crate::ui::dock::Tool;
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
    Pan,
    /// Draw a new element based on the active tool.
    Draw,
    /// Erase elements by clicking or dragging over them.
    Erase,
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

/// Drag-related state (resize, move, rotate, path-point drag).
#[derive(Clone, Copy)]
pub struct DragState {
    pub moving_anchor: RwSignal<Option<Point>>,
    pub drag_action: RwSignal<Option<Handle>>,
    pub drag_bounds: RwSignal<Option<Rect>>,
    pub drag_angle: RwSignal<Option<f64>>,
    pub last_world: RwSignal<Option<Point>>,
    pub drag_originals: RwSignal<Vec<Element>>,
    pub overlay_freeze: RwSignal<Option<Rect>>,
    pub rotation_delta: RwSignal<f64>,
    pub hovered_handle: RwSignal<Option<Handle>>,
}

impl DragState {
    pub fn new() -> Self {
        Self {
            moving_anchor: create_rw_signal(None),
            drag_action: create_rw_signal(None),
            drag_bounds: create_rw_signal(None),
            drag_angle: create_rw_signal(None),
            last_world: create_rw_signal(None),
            drag_originals: create_rw_signal(Vec::new()),
            overlay_freeze: create_rw_signal(None),
            rotation_delta: create_rw_signal(0.0),
            hovered_handle: create_rw_signal(None),
        }
    }
}

/// Text-editing state.
#[derive(Clone, Copy)]
pub struct TextEditState {
    pub editing_id: RwSignal<Option<ElementId>>,
    pub edit_text: RwSignal<String>,
    pub textarea_ref: NodeRef<leptos::html::Textarea>,
}

impl TextEditState {
    pub fn new() -> Self {
        Self {
            editing_id: create_rw_signal(None),
            edit_text: create_rw_signal(String::new()),
            textarea_ref: create_node_ref(),
        }
    }
}

pub type CropExportCallback = Rc<dyn Fn(Rect)>;

/// Tracks an in-progress draw operation.
#[derive(Clone)]
pub struct DrawingState {
    /// World-space point where the drag started.
    pub anchor: (f64, f64),
    /// Which tool is being used.
    pub tool: Tool,
    /// Colour to apply to the new element.
    pub color: Color,
}

/// All interior mutable state owned by the canvas component.
#[derive(Clone, Copy)]
pub struct CanvasState {
    pub screen_size: RwSignal<Point>,
    pub svg_ref: NodeRef<leptos::svg::Svg>,
    pub drawing: RwSignal<Option<DrawingState>>,
    pub shift_pressed: RwSignal<bool>,
    pub alt_pressed: RwSignal<bool>,
    pub pan_anchor: RwSignal<Option<(f64, f64)>>,
    pub select_anchor: RwSignal<Option<Point>>,
    pub erasing: RwSignal<bool>,
    pub selected_ids: RwSignal<Vec<ElementId>>,
    pub cursor_style: RwSignal<String>,
    pub drag: DragState,
    pub text_edit: TextEditState,
}

impl CanvasState {
    pub fn new() -> Self {
        Self {
            screen_size: create_rw_signal(Point::new(0.0, 0.0)),
            svg_ref: create_node_ref(),
            drawing: create_rw_signal(None),
            shift_pressed: create_rw_signal(false),
            alt_pressed: create_rw_signal(false),
            pan_anchor: create_rw_signal(None),
            select_anchor: create_rw_signal(None),
            erasing: create_rw_signal(false),
            selected_ids: create_rw_signal(Vec::new()),
            cursor_style: create_rw_signal("default".to_string()),
            drag: DragState::new(),
            text_edit: TextEditState::new(),
        }
    }
}

/// Props passed into the Canvas component, bundled for easy forwarding to handlers.
#[derive(Clone)]
pub struct CanvasInputs {
    pub cursor_screen: RwSignal<(f64, f64)>,
    pub cursor_world: RwSignal<Point>,
    pub viewport: RwSignal<super::viewport::Viewport>,
    pub selected_tool: RwSignal<Tool>,
    pub selected_color: RwSignal<Color>,
    pub canvas_mode: RwSignal<CanvasMode>,
    pub scene: RwSignal<Scene>,
    pub eraser_active: RwSignal<bool>,
    pub settings: RwSignal<CanvasSettings>,
    pub push_snapshot: Rc<dyn Fn()>,
    pub export_crop_active: RwSignal<bool>,
    pub on_crop_export: RwSignal<Option<CropExportCallback>>,
    pub default_style: RwSignal<ElementStyle>,
    pub default_line_style: RwSignal<LineStyle>,
}

// ── Helper functions used by pointer event handlers ──────────────────────

pub fn hit_and_erase(point: Point, scene: RwSignal<Scene>) {
    let id = scene.with(|s| {
        s.elements()
            .iter()
            .rev()
            .find(|el| el.hit_test(point, HIT_MARGIN))
            .map(|el| el.id())
    });
    if let Some(id) = id {
        scene.update(|s| s.remove_by_id(id));
    }
}

pub fn rect_fully_contains_element(rect: Rect, el: &Element) -> bool {
    rect.contains_rect(&el.bounds())
}

pub fn combined_bounds(ids: &[ElementId], elements: &[Element]) -> Option<Rect> {
    let mut out: Option<Rect> = None;
    for el in elements {
        if ids.contains(&el.id()) {
            let b = el.bounds();
            match out {
                None => out = Some(b),
                Some(prev) => out = Some(prev.union(&b)),
            }
        }
    }
    out
}

pub fn hit_test_topmost(point: Point, elements: &[Element]) -> Option<ElementId> {
    elements
        .iter()
        .rev()
        .find(|el| el.hit_test(point, HIT_MARGIN))
        .map(|el| el.id())
}

pub fn point_inside_any_element(point: Point, elements: &[Element]) -> bool {
    elements.iter().any(|el| {
        let b = el.bounds();
        point.x >= b.x - CLICK_MARGIN
            && point.x <= b.x + b.w + CLICK_MARGIN
            && point.y >= b.y - CLICK_MARGIN
            && point.y <= b.y + b.h + CLICK_MARGIN
    })
}
