use super::state::{CanvasInputs, CanvasState, DrawingState};
use super::MIN_DRAG_DIST;
use crate::model::elements::text::WrappedText;
use crate::model::{
    Element, ElementData, Ellipse, Freehand, FromDrag, Line, Point, Rectangle, Text, TextAlign,
    UpdateDrag,
};
use crate::ui::dock::Tool;
use leptos::{
    ev, SignalGet, SignalGetUntracked, SignalSet, SignalUpdate, SignalWith, SignalWithUntracked,
};

/// Finish (or discard) the freehand stroke currently being drawn.
///
/// Appends the release point when one is available, then simplifies the
/// stroke with a zoom-scaled tolerance. A stroke still at its starting point
/// is treated as a click: the element and the undo snapshot taken when the
/// stroke began are both discarded, so clicking with the freehand tool leaves
/// no trace. Does nothing when the in-progress element is not freehand.
pub(crate) fn finish_freehand(
    st: &mut CanvasState,
    props: &mut CanvasInputs,
    world: Option<Point>,
) {
    let Some(state) = st.drawing.get_untracked() else {
        return;
    };
    if state.tool != Tool::Freehand {
        return;
    }
    st.drawing.set(None);
    let Some(id) = state.element_id else {
        return;
    };
    let zoom = props.viewport.get_untracked().zoom;
    let mut is_click = false;
    props.scene.update(|s| {
        if let Some(mut element) = s.element_by_id_mut(id) {
            if let Element::Freehand(fh) = &mut *element {
                if let Some(world) = world {
                    fh.update_drag(world, Point::from(state.anchor), false, zoom);
                }
                if fh.points.len() < 2 {
                    is_click = true;
                } else {
                    fh.finish(zoom);
                }
            }
        }
    });
    if is_click {
        props.scene.update(|s| s.remove_by_id(id));
        (props.discard_snapshot)();
    }
}

/// Abort an in-progress draw (Esc, pointer cancel). Freehand elements live in
/// the scene from pointer-down, so the element and its undo snapshot are both
/// removed; other tools have nothing in the scene yet.
pub(crate) fn cancel_drawing(st: &mut CanvasState, props: &mut CanvasInputs) {
    let Some(state) = st.drawing.get_untracked() else {
        return;
    };
    st.drawing.set(None);
    if let Some(id) = state.element_id {
        let exists = props
            .scene
            .with_untracked(|s| s.element_by_id(id).is_some());
        if exists {
            props.scene.update(|s| s.remove_by_id(id));
            (props.discard_snapshot)();
        }
    }
}

/// Handle a pointer-down event while in `Draw` mode.
///
/// **Behaviour by tool:**
///
/// - **Text** – creates a zero-sized `Text` element at the click position,
///   immediately opens the text-editing overlay. The snapshot is taken before
///   creation so undo removes the empty element.
///
/// - **Freehand** – creates a `Freehand` element with a single point at the
///   click position and stores a `DrawingState` so subsequent move events
///   can append points via `update_drag`.
///
/// - **All other tools** (Rectangle, Ellipse, Line, Arrow) – stores a
///   `DrawingState` with the anchor at the click position. The element is
///   created later in `draw_pointer_up` when the drag completes.
///   No snapshot is taken here — the snapshot fires once in
///   `draw_pointer_up` right before the element is added, so a degenerate
///   click (below MIN_DRAG_DIST) never pollutes the undo stack.
pub fn draw_pointer_down(world: Point, st: &mut CanvasState, props: &mut CanvasInputs) {
    let tool = props.selected_tool.get();
    let color = props.selected_color.get();

    if tool == Tool::Text {
        (props.push_snapshot)();
        let mut data = ElementData::new(0);
        data.world_point = world;
        data.width = 0.0;
        data.height = 0.0;
        data.style = props.default_style.get();
        let id = props.scene.with(|s| s.next_id);
        props.scene.update(|s| {
            s.add_element(Element::Text(Text {
                data,
                wrapped: WrappedText::new(""),
                align: TextAlign::Left,
            }));
        });
        st.text_edit.editing_id.set(Some(id));
        st.text_edit.edit_text.set(String::new());
        return;
    }

    if tool == Tool::Freehand {
        (props.push_snapshot)();
        let id = props.scene.with(|s| s.next_id);
        props.scene.update(|s| {
            let mut data = ElementData::new(0);
            data.style = props.default_style.get();
            s.add_element(Element::Freehand(Freehand {
                data,
                points: vec![world],
            }));
        });
        st.drawing.set(Some(DrawingState {
            anchor: (world.x, world.y),
            tool,
            color,
            element_id: Some(id),
        }));
        return;
    }

    st.drawing.set(Some(DrawingState {
        anchor: (world.x, world.y),
        tool,
        color,
        element_id: None,
    }));
}

/// Handle a pointer-up event while in `Draw` mode.
///
/// **Behaviour by tool:**
///
/// - **Freehand** – the stroke was built incrementally during move, so we
///   just clear the drawing state without creating a new element.
///
/// - **All other tools** – if the drag distance is below `MIN_DRAG_DIST`,
///   the drawing is discarded (treated as a click, not a drag). Otherwise,
///   the element is constructed via `FromDrag::from_drag` with the anchor,
///   final cursor position, colour, and shift state. A snapshot is taken
///   **after** construction (so the undo snapshot happens before the element
///   is added), then the element is added to the scene and the drawing state
///   is cleared.
pub fn draw_pointer_up(
    _ev: &ev::PointerEvent,
    world: Point,
    st: &mut CanvasState,
    props: &mut CanvasInputs,
) {
    if let Some(state) = st.drawing.get() {
        if state.tool == Tool::Freehand {
            finish_freehand(st, props, Some(world));
            return;
        }

        let dx = world.x - state.anchor.0;
        let dy = world.y - state.anchor.1;
        if dx.hypot(dy) < MIN_DRAG_DIST {
            st.drawing.set(None);
            return;
        }
        let anchor = Point::from(state.anchor);
        let world_pt = world;
        let mut el: Element = match state.tool {
            Tool::Rectangle => {
                Rectangle::from_drag(anchor, world_pt, state.color, st.shift_pressed.get()).into()
            }
            Tool::Ellipse => {
                Ellipse::from_drag(anchor, world_pt, state.color, st.shift_pressed.get()).into()
            }
            Tool::Line => {
                Line::from_drag(anchor, world_pt, state.color, st.shift_pressed.get()).into()
            }
            Tool::Arrow => {
                let line = Line::from_drag(anchor, world_pt, state.color, st.shift_pressed.get());
                Element::Line(line)
            }
            Tool::Text => {
                Text::from_drag(anchor, world_pt, state.color, st.shift_pressed.get()).into()
            }
            Tool::Freehand => {
                Freehand::from_drag(anchor, world_pt, state.color, st.shift_pressed.get()).into()
            }
        };
        el.data_mut().style = props.default_style.get();
        if let Element::Line(ref mut l) = el {
            l.line_style = props.default_line_style.get();
        }
        (props.push_snapshot)();
        props.scene.update(|s| {
            s.add_element(el);
        });
        st.drawing.set(None);
    }
}
