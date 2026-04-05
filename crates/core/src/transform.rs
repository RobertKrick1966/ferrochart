// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use crate::{Point, PriceRange, Rect, TimeRange};

/// Y-axis scale mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum YScaleMode {
    /// Linear price-to-pixel mapping (default).
    #[default]
    Linear,
    /// Logarithmic mapping: `ln(price)` is mapped linearly to pixels.
    /// Requires `price_range.min > 0`.
    Logarithmic,
}

/// Defines the visible data window and the pixel area it maps to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// Pixel rectangle the viewport maps into.
    pub rect: Rect,
    /// Visible bar index range.
    pub time_range: TimeRange,
    /// Visible price range.
    pub price_range: PriceRange,
}

/// Bidirectional mapping between data space (bar index, price) and pixel space.
///
/// Precomputes scale and offset so repeated `to_pixel` calls are just multiply-add.
/// In logarithmic mode, the Y mapping operates on `ln(price)`.
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    x_scale: f64,
    y_scale: f64,
    x_offset: f64,
    y_offset: f64,
    y_mode: YScaleMode,
}

impl Transform {
    /// Build a linear transform from a [`Viewport`].
    ///
    /// Maps `time_range.start` -> left edge of `rect`, `time_range.end - 1` -> right edge.
    /// Maps `price_range.max` -> top of `rect`, `price_range.min` -> bottom.
    #[must_use]
    pub fn from_viewport(vp: &Viewport) -> Self {
        Self::from_viewport_with_mode(vp, YScaleMode::Linear)
    }

    /// Build a transform with a specific Y-axis scale mode.
    ///
    /// In [`YScaleMode::Logarithmic`] mode, prices are mapped through `ln()`.
    /// Falls back to linear if `price_range.min <= 0`.
    #[must_use]
    pub fn from_viewport_with_mode(vp: &Viewport, mode: YScaleMode) -> Self {
        let num_bars = vp.time_range.len();
        let x_scale = if num_bars > 1 {
            vp.rect.width / (num_bars - 1) as f64
        } else {
            vp.rect.width
        };
        let x_offset = vp.rect.x - vp.time_range.start as f64 * x_scale;

        // Fall back to linear if prices are non-positive (log undefined)
        let effective_mode = if mode == YScaleMode::Logarithmic && vp.price_range.min > 0.0 {
            YScaleMode::Logarithmic
        } else {
            YScaleMode::Linear
        };

        let (y_scale, y_offset) = match effective_mode {
            YScaleMode::Linear => {
                let price_span = vp.price_range.span();
                let ys = if price_span.abs() > f64::EPSILON {
                    -vp.rect.height / price_span
                } else {
                    0.0
                };
                let yo = vp.rect.y - vp.price_range.max * ys;
                (ys, yo)
            }
            YScaleMode::Logarithmic => {
                let log_max = vp.price_range.max.ln();
                let log_min = vp.price_range.min.ln();
                let log_span = log_max - log_min;
                let ys = if log_span.abs() > f64::EPSILON {
                    -vp.rect.height / log_span
                } else {
                    0.0
                };
                let yo = vp.rect.y - log_max * ys;
                (ys, yo)
            }
        };

        Self {
            x_scale,
            y_scale,
            x_offset,
            y_offset,
            y_mode: effective_mode,
        }
    }

    /// Map a (`bar_index`, price) pair to pixel coordinates.
    #[must_use]
    pub fn to_pixel(&self, bar_index: f64, price: f64) -> Point {
        let y_val = match self.y_mode {
            YScaleMode::Linear => price,
            YScaleMode::Logarithmic => price.max(f64::EPSILON).ln(),
        };
        Point {
            x: bar_index * self.x_scale + self.x_offset,
            y: y_val * self.y_scale + self.y_offset,
        }
    }

    /// Map pixel coordinates back to (`bar_index`, price).
    #[must_use]
    pub fn to_data(&self, pixel: Point) -> (f64, f64) {
        let bar_index = if self.x_scale.abs() > f64::EPSILON {
            (pixel.x - self.x_offset) / self.x_scale
        } else {
            0.0
        };
        let raw_y = if self.y_scale.abs() > f64::EPSILON {
            (pixel.y - self.y_offset) / self.y_scale
        } else {
            0.0
        };
        let price = match self.y_mode {
            YScaleMode::Linear => raw_y,
            YScaleMode::Logarithmic => raw_y.exp(),
        };
        (bar_index, price)
    }

