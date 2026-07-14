use crate::model::StylingKind;

/// Available drawing tool types.
#[derive(Clone, Copy, PartialEq)]
pub enum Tool {
    Rectangle,
    Ellipse,
    Line,
    Arrow,
    Text,
    Freehand,
}

impl Tool {
    /// The kind of styling panel to show when this tool is active.
    pub fn styling_kind(&self) -> StylingKind {
        match self {
            Tool::Rectangle => StylingKind::Rectangle,
            Tool::Ellipse => StylingKind::Ellipse,
            Tool::Line => StylingKind::Line,
            Tool::Arrow => StylingKind::Arrow,
            Tool::Text => StylingKind::Text,
            Tool::Freehand => StylingKind::Freehand,
        }
    }
}
