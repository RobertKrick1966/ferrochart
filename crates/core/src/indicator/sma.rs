// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use super::{
    Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle, closes,
    compute_sma,
};
use crate::Ohlcv;

/// Simple Moving Average.
#[derive(Debug, Clone)]
pub struct Sma {
    pub period: usize,
}

impl Indicator for Sma {
    fn name(&self) -> &'static str {
        "SMA"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::Overlay
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let values = compute_sma(&closes(data), self.period);
        IndicatorOutput {
            name: format!("SMA({})", self.period),
            placement: self.placement(),
            series: vec![IndicatorSeries {
                name: "SMA",
                values,
                style_hint: SeriesStyle::Line,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Ohlcv;

    fn bar(close: f64) -> Ohlcv {
        Ohlcv {
            timestamp: 0,
            open: close,
            high: close,
            low: close,
            close,
            volume: 0.0,
        }
    }

    #[test]
    fn sma_basic() {
        let data: Vec<Ohlcv> = [1.0, 2.0, 3.0, 4.0, 5.0].iter().map(|&c| bar(c)).collect();
        let out = Sma { period: 3 }.compute(&data);
        let v = &out.series[0].values;
        assert!(v[0].is_nan());
        assert!(v[1].is_nan());
        assert!((v[2] - 2.0).abs() < 1e-9);
        assert!((v[3] - 3.0).abs() < 1e-9);
        assert!((v[4] - 4.0).abs() < 1e-9);
    }

    #[test]
    fn sma_period_equals_length() {
        let data: Vec<Ohlcv> = [10.0, 20.0, 30.0].iter().map(|&c| bar(c)).collect();
        let out = Sma { period: 3 }.compute(&data);
        let v = &out.series[0].values;
        assert!(v[0].is_nan());
        assert!(v[1].is_nan());
        assert!((v[2] - 20.0).abs() < 1e-9);
    }

    #[test]
    fn sma_period_larger_than_data() {
        let data: Vec<Ohlcv> = [1.0, 2.0].iter().map(|&c| bar(c)).collect();
        let out = Sma { period: 5 }.compute(&data);
        assert!(out.series[0].values.iter().all(|v| v.is_nan()));
    }

    #[test]
    fn sma_empty_data() {
        let out = Sma { period: 3 }.compute(&[]);
        assert!(out.series[0].values.is_empty());
    }

    #[test]
    fn sma_period_one() {
        let data: Vec<Ohlcv> = [5.0, 10.0, 15.0].iter().map(|&c| bar(c)).collect();
        let out = Sma { period: 1 }.compute(&data);
        let v = &out.series[0].values;
        assert!((v[0] - 5.0).abs() < 1e-9);
        assert!((v[1] - 10.0).abs() < 1e-9);
        assert!((v[2] - 15.0).abs() < 1e-9);
    }

    #[test]
    fn sma_placement_is_overlay() {
        assert_eq!(Sma { period: 20 }.placement(), IndicatorPlacement::Overlay);
    }

    #[test]
    fn sma_name_includes_period() {
        let out = Sma { period: 50 }.compute(&[bar(100.0)]);
        assert_eq!(out.name, "SMA(50)");
    }
}
