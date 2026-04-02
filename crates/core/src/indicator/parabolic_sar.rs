// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use super::{Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle};
use crate::Ohlcv;

/// Parabolic SAR (Stop and Reverse) trailing stop indicator.
///
/// Outputs a SAR price value for each bar. The SAR trails the price
/// and flips sides when the price crosses it, signalling potential
/// trend reversals.
///
/// The first two output values are NaN.
#[derive(Debug, Clone)]
pub struct ParabolicSar {
    /// Acceleration factor increment (default 0.02).
    pub af_step: f64,
    /// Maximum acceleration factor (default 0.20).
    pub af_max: f64,
}

impl Default for ParabolicSar {
    fn default() -> Self {
        Self {
            af_step: 0.02,
            af_max: 0.20,
        }
    }
}

impl Indicator for ParabolicSar {
    fn name(&self) -> &'static str {
        "Parabolic SAR"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::Overlay
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let n = data.len();
        let mut sar_vals = vec![f64::NAN; n];

        if n < 2 {
            return IndicatorOutput {
                name: "SAR".to_string(),
                placement: self.placement(),
                series: vec![IndicatorSeries {
                    name: "SAR",
                    values: sar_vals,
                    style_hint: SeriesStyle::Line,
                }],
            };
        }

        // Initialise: determine trend from first two bars
        let mut is_up = data[1].close > data[0].close;
        let mut sar = if is_up { data[0].low } else { data[0].high };
        let mut ep = if is_up { data[0].high } else { data[0].low };
        let mut af = self.af_step;

        for i in 1..n {
            // Compute new SAR before checking reversal
            let new_sar = sar + af * (ep - sar);

            if is_up {
                // Clamp: SAR must not be above prior two lows
                let clamped_sar = if i >= 2 {
                    new_sar.min(data[i - 1].low).min(data[i - 2].low)
                } else {
                    new_sar.min(data[i - 1].low)
                };

                if data[i].low < clamped_sar {
                    // Reverse to downtrend
                    is_up = false;
                    sar = ep; // SAR jumps to the prior EP
                    ep = data[i].low;
                    af = self.af_step;
                } else {
                    sar = clamped_sar;
                    if data[i].high > ep {
                        ep = data[i].high;
                        af = (af + self.af_step).min(self.af_max);
                    }
                }
            } else {
                // Clamp: SAR must not be below prior two highs
                let clamped_sar = if i >= 2 {
                    new_sar.max(data[i - 1].high).max(data[i - 2].high)
                } else {
                    new_sar.max(data[i - 1].high)
                };

                if data[i].high > clamped_sar {
                    // Reverse to uptrend
                    is_up = true;
                    sar = ep;
                    ep = data[i].high;
                    af = self.af_step;
                } else {
                    sar = clamped_sar;
                    if data[i].low < ep {
                        ep = data[i].low;
                        af = (af + self.af_step).min(self.af_max);
                    }
                }
            }

            sar_vals[i] = sar;
        }

        // First bar is always NaN (no prior bar to reference)
        sar_vals[0] = f64::NAN;

        IndicatorOutput {
            name: "SAR".to_string(),
            placement: self.placement(),
            series: vec![IndicatorSeries {
                name: "SAR",
                values: sar_vals,
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
    fn parabolic_sar_empty_data() {
        let out = ParabolicSar::default().compute(&[]);
        assert!(out.series[0].values.is_empty());
    }

    #[test]
    fn parabolic_sar_single_bar() {
        let data = vec![bar(110.0, 90.0, 100.0)];
        let out = ParabolicSar::default().compute(&data);
        assert!(out.series[0].values[0].is_nan());
    }

    #[test]
    fn parabolic_sar_nan_prefix() {
        let data: Vec<Ohlcv> = (0..20)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = ParabolicSar::default().compute(&data);
        let v = &out.series[0].values;
        assert!(v[0].is_nan(), "index 0 should be NaN");
        assert!(!v[1].is_nan(), "index 1 should be valid");
    }

    #[test]
    fn parabolic_sar_series_count_and_placement() {
        let data: Vec<Ohlcv> = (0..20)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = ParabolicSar::default().compute(&data);
        assert_eq!(out.series.len(), 1);
        assert_eq!(out.series[0].name, "SAR");
        assert_eq!(out.placement, IndicatorPlacement::Overlay);
    }

    #[test]
    fn parabolic_sar_uptrend_sar_below_price() {
        // In a rising market the SAR should stay below the close
        let data: Vec<Ohlcv> = (0..30)
            .map(|i| {
                bar(
                    110.0 + f64::from(i),
                    100.0 + f64::from(i),
                    105.0 + f64::from(i),
                )
            })
            .collect();
        let out = ParabolicSar::default().compute(&data);
        let v = &out.series[0].values;
        // After a few bars of uptrend, SAR should be below close
        for i in 5..30 {
            if !v[i].is_nan() {
                assert!(
                    v[i] < data[i].close,
                    "uptrend SAR should be below close at bar {i}: sar={}, close={}",
                    v[i],
                    data[i].close
                );
            }
        }
    }

    #[test]
    fn parabolic_sar_output_length_matches_data() {
        let data: Vec<Ohlcv> = (0..50)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = ParabolicSar::default().compute(&data);
        assert_eq!(out.series[0].values.len(), data.len());
    }
}
