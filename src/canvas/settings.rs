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

/// Application palette. The canvas, chrome, and exports all follow this.
///
/// Older settings files stored `canvas_bg = "Dark"` / `"Light"`; the aliases
/// keep those files readable. The persistence layer writes the new `theme`
/// key instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    #[serde(alias = "Dark")]
    Storm,
    TokyoNight,
    Mono,
    #[serde(alias = "Light")]
    Paper,
    Greenboard,
}

/// Concrete colours behind a theme.
///
/// The chrome reads the matching CSS custom properties from `input.css`; the
/// canvas, overlays, and SVG exports need literal hex values instead because
/// SVG markup cannot use Tailwind tokens.
#[derive(Clone, Copy)]
pub struct ThemePalette {
    pub surface: &'static str,
    pub panel: &'static str,
    pub fg: &'static str,
    pub accent: &'static str,
    pub border: &'static str,
    pub green: &'static str,
    pub canvas: &'static str,
    pub grid: &'static str,
    pub halo: &'static str,
    pub light: bool,
}

const STORM: ThemePalette = ThemePalette {
    surface: "#1e1f2e",
    panel: "#24283b",
    fg: "#a9b1d6",
    accent: "#7aa2f7",
    border: "#3b4261",
    green: "#9ece6a",
    canvas: "#1a1b26",
    grid: "#d1d5db",
    halo: "#0b0d12",
    light: false,
};

const TOKYO_NIGHT: ThemePalette = ThemePalette {
    surface: "#1a1b26",
    panel: "#1f2335",
    fg: "#c0caf5",
    accent: "#bb9af7",
    border: "#2f3549",
    green: "#9ece6a",
    canvas: "#16161e",
    grid: "#c0caf5",
    halo: "#0b0d16",
    light: false,
};

const MONO: ThemePalette = ThemePalette {
    surface: "#171718",
    panel: "#1c1c1e",
    fg: "#e6e6e9",
    accent: "#c9c9ce",
    border: "#2c2c2f",
    green: "#c2c2c7",
    canvas: "#121213",
    grid: "#e6e6e9",
    halo: "#08080a",
    light: false,
};

const PAPER: ThemePalette = ThemePalette {
    surface: "#ebe7de",
    panel: "#fbf9f4",
    fg: "#2b2925",
    accent: "#3f6a8c",
    border: "#d8d1c3",
    green: "#4c7a46",
    canvas: "#f3f0e9",
    grid: "#8c867a",
    halo: "#fbf9f4",
    light: true,
};

const GREENBOARD: ThemePalette = ThemePalette {
    surface: "#12251c",
    panel: "#162d22",
    fg: "#d7e6db",
    accent: "#8fd3a5",
    border: "#294637",
    green: "#a3d977",
    canvas: "#0e1d17",
    grid: "#c8d8cc",
    halo: "#07110d",
    light: false,
};

impl Theme {
    pub const ALL: [Theme; 5] = [
        Theme::Storm,
        Theme::TokyoNight,
        Theme::Mono,
        Theme::Paper,
        Theme::Greenboard,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Theme::Storm => "Storm",
            Theme::TokyoNight => "Tokyo Night",
            Theme::Mono => "Mono",
            Theme::Paper => "Paper",
            Theme::Greenboard => "Greenboard",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Theme::Storm => "Cool blue night",
            Theme::TokyoNight => "Neon violet night",
            Theme::Mono => "Charcoal and ink",
            Theme::Paper => "Warm light canvas",
            Theme::Greenboard => "Chalk on deep green",
        }
    }

    /// Value used for the `data-theme` attribute on the document root.
    pub fn attr(self) -> &'static str {
        match self {
            Theme::Storm => "storm",
            Theme::TokyoNight => "tokyo-night",
            Theme::Mono => "mono",
            Theme::Paper => "paper",
            Theme::Greenboard => "greenboard",
        }
    }

    pub fn palette(self) -> &'static ThemePalette {
        match self {
            Theme::Storm => &STORM,
            Theme::TokyoNight => &TOKYO_NIGHT,
            Theme::Mono => &MONO,
            Theme::Paper => &PAPER,
            Theme::Greenboard => &GREENBOARD,
        }
    }

    pub fn canvas(self) -> &'static str {
        self.palette().canvas
    }

    pub fn grid(self) -> &'static str {
        self.palette().grid
    }

    pub fn accent(self) -> &'static str {
        self.palette().accent
    }

    pub fn guide_alignment(self) -> &'static str {
        self.palette().accent
    }

    pub fn guide_spacing(self) -> &'static str {
        self.palette().green
    }

    pub fn halo(self) -> &'static str {
        self.palette().halo
    }

    pub fn is_light(self) -> bool {
        self.palette().light
    }

    /// Custom cursor artwork is white on dark themes; invert it on paper.
    pub fn cursor(self) -> &'static str {
        if self.is_light() {
            "#1f2937"
        } else {
            "#ffffff"
        }
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CanvasSettings {
    pub center_style: CenterStyle,
    pub grid_style: GridStyle,
    pub grid_size: GridSize,
    pub autosave: bool,
    pub theme: Theme,
}
