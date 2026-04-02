// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use super::{Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle};
use crate::Ohlcv;

/// Commodity Channel Index (CCI).
///
/// Measures deviation of the typical price from its simple moving average,
/// normalised by mean absolute deviation. Values above +100 suggest overbought;
/// below −100 suggest oversold.
#[derive(Debug, Clone)]
pub struct Cci {
    /// Lookback period (default 20).
    pub period: usize,
}

impl Default for Cci {
    fn default() -> Self {
        Self { period: 20 }
    }
}

impl Indicator for Cci {
    fn name(&self) -> &'static str {
        "CCI"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::SubPanelAuto
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let n = data.len();
        let mut values = vec![f64::NAN; n];

        if self.period == 0 || n < self.period {
            return self.build_output(values, n);
        }

        // Typical prices
        let tp: Vec<f64> = data
            .iter()
            .map(|b| (b.high + b.low + b.close) / 3.0)
            .collect();

        for i in (self.period - 1)..n {
            let start = i + 1 - self.period;
            let window = &tp[start..=i];

            // SMA of typical price
            let sma_tp: f64 = window.iter().sum::<f64>() / self.period as f64;

            // Mean absolute deviation
            let mean_dev: f64 =
                window.iter().map(|&v| (v - sma_tp).abs()).sum::<f64>() / self.period as f64;

            values[i] = if mean_dev < f64::EPSILON {
                0.0
            } else {
                (tp[i] - sma_tp) / (0.015 * mean_dev)
            };
        }

        self.build_output(values, n)
    }
}

impl Cci {
    fn build_output(&self, values: Vec<f64>, n: usize) -> IndicatorOutput {
        IndicatorOutput {
            name: format!("CCI({})", self.period),
            placement: self.placement(),
            series: vec![
                IndicatorSeries {
                    name: "CCI",
                    values,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "overbought",
                    values: vec![100.0; n],
                    style_hint: SeriesStyle::HorizontalLine,
                },
                IndicatorSeries {
                    name: "oversold",
                    values: vec![-100.0; n],
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
    fn cci_empty_data() {
        let out = Cci::default().compute(&[]);
        assert!(out.series[0].values.is_empty());
    }

    #[test]
    fn cci_nan_prefix() {
        let data: Vec<Ohlcv> = (0..30)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = Cci { period: 20 }.compute(&data);
        let v = &out.series[0].values;
        for val in &v[..19] {
            assert!(val.is_nan(), "expected NaN in prefix, got {val}");
        }
        assert!(!v[19].is_nan(), "expected valid value at index 19");
    }

    #[test]
    fn cci_series_count_and_placement() {
        let data: Vec<Ohlcv> = (0..30)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = Cci { period: 20 }.compute(&data);
        assert_eq!(out.series.len(), 3);
        assert_eq!(out.series[0].name, "CCI");
        assert_eq!(out.series[1].name, "overbought");
        assert_eq!(out.series[2].name, "oversold");
        assert_eq!(out.placement, IndicatorPlacement::SubPanelAuto);
    }

    #[test]
    fn cci_constant_prices_outputs_zero() {
        // No deviation → CCI = 0
        let data: Vec<Ohlcv> = (0..25).map(|_| bar(100.0, 100.0, 100.0)).collect();
        let out = Cci { period: 20 }.compute(&data);
        let v = &out.series[0].values;
        for val in v[19..].iter() {
            assert!(val.abs() < 1e-9, "expected 0, got {val}");
        }
    }

    #[test]
    fn cci_reference_lines() {
        let data: Vec<Ohlcv> = (0..30)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = Cci { period: 20 }.compute(&data);
        assert!((out.series[1].values[0] - 100.0).abs() < f64::EPSILON);
        assert!((out.series[2].values[0] - (-100.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn cci_numeric_result() {
        // Known values: trend up with constant step
        let data: Vec<Ohlcv> = (0..25)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    99.0 + f64::from(i),
                    99.5 + f64::from(i),
                )
            })
            .collect();
        let out = Cci { period: 5 }.compute(&data);
        let v = &out.series[0].values;
        // For a linearly rising series the CCI is non-NaN and finite
        for val in v[4..].iter() {
            assert!(val.is_finite(), "expected finite value, got {val}");
        }
    }
}
