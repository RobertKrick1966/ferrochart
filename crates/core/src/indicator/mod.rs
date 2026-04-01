// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

//! Technical indicator computations.
//!
//! All indicators implement the [`Indicator`] trait and are pure functions
//! (no I/O, no state between calls).

mod bollinger;
mod ema;
mod macd;
mod rsi;
mod sma;
mod volume_sma;

/// Re-exported Bollinger Bands indicator.
pub use bollinger::BollingerBands;
/// Re-exported Exponential Moving Average indicator.
pub use ema::Ema;
/// Re-exported MACD indicator.
pub use macd::Macd;
/// Re-exported Relative Strength Index indicator.
pub use rsi::Rsi;
/// Re-exported Simple Moving Average indicator.
pub use sma::Sma;
/// Re-exported Volume SMA indicator.
pub use volume_sma::VolumeSma;

use crate::Ohlcv;

/// Where an indicator should be rendered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IndicatorPlacement {
    /// Drawn on top of the price panel (SMA, EMA, Bollinger Bands).
    Overlay,
    /// Own sub-panel with a fixed Y range (e.g. RSI: 0–100).
    SubPanel {
        /// Fixed minimum Y value.
        y_min: f64,
        /// Fixed maximum Y value.
        y_max: f64,
    },
    /// Own sub-panel with auto-scaled Y range (e.g. MACD).
    SubPanelAuto,
}

/// Rendering hint for a series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeriesStyle {
    /// Continuous line connecting data points.
    Line,
    /// Vertical bars drawn from zero.
    Histogram,
    /// Horizontal reference line at a fixed value.
    HorizontalLine,
}

/// A single named output series from an indicator.
#[derive(Debug, Clone)]
pub struct IndicatorSeries {
    /// Display name of the series (e.g. "SMA(20)").
    pub name: &'static str,
    /// Computed values, one per bar; leading entries may be `NAN`.
    pub values: Vec<f64>,
    /// Hint for how the renderer should draw this series.
    pub style_hint: SeriesStyle,
}

/// Complete output of an indicator computation.
#[derive(Debug, Clone)]
pub struct IndicatorOutput {
    /// Human-readable name of the indicator.
    pub name: String,
    /// Where the indicator should be rendered on the chart.
    pub placement: IndicatorPlacement,
    /// One or more data series produced by the indicator.
    pub series: Vec<IndicatorSeries>,
}

impl IndicatorOutput {
    /// Slice all series to the given index range.
    /// Used to extract the visible portion from a full-dataset computation.
    #[must_use]
    pub fn slice(&self, range: std::ops::Range<usize>) -> Self {
        Self {
            name: self.name.clone(),
            placement: self.placement,
            series: self
                .series
                .iter()
                .map(|s| IndicatorSeries {
                    name: s.name,
                    values: if range.end <= s.values.len() {
                        s.values[range.clone()].to_vec()
                    } else {
                        // Handle out-of-range gracefully
                        let end = range.end.min(s.values.len());
                        let start = range.start.min(end);
                        s.values[start..end].to_vec()
                    },
                    style_hint: s.style_hint,
                })
                .collect(),
        }
    }
}

/// Core indicator trait. Stateless — takes data, returns output.
pub trait Indicator {
    /// Returns the indicator's display name.
    fn name(&self) -> &'static str;
    /// Returns where the indicator should be placed on the chart.
    fn placement(&self) -> IndicatorPlacement;
    /// Computes the indicator over the full OHLCV dataset.
    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput;
}

/// Extract closing prices from OHLCV data.
fn closes(data: &[Ohlcv]) -> Vec<f64> {
    data.iter().map(|b| b.close).collect()
}

/// Compute a simple moving average over `values` with the given `period`.
/// Returns a `Vec<f64>` of the same length; first `period - 1` entries are `NAN`.
fn compute_sma(values: &[f64], period: usize) -> Vec<f64> {
    let n = values.len();
    let mut result = vec![f64::NAN; n];
    if period == 0 || period > n {
        return result;
    }

    let mut sum: f64 = values[..period].iter().sum();
    result[period - 1] = sum / period as f64;

    for i in period..n {
        sum += values[i] - values[i - period];
        result[i] = sum / period as f64;
    }
    result
}

