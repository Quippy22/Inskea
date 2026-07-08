use crate::model::elements::text::WrappedText;
use crate::model::{Arrow, Element, ElementData, Ellipse, Freehand, FromDrag, Line, Point, Rectangle, Text};
use crate::ui::dock::Tool;
use leptos::{ev, SignalGet, SignalSet, SignalUpdate, SignalWith};
use super::state::{CanvasInputs, CanvasState, DrawingState};
use super::MIN_DRAG_DIST;

const DEFAULT_FONT_SIZE: f64 = 24.0;

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
pub fn draw_pointer_down(
    _ev: &ev::PointerEvent,
    world: (f64, f64),
    st: &mut CanvasState,
    props: &mut CanvasInputs,
) {
    let tool = props.selected_tool.get();
    let color = props.selected_color.get();

    if tool == Tool::Text {
        (props.push_snapshot)();
        let mut data = ElementData::new(0);
        data.x = world.0;
        data.y = world.1;
        data.font_size = DEFAULT_FONT_SIZE;
        data.width = 0.0;
        data.height = 0.0;
        data.stroke_color = color;
        let id = props.scene.with(|s| s.next_id);
        props.scene.update(|s| {
            let w = data.width;
            let fs = data.font_size;
            s.add_element(Element::Text(Text { data, wrapped: WrappedText::new("", w, fs) }));
        });
        st.editing_id.set(Some(id));
        st.edit_text.set(String::new());
        return;
    }

    if tool == Tool::Freehand {
        (props.push_snapshot)();
        props.scene.update(|s| {
            let mut data = ElementData::new(0);
            data.stroke_color = color;
            s.add_element(Element::Freehand(Freehand {
                data,
                points: vec![Point { x: world.0, y: world.1 }],
            }));
        });
        st.drawing.set(Some(DrawingState { anchor: world, tool, color }));
        return;
    }

    // No snapshot here — it fires once in draw_pointer_up, immediately
    // before the element is added to the scene, so degenerate clicks
    // below MIN_DRAG_DIST don't pollute the undo stack.
    st.drawing.set(Some(DrawingState { anchor: world, tool, color }));
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
    st: &mut CanvasState,
    props: &mut CanvasInputs,
) {
    if let Some(state) = st.drawing.get() {
        if state.tool == Tool::Freehand {
            st.drawing.set(None);
            return;
        }

        let world = props.cursor_world.get();
        let dx = world.0 - state.anchor.0;
        let dy = world.1 - state.anchor.1;
        if dx.hypot(dy) < MIN_DRAG_DIST {
            st.drawing.set(None);
            return;
        }
        let el: Element = match state.tool {
            Tool::Rectangle => Rectangle::from_drag(state.anchor, world, state.color, st.shift_pressed.get()).into(),
            Tool::Ellipse => Ellipse::from_drag(state.anchor, world, state.color, st.shift_pressed.get()).into(),
            Tool::Line => Line::from_drag(state.anchor, world, state.color, st.shift_pressed.get()).into(),
            Tool::Arrow => Arrow::from_drag(state.anchor, world, state.color, st.shift_pressed.get()).into(),
            Tool::Text => Text::from_drag(state.anchor, world, state.color, st.shift_pressed.get()).into(),
            Tool::Freehand => Freehand::from_drag(state.anchor, world, state.color, st.shift_pressed.get()).into(),
        };
        (props.push_snapshot)();
        props.scene.update(|s| { s.add_element(el); });
        st.drawing.set(None);
    }
}
