// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

/// RGBA color (0–255 per channel).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Color {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel (255 = fully opaque).
    pub a: u8,
}

impl Color {
    /// Creates a fully opaque color from RGB components.
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Creates a color from RGBA components.
    #[must_use]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Format as CSS `rgba(...)` string.
    #[must_use]
    pub fn to_css(&self) -> String {
        if self.a == 255 {
            format!("rgb({},{},{})", self.r, self.g, self.b)
        } else {
            format!(
                "rgba({},{},{},{:.2})",
                self.r,
                self.g,
                self.b,
                f64::from(self.a) / 255.0
            )
        }
    }

    /// Black (`#000000`).
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    /// White (`#FFFFFF`).
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    /// Red, used for bearish candles.
    pub const RED: Self = Self::rgb(239, 83, 80);
    /// Green, used for bullish candles.
    pub const GREEN: Self = Self::rgb(38, 166, 154);
    /// Mid-gray.
    pub const GRAY: Self = Self::rgb(128, 128, 128);
    /// Light gray, used for wicks and labels.
    pub const LIGHT_GRAY: Self = Self::rgb(200, 200, 200);
}

/// Style for line drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineStyle {
    /// Stroke color.
    pub color: Color,
    /// Stroke width in pixels.
    pub width: f64,
}

impl Default for LineStyle {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            width: 1.0,
        }
    }
}

/// Style for filled shapes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillStyle {
    /// Fill color.
    pub color: Color,
}

/// Style for text rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// Text color.
    pub color: Color,
    /// Font size in pixels.
    pub size: f64,
    /// CSS font-family string.
    pub font_family: String,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            size: 12.0,
            font_family: "monospace".to_string(),
        }
    }
}

/// Horizontal text anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAnchor {
    /// Anchors text at its start (left-aligned).
    Start,
    /// Anchors text at its midpoint (center-aligned).
    Middle,
    /// Anchors text at its end (right-aligned).
    End,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_rgb_css() {
        assert_eq!(Color::rgb(255, 0, 0).to_css(), "rgb(255,0,0)");
    }

    #[test]
    fn color_rgba_css() {
        let c = Color::rgba(0, 128, 255, 128);
        let css = c.to_css();
        assert!(css.starts_with("rgba(0,128,255,"));
        assert!(css.contains("0.50"));
    }

    #[test]
    fn color_fully_opaque_uses_rgb() {
        let css = Color::rgba(10, 20, 30, 255).to_css();
        assert!(css.starts_with("rgb("));
    }

    #[test]
    fn default_line_style() {
        let ls = LineStyle::default();
        assert_eq!(ls.color, Color::BLACK);
        assert!((ls.width - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn default_text_style() {
        let ts = TextStyle::default();
        assert_eq!(ts.color, Color::BLACK);
        assert!((ts.size - 12.0).abs() < f64::EPSILON);
        assert_eq!(ts.font_family, "monospace");
    }
}
