/// Stroke or fill color for a shape, drawn from a fixed palette that works
/// well with the Tokyo-Night-based dark theme.
///
/// Each variant maps to the Tailwind 500‑shade hex value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ShapeColor {
    Purple,
    Blue,
    Cyan,
    Green,
    Yellow,
    Orange,
    Red,
    #[default]
    White,
}

impl std::fmt::Display for ShapeColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Purple => write!(f, "Purple"),
            Self::Blue => write!(f, "Blue"),
            Self::Cyan => write!(f, "Cyan"),
            Self::Green => write!(f, "Green"),
            Self::Yellow => write!(f, "Yellow"),
            Self::Orange => write!(f, "Orange"),
            Self::Red => write!(f, "Red"),
            Self::White => write!(f, "White"),
        }
    }
}

impl ShapeColor {
    /// Return the Tailwind 500‑shade hex string for this color.
    pub fn to_hex(self) -> &'static str {
        match self {
            Self::Purple => "#a855f7",
            Self::Blue => "#3b82f6",
            Self::Cyan => "#06b6d4",
            Self::Green => "#22c55e",
            Self::Yellow => "#eab308",
            Self::Orange => "#f97316",
            Self::Red => "#ef4444",
            Self::White => "#ffffff",
        }
    }
}
