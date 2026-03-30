use crate::{Ohlcv, Transform};

/// Pixel coordinates for a single candlestick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandleGeometry {
    /// Center X of the candle.
    pub x: f64,
    /// Top of the body (smaller Y value = higher on screen).
    pub body_top: f64,
    /// Bottom of the body (larger Y value = lower on screen).
    pub body_bottom: f64,
    /// Top of the wick (high price).
    pub wick_top: f64,
    /// Bottom of the wick (low price).
    pub wick_bottom: f64,
    /// Width of the body rectangle.
    pub body_width: f64,
    /// `true` if close >= open (bullish candle).
    pub bullish: bool,
}

impl CandleGeometry {
    /// Compute geometry for a single bar.
    ///
    /// `body_ratio` controls how much of the bar width the body occupies (e.g. 0.8 = 80%).
    #[must_use]
    pub fn from_ohlcv(
        bar: &Ohlcv,
        bar_index: usize,
        transform: &Transform,
        body_ratio: f64,
    ) -> Self {
        let x = transform.bar_x(bar_index);
        let body_width = transform.bar_width() * body_ratio;
        let bullish = bar.close >= bar.open;

        let open_y = transform.price_y(bar.open);
        let close_y = transform.price_y(bar.close);

        // In pixel space, smaller Y is higher on screen.
        let (body_top, body_bottom) = if open_y < close_y {
            (open_y, close_y)
        } else {
            (close_y, open_y)
        };

        let wick_top = transform.price_y(bar.high);
        let wick_bottom = transform.price_y(bar.low);

        Self {
            x,
            body_top,
            body_bottom,
            wick_top,
            wick_bottom,
            body_width,
            bullish,
        }
    }

    /// Compute geometry for a slice of bars.
    #[must_use]
    pub fn compute_all(
        data: &[Ohlcv],
        start_index: usize,
        transform: &Transform,
        body_ratio: f64,
    ) -> Vec<Self> {
        data.iter()
            .enumerate()
            .map(|(i, bar)| Self::from_ohlcv(bar, start_index + i, transform, body_ratio))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PriceRange, Rect, TimeRange, Viewport};

    fn make_transform() -> Transform {
        let vp = Viewport {
            rect: Rect::new(0.0, 0.0, 1000.0, 500.0),
            time_range: TimeRange::new(0, 10),
            price_range: PriceRange::new(90.0, 210.0),
        };
        Transform::from_viewport(&vp)
    }

    fn bullish_bar() -> Ohlcv {
        Ohlcv {
            timestamp: 1,
            open: 100.0,
            high: 120.0,
            low: 95.0,
            close: 115.0,
            volume: 1000.0,
        }
    }

    fn bearish_bar() -> Ohlcv {
        Ohlcv {
            timestamp: 2,
            open: 115.0,
            high: 118.0,
            low: 98.0,
            close: 102.0,
            volume: 1500.0,
        }
    }

    #[test]
    fn bullish_candle_detection() {
        let t = make_transform();
        let c = CandleGeometry::from_ohlcv(&bullish_bar(), 0, &t, 0.8);
        assert!(c.bullish);
    }

    #[test]
    fn bearish_candle_detection() {
        let t = make_transform();
        let c = CandleGeometry::from_ohlcv(&bearish_bar(), 0, &t, 0.8);
        assert!(!c.bullish);
    }

    #[test]
    fn body_top_is_above_body_bottom_in_pixels() {
        let t = make_transform();
        for bar in [bullish_bar(), bearish_bar()] {
            let c = CandleGeometry::from_ohlcv(&bar, 0, &t, 0.8);
            assert!(
                c.body_top <= c.body_bottom,
                "body_top ({}) should be <= body_bottom ({}) in pixel coords",
                c.body_top,
                c.body_bottom
            );
        }
    }

    #[test]
    fn wick_extends_beyond_body() {
        let t = make_transform();
        let c = CandleGeometry::from_ohlcv(&bullish_bar(), 0, &t, 0.8);
        assert!(
            c.wick_top <= c.body_top,
            "wick top ({}) should be <= body top ({}) in pixels",
            c.wick_top,
            c.body_top
        );
        assert!(
            c.wick_bottom >= c.body_bottom,
            "wick bottom ({}) should be >= body bottom ({}) in pixels",
            c.wick_bottom,
            c.body_bottom
        );
    }

    #[test]
    fn body_width_matches_ratio() {
        let t = make_transform();
        let c = CandleGeometry::from_ohlcv(&bullish_bar(), 0, &t, 0.8);
        let expected = t.bar_width() * 0.8;
        assert!((c.body_width - expected).abs() < 1e-9);
    }

    #[test]
    fn compute_all_length_matches_input() {
        let t = make_transform();
        let bars = vec![bullish_bar(), bearish_bar(), bullish_bar()];
        let candles = CandleGeometry::compute_all(&bars, 0, &t, 0.8);
        assert_eq!(candles.len(), 3);
    }

    #[test]
    fn compute_all_empty_input() {
        let t = make_transform();
        let candles = CandleGeometry::compute_all(&[], 0, &t, 0.8);
        assert!(candles.is_empty());
    }

    #[test]
    fn x_positions_increase_with_index() {
        let t = make_transform();
        let bars = vec![bullish_bar(), bearish_bar(), bullish_bar()];
        let candles = CandleGeometry::compute_all(&bars, 0, &t, 0.8);
        for pair in candles.windows(2) {
            assert!(pair[1].x > pair[0].x);
        }
    }

    #[test]
    fn start_index_offset_affects_x() {
        let t = make_transform();
        let bar = bullish_bar();
        let c0 = CandleGeometry::from_ohlcv(&bar, 0, &t, 0.8);
        let c5 = CandleGeometry::from_ohlcv(&bar, 5, &t, 0.8);
        assert!(c5.x > c0.x);
        assert!((c5.x - c0.x - 5.0 * t.bar_width()).abs() < 1e-9);
    }

    #[test]
    fn doji_candle_has_zero_body_height() {
        let t = make_transform();
        let doji = Ohlcv {
            timestamp: 1,
            open: 100.0,
            high: 110.0,
            low: 90.0,
            close: 100.0,
            volume: 500.0,
        };
        let c = CandleGeometry::from_ohlcv(&doji, 0, &t, 0.8);
        assert!(c.bullish); // close == open → bullish
        assert!((c.body_top - c.body_bottom).abs() < 1e-9);
    }
}
