// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use super::{
    Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle, compute_sma,
};
use crate::Ohlcv;

/// Stochastic Oscillator — measures close relative to the recent high-low range.
#[derive(Debug, Clone)]
pub struct Stochastic {
    /// Look-back period for `%K` (default 14).
    pub k_period: usize,
    /// Smoothing period for `%D` (default 3).
    pub d_period: usize,
}

impl Default for Stochastic {
    fn default() -> Self {
        Self {
            k_period: 14,
            d_period: 3,
        }
    }
}

impl Indicator for Stochastic {
    fn name(&self) -> &'static str {
        "Stochastic"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::SubPanel {
            y_min: 0.0,
            y_max: 100.0,
        }
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let n = data.len();
        let mut k_values = vec![f64::NAN; n];

        if self.k_period == 0 || n < self.k_period {
            let d_values = vec![f64::NAN; n];
            return self.build_output(k_values, d_values, n);
        }

        for i in (self.k_period - 1)..n {
            let start = i + 1 - self.k_period;
            let highest_high = data[start..=i]
                .iter()
                .map(|b| b.high)
                .fold(f64::NEG_INFINITY, f64::max);
            let lowest_low = data[start..=i]
                .iter()
                .map(|b| b.low)
                .fold(f64::INFINITY, f64::min);

            let range = highest_high - lowest_low;
            k_values[i] = if range < f64::EPSILON {
                50.0 // divide-by-zero guard
            } else {
                (data[i].close - lowest_low) / range * 100.0
            };
        }

        let d_values = compute_sma(&k_values, self.d_period);

        self.build_output(k_values, d_values, n)
    }
}

impl Stochastic {
    fn build_output(&self, k_values: Vec<f64>, d_values: Vec<f64>, n: usize) -> IndicatorOutput {
        IndicatorOutput {
            name: format!("Stoch({},{})", self.k_period, self.d_period),
            placement: self.placement(),
            series: vec![
                IndicatorSeries {
                    name: "%K",
                    values: k_values,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "%D",
                    values: d_values,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "overbought",
                    values: vec![80.0; n],
                    style_hint: SeriesStyle::HorizontalLine,
                },
                IndicatorSeries {
                    name: "oversold",
                    values: vec![20.0; n],
                    style_hint: SeriesStyle::HorizontalLine,
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
    fn stochastic_empty_data() {
        let out = Stochastic::default().compute(&[]);
        assert!(out.series[0].values.is_empty());
    }

    #[test]
    fn stochastic_nan_prefix() {
        let data: Vec<Ohlcv> = (0..20)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = Stochastic {
            k_period: 5,
            d_period: 3,
        }
        .compute(&data);
        let k = &out.series[0].values;
        let d = &out.series[1].values;
        // First k_period-1 values of %K are NaN
        for val in &k[..4] {
            assert!(val.is_nan());
        }
        assert!(!k[4].is_nan());
        // First k_period-1+d_period-1 values of %D are NaN
        for val in &d[..6] {
            assert!(val.is_nan());
        }
    }

    #[test]
    fn stochastic_series_count_and_names() {
        let data: Vec<Ohlcv> = (0..20)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = Stochastic::default().compute(&data);
        assert_eq!(out.series.len(), 4);
        assert_eq!(out.series[0].name, "%K");
        assert_eq!(out.series[1].name, "%D");
        assert_eq!(out.series[2].name, "overbought");
        assert_eq!(out.series[3].name, "oversold");
    }

    #[test]
    fn stochastic_known_value_all_up() {
        // Steadily rising close at the top of the range → %K near 100
        let data: Vec<Ohlcv> = (0..20)
            .map(|i| {
                let v = f64::from(i);
                bar(100.0 + v, 90.0 + v, 100.0 + v) // close == high
            })
            .collect();
        let out = Stochastic {
            k_period: 5,
            d_period: 3,
        }
        .compute(&data);
        let k = &out.series[0].values;
        for val in &k[4..] {
            assert!((val - 100.0).abs() < 1e-9, "expected %K=100, got {val}");
        }
    }

    #[test]
    fn stochastic_zero_range_returns_50() {
        // All bars identical → range = 0, guard returns 50
        let data: Vec<Ohlcv> = (0..10).map(|_| bar(100.0, 100.0, 100.0)).collect();
        let out = Stochastic {
            k_period: 5,
            d_period: 3,
        }
        .compute(&data);
        let k = &out.series[0].values;
        for val in &k[4..] {
            assert!((val - 50.0).abs() < 1e-9);
        }
    }

    #[test]
    fn stochastic_reference_lines() {
        let data: Vec<Ohlcv> = (0..20)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = Stochastic::default().compute(&data);
        assert!((out.series[2].values[0] - 80.0).abs() < f64::EPSILON);
        assert!((out.series[3].values[0] - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stochastic_placement() {
        let p = Stochastic::default().placement();
        assert!(matches!(p, IndicatorPlacement::SubPanel { y_min, y_max }
                if (y_min - 0.0).abs() < f64::EPSILON && (y_max - 100.0).abs() < f64::EPSILON));
    }
}
