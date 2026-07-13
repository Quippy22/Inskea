use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CenterStyle {
    Crosshair,
    Dot,
    Off,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GridStyle {
    Dot,
    Line,
    Off,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GridSize(f64);

impl GridSize {
    pub fn px(&self) -> f64 {
        self.0
    }

    pub fn new(v: f64) -> Self {
        Self(v)
    }
}

impl From<GridSize> for f64 {
    fn from(v: GridSize) -> Self {
        v.0
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CanvasBg {
    Dark,
    Light,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CanvasSettings {
    pub center_style: CenterStyle,
    pub grid_style: GridStyle,
    pub grid_size: GridSize,
    pub autosave: bool,
    pub canvas_bg: CanvasBg,
}
