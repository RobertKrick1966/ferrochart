// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use super::atr::compute_atr;
use super::{
    Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle, closes,
    compute_ema,
};
use crate::Ohlcv;

/// Keltner Channels — EMA-based envelope using ATR as the width multiplier.
#[derive(Debug, Clone)]
pub struct Keltner {
    /// Period for the middle EMA (default 20).
    pub ema_period: usize,
    /// Period for the ATR calculation (default 10).
    pub atr_period: usize,
    /// ATR multiplier for the upper/lower bands (default 2.0).
    pub multiplier: f64,
}

impl Default for Keltner {
    fn default() -> Self {
        Self {
            ema_period: 20,
            atr_period: 10,
            multiplier: 2.0,
        }
    }
}

impl Indicator for Keltner {
    fn name(&self) -> &'static str {
        "Keltner"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::Overlay
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let n = data.len();
        let c = closes(data);
        let middle = compute_ema(&c, self.ema_period);
        let atr = compute_atr(data, self.atr_period);

        let mut upper = vec![f64::NAN; n];
        let mut lower = vec![f64::NAN; n];

        for i in 0..n {
            if !middle[i].is_nan() && !atr[i].is_nan() {
                upper[i] = middle[i] + self.multiplier * atr[i];
                lower[i] = middle[i] - self.multiplier * atr[i];
            }
        }

        IndicatorOutput {
            name: format!(
                "Keltner({},{},{})",
                self.ema_period, self.atr_period, self.multiplier
            ),
            placement: self.placement(),
            series: vec![
                IndicatorSeries {
                    name: "Upper",
                    values: upper,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "Mid",
                    values: middle,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "Lower",
                    values: lower,
                    style_hint: SeriesStyle::Line,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(high: f64, low: f64, close: f64) -> Ohlcv {
        Ohlcv {
            timestamp: 0,
            open: close,
            high,
            low,
            close,
            volume: 0.0,
            institutional_ratio: 0.0,
        }
    }

    #[test]
    fn keltner_empty_data() {
        let out = Keltner::default().compute(&[]);
        assert!(out.series[0].values.is_empty());
    }

    #[test]
    fn keltner_nan_prefix() {
        let data: Vec<Ohlcv> = (0..30)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = Keltner {
            ema_period: 10,
            atr_period: 5,
            multiplier: 2.0,
        }
        .compute(&data);
        let upper = &out.series[0].values;
        // Leading values must be NaN (until both EMA and ATR have warmed up)
        assert!(upper[0].is_nan());
    }

    #[test]
    fn keltner_series_count_and_names() {
        let data: Vec<Ohlcv> = (0..30)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = Keltner::default().compute(&data);
        assert_eq!(out.series.len(), 3);
        assert_eq!(out.series[0].name, "Upper");
        assert_eq!(out.series[1].name, "Mid");
        assert_eq!(out.series[2].name, "Lower");
    }

    #[test]
    fn keltner_upper_always_above_lower() {
        let data: Vec<Ohlcv> = (0..40)
            .map(|i| {
                let c = 100.0 + f64::from(i % 10) * 2.0;
                bar(c + 5.0, c - 5.0, c)
            })
            .collect();
        let out = Keltner {
            ema_period: 10,
            atr_period: 5,
            multiplier: 2.0,
        }
        .compute(&data);
        let upper = &out.series[0].values;
        let lower = &out.series[2].values;
        for i in 0..data.len() {
            if !upper[i].is_nan() {
                assert!(
                    upper[i] > lower[i],
                    "upper must be above lower at index {i}"
                );
            }
        }
    }

    #[test]
    fn keltner_constant_bars_symmetric() {
        // Constant price: EMA = price, ATR = 0 → upper = mid = lower
        let data: Vec<Ohlcv> = (0..30).map(|_| bar(100.0, 100.0, 100.0)).collect();
        let out = Keltner {
            ema_period: 5,
            atr_period: 5,
            multiplier: 2.0,
        }
        .compute(&data);
        let upper = &out.series[0].values;
        let mid = &out.series[1].values;
        let lower = &out.series[2].values;
        for i in 0..data.len() {
            if !upper[i].is_nan() {
                assert!((upper[i] - mid[i]).abs() < 1e-9);
                assert!((lower[i] - mid[i]).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn keltner_placement_is_overlay() {
        assert_eq!(Keltner::default().placement(), IndicatorPlacement::Overlay);
    }
}
