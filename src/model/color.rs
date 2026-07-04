/// Stroke or fill color for a shape, drawn from a fixed palette that works
/// well with the Tokyo-Night-based dark theme.
///
/// Each variant maps to the Tailwind 500‑shade hex value.
#[derive(Clone, Debug, PartialEq)]
pub enum ShapeColor {
    Purple,
    Blue,
    Cyan,
    Green,
    Yellow,
    Orange,
    Red,
    White,
}

impl Default for ShapeColor {
    fn default() -> Self {
        Self::White
    }
}

impl ShapeColor {
    /// Return the Tailwind 500‑shade hex string for this color.
    pub fn to_hex(&self) -> &'static str {
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
