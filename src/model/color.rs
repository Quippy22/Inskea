use serde::de::{self, Deserializer, Unexpected};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use std::fmt;

/// An RGB color stored as a `#rrggbb` hex string.
#[derive(Clone, Debug, PartialEq)]
pub struct Color(String);

impl Color {
    pub fn new(hex: &str) -> Self {
        let clean = hex.trim_start_matches('#');
        if clean.len() == 3 {
            let expanded: String = clean.chars().flat_map(|c| std::iter::repeat(c).take(2)).collect();
            Self(format!("#{expanded}"))
        } else if clean.len() == 6 {
            Self(format!("#{clean}"))
        } else {
            Self("#ffffff".to_string())
        }
    }

    pub fn to_hex(&self) -> String {
        self.0.clone()
    }

    pub const WHITE: &'static str = "#ffffff";
    pub const YELLOW: &'static str = "#eab308";
    pub const GREEN: &'static str = "#22c55e";
    pub const CYAN: &'static str = "#06b6d4";
    pub const BLUE: &'static str = "#3b82f6";
    pub const PURPLE: &'static str = "#a855f7";
    pub const ORANGE: &'static str = "#f97316";
    pub const RED: &'static str = "#ef4444";
}

impl Default for Color {
    fn default() -> Self {
        Self("#ffffff".to_string())
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        if s.starts_with('#') && (s.len() == 7 || s.len() == 4) {
            Ok(Color::new(&s))
        } else {
            Err(de::Error::invalid_value(
                Unexpected::Str(&s),
                &"a hex colour string (#rrggbb or #rgb)",
            ))
        }
    }
}