/// Compute an exponential moving average over `values` with the given `period`.
/// First `period - 1` entries are `NAN`, entry at `period - 1` is the seed SMA.
fn compute_ema(values: &[f64], period: usize) -> Vec<f64> {
    let n = values.len();
    let mut result = vec![f64::NAN; n];
    if period == 0 || period > n {
        return result;
    }

    let k = 2.0 / (period as f64 + 1.0);
    // Seed with SMA
    let seed: f64 = values[..period].iter().sum::<f64>() / period as f64;
    result[period - 1] = seed;

    for i in period..n {
        result[i] = values[i] * k + result[i - 1] * (1.0 - k);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indicator_output_slice_basic() {
        let output = IndicatorOutput {
            name: "test".to_string(),
            placement: IndicatorPlacement::Overlay,
            series: vec![IndicatorSeries {
                name: "values",
                values: vec![1.0, 2.0, 3.0, 4.0, 5.0],
                style_hint: SeriesStyle::Line,
            }],
        };

        let sliced = output.slice(1..4);
        assert_eq!(sliced.series[0].values, vec![2.0, 3.0, 4.0]);
        assert_eq!(sliced.name, "test");
        assert_eq!(sliced.placement, IndicatorPlacement::Overlay);
    }

    #[test]
    fn indicator_output_slice_full_range() {
        let output = IndicatorOutput {
            name: "test".to_string(),
            placement: IndicatorPlacement::Overlay,
            series: vec![IndicatorSeries {
                name: "v",
                values: vec![10.0, 20.0, 30.0],
                style_hint: SeriesStyle::Line,
            }],
        };
        let sliced = output.slice(0..3);
        assert_eq!(sliced.series[0].values, vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn indicator_output_slice_out_of_range() {
        let output = IndicatorOutput {
            name: "test".to_string(),
            placement: IndicatorPlacement::Overlay,
            series: vec![IndicatorSeries {
                name: "v",
                values: vec![1.0, 2.0],
                style_hint: SeriesStyle::Line,
            }],
        };
        // Range beyond data — should not panic
        let sliced = output.slice(0..10);
        assert_eq!(sliced.series[0].values, vec![1.0, 2.0]);
    }

    #[test]
    fn indicator_output_slice_preserves_nan() {
        let output = IndicatorOutput {
            name: "sma".to_string(),
            placement: IndicatorPlacement::Overlay,
            series: vec![IndicatorSeries {
                name: "SMA",
                values: vec![f64::NAN, f64::NAN, 2.0, 3.0, 4.0],
                style_hint: SeriesStyle::Line,
            }],
        };
        let sliced = output.slice(1..4);
        assert!(sliced.series[0].values[0].is_nan());
        assert!((sliced.series[0].values[1] - 2.0).abs() < f64::EPSILON);
        assert!((sliced.series[0].values[2] - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn indicator_output_slice_multiple_series() {
        let output = IndicatorOutput {
            name: "bb".to_string(),
            placement: IndicatorPlacement::Overlay,
            series: vec![
                IndicatorSeries {
                    name: "upper",
                    values: vec![10.0, 20.0, 30.0],
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "mid",
                    values: vec![8.0, 18.0, 28.0],
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "lower",
                    values: vec![6.0, 16.0, 26.0],
                    style_hint: SeriesStyle::Line,
                },
            ],
        };
        let sliced = output.slice(1..3);
        assert_eq!(sliced.series.len(), 3);
        assert_eq!(sliced.series[0].values, vec![20.0, 30.0]);
        assert_eq!(sliced.series[1].values, vec![18.0, 28.0]);
        assert_eq!(sliced.series[2].values, vec![16.0, 26.0]);
    }

    #[test]
    fn warmup_sma_computed_on_full_data() {
        // Simulate: 200 bars, SMA(20), view bars 0..50
        // SMA should have valid values from index 19 onwards
        let data: Vec<Ohlcv> = (0..200)
            .map(|i| Ohlcv {
                timestamp: 0,
                open: 100.0,
                high: 100.0,
                low: 100.0,
                close: 100.0 + f64::from(i) * 0.1,
                volume: 0.0,
                institutional_ratio: 0.0,
            })
            .collect();

        let sma = Sma { period: 20 };
        let full_output = sma.compute(&data);

        // Full output: first 19 are NaN, rest are valid
        assert!(full_output.series[0].values[18].is_nan());
        assert!(!full_output.series[0].values[19].is_nan());

        // Slice for visible range 0..50
        let visible = full_output.slice(0..50);
        assert_eq!(visible.series[0].values.len(), 50);
        // Index 19 in the slice should have a valid value (warmup from full data)
        assert!(!visible.series[0].values[19].is_nan());

        // Now slice for range 100..150 — all should be valid
        let later = full_output.slice(100..150);
        assert!(later.series[0].values.iter().all(|v| !v.is_nan()));
    }
}
