// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use super::{
    Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle, atr::compute_atr,
};
use crate::Ohlcv;

/// Supertrend overlay indicator.
///
/// Computes a dynamic support/resistance level based on ATR. When price is
/// above the band the trend is bullish (lower band shown); when below, bearish
/// (upper band shown). The indicator is commonly used as a trailing stop or
/// trend-direction filter.
#[derive(Debug, Clone)]
pub struct Supertrend {
    /// ATR lookback period (default 10).
    pub period: usize,
    /// ATR multiplier (default 3.0).
    pub multiplier: f64,
}

impl Default for Supertrend {
    fn default() -> Self {
        Self {
            period: 10,
            multiplier: 3.0,
        }
    }
}

impl Indicator for Supertrend {
    fn name(&self) -> &'static str {
        "Supertrend"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::Overlay
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let n = data.len();
        let mut st_vals = vec![f64::NAN; n];

        if self.period == 0 || n < self.period {
            return IndicatorOutput {
                name: format!("Supertrend({},{})", self.period, self.multiplier),
                placement: self.placement(),
                series: vec![IndicatorSeries {
                    name: "Supertrend",
                    values: st_vals,
                    style_hint: SeriesStyle::Line,
                }],
            };
        }

        let atr = compute_atr(data, self.period);

        // Basic upper/lower bands
        let mut basic_upper = vec![f64::NAN; n];
        let mut basic_lower = vec![f64::NAN; n];
        for i in 0..n {
            if atr[i].is_nan() {
                continue;
            }
            let hl2 = f64::midpoint(data[i].high, data[i].low);
            basic_upper[i] = hl2 + self.multiplier * atr[i];
            basic_lower[i] = hl2 - self.multiplier * atr[i];
        }

        // Find first valid index
        let Some(start) = basic_upper.iter().position(|v| !v.is_nan()) else {
            return IndicatorOutput {
                name: format!("Supertrend({},{})", self.period, self.multiplier),
                placement: self.placement(),
                series: vec![IndicatorSeries {
                    name: "Supertrend",
                    values: st_vals,
                    style_hint: SeriesStyle::Line,
                }],
            };
        };

        let mut final_upper = vec![f64::NAN; n];
        let mut final_lower = vec![f64::NAN; n];
        final_upper[start] = basic_upper[start];
        final_lower[start] = basic_lower[start];

        // Initial direction: bullish if close > basic_upper, else bearish
        // (standard: bullish when close > basic_lower, bear when below basic_upper)
        let mut is_up = data[start].close > basic_lower[start];
        st_vals[start] = if is_up {
            final_lower[start]
        } else {
            final_upper[start]
        };

        for i in (start + 1)..n {
            if basic_upper[i].is_nan() {
                continue;
            }

            // Final upper: tighten when possible (decrease), but reset if price was above
            final_upper[i] =
                if basic_upper[i] < final_upper[i - 1] || data[i - 1].close > final_upper[i - 1] {
                    basic_upper[i]
                } else {
                    final_upper[i - 1]
                };

            // Final lower: tighten when possible (increase), but reset if price was below
            final_lower[i] =
                if basic_lower[i] > final_lower[i - 1] || data[i - 1].close < final_lower[i - 1] {
                    basic_lower[i]
                } else {
                    final_lower[i - 1]
                };

            // Determine direction
            if is_up {
                if data[i].close <= final_upper[i] {
                    // Price crossed below upper band → flip to downtrend
                    is_up = false;
                }
            } else if data[i].close >= final_lower[i] {
                // Price crossed above lower band → flip to uptrend
                is_up = true;
            }

            st_vals[i] = if is_up {
                final_lower[i]
            } else {
                final_upper[i]
            };
        }

        IndicatorOutput {
            name: format!("Supertrend({},{})", self.period, self.multiplier),
            placement: self.placement(),
            series: vec![IndicatorSeries {
                name: "Supertrend",
                values: st_vals,
                style_hint: SeriesStyle::Line,
            }],
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
    fn supertrend_empty_data() {
        let out = Supertrend::default().compute(&[]);
        assert!(out.series[0].values.is_empty());
    }

    #[test]
    fn supertrend_nan_prefix() {
        let data: Vec<Ohlcv> = (0..20)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = Supertrend {
            period: 10,
            multiplier: 3.0,
        }
        .compute(&data);
        let v = &out.series[0].values;
        for val in &v[..9] {
            assert!(val.is_nan(), "expected NaN in prefix, got {val}");
        }
        assert!(!v[9].is_nan(), "expected valid value at index 9");
    }

    #[test]
    fn supertrend_series_count_and_placement() {
        let data: Vec<Ohlcv> = (0..20)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = Supertrend::default().compute(&data);
        assert_eq!(out.series.len(), 1);
        assert_eq!(out.series[0].name, "Supertrend");
        assert_eq!(out.placement, IndicatorPlacement::Overlay);
    }

    #[test]
    fn supertrend_values_are_finite() {
        let data: Vec<Ohlcv> = (0..30)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = Supertrend {
            period: 10,
            multiplier: 3.0,
        }
        .compute(&data);
        let v = &out.series[0].values;
        for val in &v[9..] {
            assert!(val.is_finite(), "expected finite value, got {val}");
        }
    }

    #[test]
    fn supertrend_output_length_matches_data() {
        let data: Vec<Ohlcv> = (0..50)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = Supertrend::default().compute(&data);
        assert_eq!(out.series[0].values.len(), data.len());
    }
}
