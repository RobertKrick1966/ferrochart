// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use super::{
    Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle, closes,
    compute_ema,
};
use crate::Ohlcv;

/// Exponential Moving Average.
#[derive(Debug, Clone)]
pub struct Ema {
    /// Lookback period.
    pub period: usize,
}

impl Indicator for Ema {
    fn name(&self) -> &'static str {
        "EMA"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::Overlay
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let values = compute_ema(&closes(data), self.period);
        IndicatorOutput {
            name: format!("EMA({})", self.period),
            placement: self.placement(),
            series: vec![IndicatorSeries {
                name: "EMA",
                values,
                style_hint: SeriesStyle::Line,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(close: f64) -> Ohlcv {
        Ohlcv {
            timestamp: 0,
            open: close,
            high: close,
            low: close,
            close,
            volume: 0.0,
            institutional_ratio: 0.0,
        }
    }

    #[test]
    fn ema_basic() {
        let data: Vec<Ohlcv> = [
            22.27, 22.19, 22.08, 22.17, 22.18, 22.13, 22.23, 22.43, 22.24, 22.29,
        ]
        .iter()
        .map(|&c| bar(c))
        .collect();
        let out = Ema { period: 10 }.compute(&data);
        let v = &out.series[0].values;
        // First 9 are NaN
        for val in &v[..9] {
            assert!(val.is_nan());
        }
        // 10th is SMA seed
        let expected_seed = data.iter().map(|b| b.close).sum::<f64>() / 10.0;
        assert!((v[9] - expected_seed).abs() < 1e-9);
    }

    #[test]
    fn ema_reacts_faster_than_sma() {
        // After a jump, EMA should be closer to the new value than SMA
        let mut prices = vec![100.0; 20];
        prices.push(200.0); // sudden jump
        let data: Vec<Ohlcv> = prices.iter().map(|&c| bar(c)).collect();

        let sma_out = super::super::Sma { period: 20 }.compute(&data);
        let ema_out = Ema { period: 20 }.compute(&data);

        let sma_val = sma_out.series[0].values[20];
        let ema_val = ema_out.series[0].values[20];
        // EMA should be closer to 200 than SMA
        assert!((200.0 - ema_val).abs() < (200.0 - sma_val).abs());
    }

    #[test]
    fn ema_empty_data() {
        let out = Ema { period: 10 }.compute(&[]);
        assert!(out.series[0].values.is_empty());
    }

    #[test]
    fn ema_period_larger_than_data() {
        let data: Vec<Ohlcv> = [1.0, 2.0].iter().map(|&c| bar(c)).collect();
        let out = Ema { period: 5 }.compute(&data);
        assert!(out.series[0].values.iter().all(|v| v.is_nan()));
    }

    #[test]
    fn ema_period_one_equals_close() {
        let data: Vec<Ohlcv> = [5.0, 10.0, 15.0].iter().map(|&c| bar(c)).collect();
        let out = Ema { period: 1 }.compute(&data);
        let v = &out.series[0].values;
        assert!((v[0] - 5.0).abs() < 1e-9);
        assert!((v[1] - 10.0).abs() < 1e-9);
        assert!((v[2] - 15.0).abs() < 1e-9);
    }
}