    /// Pixel X center for a given bar index.
    #[must_use]
    pub fn bar_x(&self, bar_index: usize) -> f64 {
        bar_index as f64 * self.x_scale + self.x_offset
    }

    /// Pixel Y for a given price.
    #[must_use]
    pub fn price_y(&self, price: f64) -> f64 {
        let y_val = match self.y_mode {
            YScaleMode::Linear => price,
            YScaleMode::Logarithmic => price.max(f64::EPSILON).ln(),
        };
        y_val * self.y_scale + self.y_offset
    }

    /// Width in pixels available per bar (for sizing candle bodies).
    #[must_use]
    pub fn bar_width(&self) -> f64 {
        self.x_scale
    }

    /// Inverse of `price_y`: pixel Y → price.
    #[must_use]
    pub fn pixel_y_to_price(&self, pixel_y: f64) -> f64 {
        let y_val = (pixel_y - self.y_offset) / self.y_scale;
        match self.y_mode {
            YScaleMode::Linear => y_val,
            YScaleMode::Logarithmic => y_val.exp(),
        }
    }

    /// Inverse of `bar_x`: pixel X → bar index (fractional, relative to visible slice).
    #[must_use]
    pub fn pixel_x_to_bar(&self, pixel_x: f64) -> f64 {
        (pixel_x - self.x_offset) / self.x_scale
    }

