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
pub enum GridSize {
    Px10,
    Px20,
    Px30,
}

impl GridSize {
    pub fn px(&self) -> f64 {
        match self {
            GridSize::Px10 => 10.0,
            GridSize::Px20 => 20.0,
            GridSize::Px30 => 30.0,
        }
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
