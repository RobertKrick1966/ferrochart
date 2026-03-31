// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use super::{Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle, closes};
use crate::Ohlcv;

/// Relative Strength Index.
#[derive(Debug, Clone)]
pub struct Rsi {
    pub period: usize,
}

impl Indicator for Rsi {
    fn name(&self) -> &'static str {
        "RSI"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::SubPanel {
            y_min: 0.0,
            y_max: 100.0,
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let c = closes(data);
        let n = c.len();
        let mut rsi_values = vec![f64::NAN; n];

        if n <= self.period || self.period == 0 {
            return self.build_output(rsi_values, n);
        }

        // Initial average gain/loss over first `period` changes
        let mut avg_gain = 0.0;
        let mut avg_loss = 0.0;
        for i in 1..=self.period {
            let change = c[i] - c[i - 1];
            if change > 0.0 {
                avg_gain += change;
            } else {
                avg_loss -= change; // make positive
            }
        }
        avg_gain /= self.period as f64;
        avg_loss /= self.period as f64;

        rsi_values[self.period] = rsi_from_avg(avg_gain, avg_loss);

        // Smoothed RSI
        let period_f = self.period as f64;
        for i in (self.period + 1)..n {
            let change = c[i] - c[i - 1];
            let (gain, loss) = if change > 0.0 {
                (change, 0.0)
            } else {
                (0.0, -change)
            };
            avg_gain = (avg_gain * (period_f - 1.0) + gain) / period_f;
            avg_loss = (avg_loss * (period_f - 1.0) + loss) / period_f;
            rsi_values[i] = rsi_from_avg(avg_gain, avg_loss);
        }

        self.build_output(rsi_values, n)
    }
}

impl Rsi {
    fn build_output(&self, rsi_values: Vec<f64>, n: usize) -> IndicatorOutput {
        IndicatorOutput {
            name: format!("RSI({})", self.period),
            placement: self.placement(),
            series: vec![
                IndicatorSeries {
                    name: "RSI",
                    values: rsi_values,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "overbought",
                    values: vec![70.0; n],
                    style_hint: SeriesStyle::HorizontalLine,
                },
                IndicatorSeries {
                    name: "oversold",
                    values: vec![30.0; n],
                    style_hint: SeriesStyle::HorizontalLine,
                },
            ],
        }
    }
}

fn rsi_from_avg(avg_gain: f64, avg_loss: f64) -> f64 {
    if avg_loss < f64::EPSILON {
        100.0
    } else {
        100.0 - 100.0 / (1.0 + avg_gain / avg_loss)
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
        }
    }

    #[test]
    fn rsi_all_gains() {
        let data: Vec<Ohlcv> = (0..20).map(|i| bar(100.0 + f64::from(i))).collect();
        let out = Rsi { period: 14 }.compute(&data);
        let v = &out.series[0].values;
        // All gains → RSI should be 100
        for val in &v[14..] {
            assert!((val - 100.0).abs() < 1e-9, "expected 100, got {val}");
        }
    }

    #[test]
    fn rsi_all_losses() {
        let data: Vec<Ohlcv> = (0..20).map(|i| bar(200.0 - f64::from(i))).collect();
        let out = Rsi { period: 14 }.compute(&data);
        let v = &out.series[0].values;
        // All losses → RSI should be 0
        for val in &v[14..] {
            assert!((val - 0.0).abs() < 1e-9, "expected 0, got {val}");
        }
    }

    #[test]
    fn rsi_in_range() {
        let data: Vec<Ohlcv> = [
            44.34, 44.09, 44.15, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84, 46.08, 45.89, 46.03,
            45.61, 46.28, 46.28, 46.00, 46.03, 46.41, 46.22, 45.64,
        ]
        .iter()
        .map(|&c| bar(c))
        .collect();
        let out = Rsi { period: 14 }.compute(&data);
        let v = &out.series[0].values;
        for val in &v[14..] {
            assert!(*val >= 0.0 && *val <= 100.0, "RSI out of range: {val}");
        }
    }

    #[test]
    fn rsi_first_values_are_nan() {
        let data: Vec<Ohlcv> = (0..20).map(|i| bar(100.0 + f64::from(i % 5))).collect();
        let out = Rsi { period: 14 }.compute(&data);
        let v = &out.series[0].values;
        for val in &v[..14] {
            assert!(val.is_nan());
        }
    }

    #[test]
    fn rsi_empty_data() {
        let out = Rsi { period: 14 }.compute(&[]);
        assert!(out.series[0].values.is_empty());
    }

    #[test]
    fn rsi_has_reference_lines() {
        let data: Vec<Ohlcv> = (0..20).map(|i| bar(100.0 + f64::from(i))).collect();
        let out = Rsi { period: 14 }.compute(&data);
        assert_eq!(out.series.len(), 3);
        assert_eq!(out.series[1].name, "overbought");
        assert_eq!(out.series[2].name, "oversold");
        assert!((out.series[1].values[0] - 70.0).abs() < f64::EPSILON);
        assert!((out.series[2].values[0] - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn rsi_placement_is_sub_panel() {
        let p = Rsi { period: 14 }.placement();
        assert!(
            matches!(p, IndicatorPlacement::SubPanel { y_min, y_max } if (y_min - 0.0).abs() < f64::EPSILON && (y_max - 100.0).abs() < f64::EPSILON)
        );
    }
}
