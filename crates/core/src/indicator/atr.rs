// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use super::{Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle};
use crate::Ohlcv;

/// Average True Range — measures market volatility using Wilder's smoothing.
#[derive(Debug, Clone)]
pub struct Atr {
    /// Lookback period (default 14).
    pub period: usize,
}

impl Default for Atr {
    fn default() -> Self {
        Self { period: 14 }
    }
}

impl Indicator for Atr {
    fn name(&self) -> &'static str {
        "ATR"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::SubPanelAuto
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let values = compute_atr(data, self.period);
        IndicatorOutput {
            name: format!("ATR({})", self.period),
            placement: self.placement(),
            series: vec![IndicatorSeries {
                name: "ATR",
                values,
                style_hint: SeriesStyle::Line,
            }],
        }
    }
}

/// Compute ATR values for the given OHLCV data and period.
///
/// - `TR[0]  = high[0] - low[0]`
/// - `TR[i]  = max(high[i]-low[i], |high[i]-prev_close|, |low[i]-prev_close|)` for `i > 0`
/// - Seed ATR = SMA of first `period` TR values.
/// - `ATR[i] = (ATR[i-1] * (period-1) + TR[i]) / period`  (Wilder's smoothing)
///
/// The first `period - 1` output values are `NAN`.
#[must_use]
pub fn compute_atr(data: &[Ohlcv], period: usize) -> Vec<f64> {
    let n = data.len();
    let mut result = vec![f64::NAN; n];

    if period == 0 || n < period {
        return result;
    }

    // True Range
    let mut tr = Vec::with_capacity(n);
    tr.push(data[0].high - data[0].low);
    for i in 1..n {
        let prev_close = data[i - 1].close;
        let hl = data[i].high - data[i].low;
        let hc = (data[i].high - prev_close).abs();
        let lc = (data[i].low - prev_close).abs();
        tr.push(hl.max(hc).max(lc));
    }

    // Seed: SMA of first `period` TR values
    let seed: f64 = tr[..period].iter().sum::<f64>() / period as f64;
    result[period - 1] = seed;

    // Wilder's smoothing
    let period_f = period as f64;
    for i in period..n {
        result[i] = (result[i - 1] * (period_f - 1.0) + tr[i]) / period_f;
    }

    result
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
    fn atr_empty_data() {
        let out = Atr::default().compute(&[]);
        assert!(out.series[0].values.is_empty());
    }

    #[test]
    fn atr_nan_prefix() {
        let data: Vec<Ohlcv> = (0..20)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = Atr { period: 14 }.compute(&data);
        let v = &out.series[0].values;
        for val in &v[..13] {
            assert!(val.is_nan(), "expected NaN at prefix");
        }
        assert!(!v[13].is_nan(), "expected valid value at index 13");
    }

    #[test]
    fn atr_output_series_count() {
        let data: Vec<Ohlcv> = (0..20)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = Atr { period: 14 }.compute(&data);
        assert_eq!(out.series.len(), 1);
        assert_eq!(out.series[0].name, "ATR");
        assert_eq!(out.series[0].values.len(), data.len());
    }

    #[test]
    fn atr_constant_bars_known_value() {
        // Constant prices: each TR = high-low = 10.0 → ATR = 10.0
        let data: Vec<Ohlcv> = (0..20).map(|_| bar(110.0, 100.0, 105.0)).collect();
        let out = Atr { period: 5 }.compute(&data);
        let v = &out.series[0].values;
        for val in &v[4..] {
            assert!((val - 10.0).abs() < 1e-9, "expected ATR=10, got {val}");
        }
    }

    #[test]
    fn atr_placement_is_sub_panel_auto() {
        assert_eq!(Atr::default().placement(), IndicatorPlacement::SubPanelAuto);
    }

    #[test]
    fn compute_atr_public_function_matches_indicator() {
        let data: Vec<Ohlcv> = (0..20)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let ind_values = Atr { period: 10 }.compute(&data).series[0].values.clone();
        let free_values = compute_atr(&data, 10);
        assert_eq!(ind_values.len(), free_values.len());
        for (a, b) in ind_values.iter().zip(free_values.iter()) {
            if a.is_nan() {
                assert!(b.is_nan());
            } else {
                assert!((a - b).abs() < 1e-9);
            }
        }
    }
}
