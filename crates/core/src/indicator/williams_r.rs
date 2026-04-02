// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use super::{Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle};
use crate::Ohlcv;

/// Williams %R momentum oscillator.
///
/// Range is −100 to 0. Values above −20 indicate overbought conditions;
/// values below −80 indicate oversold conditions.
#[derive(Debug, Clone)]
pub struct WilliamsR {
    /// Lookback period (default 14).
    pub period: usize,
}

impl Default for WilliamsR {
    fn default() -> Self {
        Self { period: 14 }
    }
}

impl Indicator for WilliamsR {
    fn name(&self) -> &'static str {
        "Williams %R"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::SubPanel {
            y_min: -100.0,
            y_max: 0.0,
        }
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let n = data.len();
        let mut values = vec![f64::NAN; n];

        if self.period == 0 || n < self.period {
            return self.build_output(values, n);
        }

        for i in (self.period - 1)..n {
            let start = i + 1 - self.period;
            let window = &data[start..=i];
            let highest_high = window
                .iter()
                .map(|b| b.high)
                .fold(f64::NEG_INFINITY, f64::max);
            let lowest_low = window.iter().map(|b| b.low).fold(f64::INFINITY, f64::min);
            let range = highest_high - lowest_low;
            values[i] = if range < f64::EPSILON {
                -50.0
            } else {
                (highest_high - data[i].close) / range * -100.0
            };
        }

        self.build_output(values, n)
    }
}

impl WilliamsR {
    fn build_output(&self, values: Vec<f64>, n: usize) -> IndicatorOutput {
        IndicatorOutput {
            name: format!("Williams %R({})", self.period),
            placement: self.placement(),
            series: vec![
                IndicatorSeries {
                    name: "%R",
                    values,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "overbought",
                    values: vec![-20.0; n],
                    style_hint: SeriesStyle::HorizontalLine,
                },
                IndicatorSeries {
                    name: "oversold",
                    values: vec![-80.0; n],
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
    fn williams_r_empty_data() {
        let out = WilliamsR::default().compute(&[]);
        assert!(out.series[0].values.is_empty());
    }

    #[test]
    fn williams_r_nan_prefix() {
        let data: Vec<Ohlcv> = (0..20)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = WilliamsR { period: 14 }.compute(&data);
        let v = &out.series[0].values;
        for val in &v[..13] {
            assert!(val.is_nan(), "expected NaN in prefix, got {val}");
        }
        assert!(!v[13].is_nan(), "expected valid value at index 13");
    }

    #[test]
    fn williams_r_series_count_and_placement() {
        let data: Vec<Ohlcv> = (0..20)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = WilliamsR { period: 14 }.compute(&data);
        assert_eq!(out.series.len(), 3);
        assert_eq!(out.series[0].name, "%R");
        assert_eq!(out.series[1].name, "overbought");
        assert_eq!(out.series[2].name, "oversold");
        assert!(
            matches!(out.placement, IndicatorPlacement::SubPanel { y_min, y_max }
                if (y_min - (-100.0)).abs() < f64::EPSILON && (y_max - 0.0).abs() < f64::EPSILON)
        );
    }

    #[test]
    fn williams_r_range_0_to_neg100() {
        // Rising data: close at high → %R near 0
        let data: Vec<Ohlcv> = (0..20)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    100.0 + f64::from(i),
                )
            })
            .collect();
        let out = WilliamsR { period: 14 }.compute(&data);
        let v = &out.series[0].values;
        for val in v[13..].iter() {
            assert!(*val >= -100.0 && *val <= 0.0, "out of range: {val}");
        }
    }

    #[test]
    fn williams_r_close_at_high() {
        // When close == highest_high → %R = 0
        let data: Vec<Ohlcv> = (0..20).map(|_| bar(100.0, 90.0, 100.0)).collect();
        let out = WilliamsR { period: 5 }.compute(&data);
        let v = &out.series[0].values;
        for val in v[4..].iter() {
            assert!(val.abs() < 1e-9, "expected 0, got {val}");
        }
    }

    #[test]
    fn williams_r_close_at_low() {
        // When close == lowest_low → %R = -100
        let data: Vec<Ohlcv> = (0..20).map(|_| bar(100.0, 90.0, 90.0)).collect();
        let out = WilliamsR { period: 5 }.compute(&data);
        let v = &out.series[0].values;
        for val in v[4..].iter() {
            assert!((val - (-100.0)).abs() < 1e-9, "expected -100, got {val}");
        }
    }

    #[test]
    fn williams_r_zero_range_outputs_neg50() {
        // Constant price: range = 0 → -50
        let data: Vec<Ohlcv> = (0..20).map(|_| bar(100.0, 100.0, 100.0)).collect();
        let out = WilliamsR { period: 5 }.compute(&data);
        let v = &out.series[0].values;
        for val in v[4..].iter() {
            assert!((val - (-50.0)).abs() < 1e-9, "expected -50, got {val}");
        }
    }

    #[test]
    fn williams_r_reference_lines() {
        let data: Vec<Ohlcv> = (0..20)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = WilliamsR { period: 14 }.compute(&data);
        assert!((out.series[1].values[0] - (-20.0)).abs() < f64::EPSILON);
        assert!((out.series[2].values[0] - (-80.0)).abs() < f64::EPSILON);
    }
}
