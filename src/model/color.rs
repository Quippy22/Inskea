use serde::de::{self, Deserializer, Unexpected};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use std::fmt;

/// An RGB color stored as exactly three 8-bit channels.
///
/// Serialization remains the existing `#rrggbb` string format, so `.skea`
/// files and browser settings remain compatible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color([u8; 3]);

impl Color {
    const fn from_rgb(rgb: u32) -> Self {
        Self([(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8])
    }

    /// Parse `#rgb` or `#rrggbb`. Invalid input safely falls back to white.
    pub fn new(hex: &str) -> Self {
        Self::try_new(hex).unwrap_or(Self::WHITE_VALUE)
    }

    /// Parse a CSS short or full RGB hex color without panicking.
    pub fn try_new(hex: &str) -> Option<Self> {
        let clean = hex.strip_prefix('#').unwrap_or(hex);
        let value = match clean.len() {
            3 => {
                if !clean.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return None;
                }
                let mut chars = clean.chars();
                let r = chars.next()?.to_digit(16)?;
                let g = chars.next()?.to_digit(16)?;
                let b = chars.next()?.to_digit(16)?;
                if chars.next().is_some() {
                    return None;
                }
                (r * 0x110000) | (g * 0x1100) | (b * 0x11)
            }
            6 => {
                if !clean.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return None;
                }
                u32::from_str_radix(clean, 16).ok()?
            }
            _ => return None,
        };
        Some(Self::from_rgb(value))
    }

    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0[0], self.0[1], self.0[2])
    }

    pub const WHITE: &'static str = "#ffffff";
    pub const YELLOW: &'static str = "#eab308";
    pub const GREEN: &'static str = "#22c55e";
    pub const BLUE: &'static str = "#3b82f6";
    pub const RED: &'static str = "#ef4444";
    const WHITE_VALUE: Self = Self([u8::MAX; 3]);
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE_VALUE
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.0[0], self.0[1], self.0[2])
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Color::try_new(&s).ok_or_else(|| {
            de::Error::invalid_value(
                Unexpected::Str(&s),
                &"a hex colour string (#rrggbb or #rgb)",
            )
        })
    }
}