    /// Returns the active Y-axis scale mode.
    #[must_use]
    pub fn y_mode(&self) -> YScaleMode {
        self.y_mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_viewport() -> Viewport {
        Viewport {
            rect: Rect::new(0.0, 0.0, 900.0, 600.0),
            time_range: TimeRange::new(0, 10),
            price_range: PriceRange::new(100.0, 200.0),
        }
    }

    #[test]
    fn first_bar_maps_to_left_edge() {
        let t = Transform::from_viewport(&test_viewport());
        let p = t.to_pixel(0.0, 150.0);
        assert!((p.x - 0.0).abs() < 1e-9);
    }

    #[test]
    fn last_bar_maps_to_right_edge() {
        let t = Transform::from_viewport(&test_viewport());
        let p = t.to_pixel(9.0, 150.0);
        assert!((p.x - 900.0).abs() < 1e-9);
    }

    #[test]
    fn max_price_maps_to_top() {
        let t = Transform::from_viewport(&test_viewport());
        let p = t.to_pixel(5.0, 200.0);
        assert!((p.y - 0.0).abs() < 1e-9);
    }

    #[test]
    fn min_price_maps_to_bottom() {
        let t = Transform::from_viewport(&test_viewport());
        let p = t.to_pixel(5.0, 100.0);
        assert!((p.y - 600.0).abs() < 1e-9);
    }

    #[test]
    fn mid_price_maps_to_center_y() {
        let t = Transform::from_viewport(&test_viewport());
        let p = t.to_pixel(5.0, 150.0);
        assert!((p.y - 300.0).abs() < 1e-9);
    }

    #[test]
    fn round_trip_to_pixel_to_data() {
        let t = Transform::from_viewport(&test_viewport());
        let bar = 3.5_f64;
        let price = 142.0_f64;
        let pixel = t.to_pixel(bar, price);
        let (bar_back, price_back) = t.to_data(pixel);
        assert!((bar_back - bar).abs() < 1e-9);
        assert!((price_back - price).abs() < 1e-9);
    }

    #[test]
    fn bar_x_matches_to_pixel() {
        let t = Transform::from_viewport(&test_viewport());
        for i in 0..10 {
            let px = t.to_pixel(i as f64, 150.0);
            assert!((t.bar_x(i) - px.x).abs() < 1e-9);
        }
    }

    #[test]
    fn price_y_matches_to_pixel() {
        let t = Transform::from_viewport(&test_viewport());
        for price in [100.0, 120.0, 150.0, 180.0, 200.0] {
            let px = t.to_pixel(0.0, price);
            assert!((t.price_y(price) - px.y).abs() < 1e-9);
        }
    }

    #[test]
    fn bar_width_positive() {
        let t = Transform::from_viewport(&test_viewport());
        assert!(t.bar_width() > 0.0);
        assert!((t.bar_width() - 100.0).abs() < 1e-9); // 900 / 9 = 100
    }

    #[test]
    fn single_bar_viewport() {
        let vp = Viewport {
            rect: Rect::new(0.0, 0.0, 800.0, 400.0),
            time_range: TimeRange::new(0, 1),
            price_range: PriceRange::new(50.0, 150.0),
        };
        let t = Transform::from_viewport(&vp);
        let p = t.to_pixel(0.0, 150.0);
        assert!((p.x - 0.0).abs() < 1e-9);
        assert!((p.y - 0.0).abs() < 1e-9);
    }

    #[test]
    fn viewport_with_offset() {
        let vp = Viewport {
            rect: Rect::new(50.0, 30.0, 900.0, 600.0),
            time_range: TimeRange::new(0, 10),
            price_range: PriceRange::new(100.0, 200.0),
        };
        let t = Transform::from_viewport(&vp);
        let p = t.to_pixel(0.0, 200.0);
        assert!((p.x - 50.0).abs() < 1e-9);
        assert!((p.y - 30.0).abs() < 1e-9);
    }

    #[test]
    fn zero_price_span() {
        let vp = Viewport {
            rect: Rect::new(0.0, 0.0, 800.0, 400.0),
            time_range: TimeRange::new(0, 10),
            price_range: PriceRange::new(100.0, 100.0),
        };
        let t = Transform::from_viewport(&vp);
        // With zero span, y_scale is 0 → all prices map to y_offset
        let p = t.to_pixel(0.0, 100.0);
        assert!(p.y.is_finite());
    }

    fn log_viewport() -> Viewport {
        Viewport {
            rect: Rect::new(0.0, 0.0, 900.0, 600.0),
            time_range: TimeRange::new(0, 10),
            price_range: PriceRange::new(10.0, 1000.0),
        }
    }

    #[test]
    fn log_max_price_maps_to_top() {
        let t = Transform::from_viewport_with_mode(&log_viewport(), YScaleMode::Logarithmic);
        let p = t.to_pixel(5.0, 1000.0);
        assert!((p.y - 0.0).abs() < 1e-6);
    }

    #[test]
    fn log_min_price_maps_to_bottom() {
        let t = Transform::from_viewport_with_mode(&log_viewport(), YScaleMode::Logarithmic);
        let p = t.to_pixel(5.0, 10.0);
        assert!((p.y - 600.0).abs() < 1e-6);
    }

    #[test]
    fn log_geometric_mean_maps_to_center() {
        let t = Transform::from_viewport_with_mode(&log_viewport(), YScaleMode::Logarithmic);
        // Geometric mean of 10 and 1000 = sqrt(10*1000) = 100
        let p = t.to_pixel(5.0, 100.0);
        assert!((p.y - 300.0).abs() < 1e-6);
    }

    #[test]
    fn log_round_trip() {
        let t = Transform::from_viewport_with_mode(&log_viewport(), YScaleMode::Logarithmic);
        let bar = 3.5_f64;
        let price = 42.0_f64;
        let pixel = t.to_pixel(bar, price);
        let (bar_back, price_back) = t.to_data(pixel);
        assert!((bar_back - bar).abs() < 1e-9);
        assert!((price_back - price).abs() < 1e-6);
    }

    #[test]
    fn log_mode_falls_back_for_non_positive_prices() {
        let vp = Viewport {
            rect: Rect::new(0.0, 0.0, 800.0, 400.0),
            time_range: TimeRange::new(0, 10),
            price_range: PriceRange::new(-10.0, 100.0),
        };
        let t = Transform::from_viewport_with_mode(&vp, YScaleMode::Logarithmic);
        // Falls back to linear since min <= 0
        assert_eq!(t.y_mode(), YScaleMode::Linear);
    }

    #[test]
    fn log_price_y_matches_to_pixel() {
        let t = Transform::from_viewport_with_mode(&log_viewport(), YScaleMode::Logarithmic);
        for price in [10.0, 50.0, 100.0, 500.0, 1000.0] {
            let px = t.to_pixel(0.0, price);
            assert!((t.price_y(price) - px.y).abs() < 1e-9);
        }
    }
}
