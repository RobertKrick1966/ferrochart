// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use crate::{Point, PriceRange, Rect, TimeRange};

/// Defines the visible data window and the pixel area it maps to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub rect: Rect,
    pub time_range: TimeRange,
    pub price_range: PriceRange,
}

/// Bidirectional mapping between data space (bar index, price) and pixel space.
///
/// Precomputes scale and offset so repeated `to_pixel` calls are just multiply-add.
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    x_scale: f64,
    y_scale: f64,
    x_offset: f64,
    y_offset: f64,
}

impl Transform {
    /// Build a transform from a [`Viewport`].
    ///
    /// Maps `time_range.start` → left edge of `rect`, `time_range.end - 1` → right edge.
    /// Maps `price_range.max` → top of `rect`, `price_range.min` → bottom.
    #[must_use]
    pub fn from_viewport(vp: &Viewport) -> Self {
        let num_bars = vp.time_range.len();
        let x_scale = if num_bars > 1 {
            vp.rect.width / (num_bars - 1) as f64
        } else {
            vp.rect.width
        };

        let price_span = vp.price_range.span();
        // Negative because pixel Y increases downward, but price increases upward.
        let y_scale = if price_span.abs() > f64::EPSILON {
            -vp.rect.height / price_span
        } else {
            0.0
        };

        let x_offset = vp.rect.x - vp.time_range.start as f64 * x_scale;
        // price_range.max maps to rect.y (top)
        let y_offset = vp.rect.y - vp.price_range.max * y_scale;

        Self {
            x_scale,
            y_scale,
            x_offset,
            y_offset,
        }
    }

    /// Map a (`bar_index`, price) pair to pixel coordinates.
    #[must_use]
    pub fn to_pixel(&self, bar_index: f64, price: f64) -> Point {
        Point {
            x: bar_index * self.x_scale + self.x_offset,
            y: price * self.y_scale + self.y_offset,
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
        let price = if self.y_scale.abs() > f64::EPSILON {
            (pixel.y - self.y_offset) / self.y_scale
        } else {
            0.0
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
        price * self.y_scale + self.y_offset
    }

    /// Width in pixels available per bar (for sizing candle bodies).
    #[must_use]
    pub fn bar_width(&self) -> f64 {
        self.x_scale
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
}
