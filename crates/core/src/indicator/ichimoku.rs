// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use super::{Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle};
use crate::Ohlcv;

/// Ichimoku Kinkō Hyō — five-line trend and momentum system (simplified).
///
/// Outputs: Tenkan-sen, Kijun-sen, Senkou Span A, Senkou Span B, Chikou Span.
/// Cloud fills (between Senkou A and B) and time-shift offsets are not applied;
/// all values are output at the current bar index for a static chart context.
#[derive(Debug, Clone)]
pub struct Ichimoku {
    /// Conversion line period (default 9).
    pub tenkan_period: usize,
    /// Base line period (default 26).
    pub kijun_period: usize,
    /// Senkou Span B period (default 52).
    pub senkou_b_period: usize,
}

impl Default for Ichimoku {
    fn default() -> Self {
        Self {
            tenkan_period: 9,
            kijun_period: 26,
            senkou_b_period: 52,
        }
    }
}

impl Indicator for Ichimoku {
    fn name(&self) -> &'static str {
        "Ichimoku"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::Overlay
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let n = data.len();
        let mut tenkan = vec![f64::NAN; n];
        let mut kijun = vec![f64::NAN; n];
        let mut senkou_a = vec![f64::NAN; n];
        let mut senkou_b = vec![f64::NAN; n];
        let mut chikou = vec![f64::NAN; n];

        // Tenkan-sen: (highest_high + lowest_low) / 2 over tenkan_period
        compute_midpoint(&mut tenkan, data, self.tenkan_period);
        // Kijun-sen: same over kijun_period
        compute_midpoint(&mut kijun, data, self.kijun_period);

        // Senkou Span A: (tenkan + kijun) / 2
        for i in 0..n {
            if !tenkan[i].is_nan() && !kijun[i].is_nan() {
                senkou_a[i] = f64::midpoint(tenkan[i], kijun[i]);
            }
        }

        // Senkou Span B: (highest_high + lowest_low) / 2 over senkou_b_period
        compute_midpoint(&mut senkou_b, data, self.senkou_b_period);

        // Chikou Span: close[i + kijun_period] placed at bar i (look-back shift)
        // For static output we output close[i] at index i (no visual shift)
        for i in 0..n {
            chikou[i] = data[i].close;
        }

        IndicatorOutput {
            name: format!(
                "Ichimoku({},{},{})",
                self.tenkan_period, self.kijun_period, self.senkou_b_period
            ),
            placement: self.placement(),
            series: vec![
                IndicatorSeries {
                    name: "Tenkan",
                    values: tenkan,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "Kijun",
                    values: kijun,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "Senkou A",
                    values: senkou_a,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "Senkou B",
                    values: senkou_b,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "Chikou",
                    values: chikou,
                    style_hint: SeriesStyle::Line,
                },
            ],
        }
    }
}

/// Fills `out[i]` with `(highest_high + lowest_low) / 2` over a rolling `period` window.
fn compute_midpoint(out: &mut [f64], data: &[Ohlcv], period: usize) {
    let n = data.len();
    if period == 0 || n < period {
        return;
    }
    for i in (period - 1)..n {
        let start = i + 1 - period;
        let window = &data[start..=i];
        let hh = window
            .iter()
            .map(|b| b.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let ll = window.iter().map(|b| b.low).fold(f64::INFINITY, f64::min);
        out[i] = f64::midpoint(hh, ll);
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
    fn ichimoku_empty_data() {
        let out = Ichimoku::default().compute(&[]);
        assert_eq!(out.series.len(), 5);
        for s in &out.series {
            assert!(s.values.is_empty());
        }
    }

    #[test]
    fn ichimoku_series_count_and_placement() {
        let data: Vec<Ohlcv> = (0..60)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = Ichimoku::default().compute(&data);
        assert_eq!(out.series.len(), 5);
        assert_eq!(out.series[0].name, "Tenkan");
        assert_eq!(out.series[1].name, "Kijun");
        assert_eq!(out.series[2].name, "Senkou A");
        assert_eq!(out.series[3].name, "Senkou B");
        assert_eq!(out.series[4].name, "Chikou");
        assert_eq!(out.placement, IndicatorPlacement::Overlay);
    }

    #[test]
    fn ichimoku_tenkan_nan_prefix() {
        let data: Vec<Ohlcv> = (0..20)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = Ichimoku {
            tenkan_period: 9,
            kijun_period: 9,
            senkou_b_period: 9,
        }
        .compute(&data);
        let tenkan = &out.series[0].values;
        for val in &tenkan[..8] {
            assert!(val.is_nan(), "expected NaN, got {val}");
        }
        assert!(!tenkan[8].is_nan(), "expected valid value at index 8");
    }

    #[test]
    fn ichimoku_chikou_no_nan() {
        // Chikou = close at each bar, no NaN
        let data: Vec<Ohlcv> = (0..20)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = Ichimoku {
            tenkan_period: 9,
            kijun_period: 9,
            senkou_b_period: 9,
        }
        .compute(&data);
        let chikou = &out.series[4].values;
        for (i, val) in chikou.iter().enumerate() {
            assert!(!val.is_nan(), "chikou NaN at index {i}");
            assert!((val - data[i].close).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn ichimoku_senkou_a_is_midpoint_of_tenkan_kijun() {
        let data: Vec<Ohlcv> = (0..60)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    90.0 + f64::from(i),
                    95.0 + f64::from(i),
                )
            })
            .collect();
        let out = Ichimoku::default().compute(&data);
        let tenkan = &out.series[0].values;
        let kijun = &out.series[1].values;
        let senkou_a = &out.series[2].values;
        for i in 0..60 {
            if !tenkan[i].is_nan() && !kijun[i].is_nan() {
                let expected = (tenkan[i] + kijun[i]) / 2.0;
                assert!(
                    (senkou_a[i] - expected).abs() < 1e-9,
                    "senkou_a[{i}] mismatch: {} vs {}",
                    senkou_a[i],
                    expected
                );
            }
        }
    }

    #[test]
    fn ichimoku_numeric_midpoint() {
        // Constant range: tenkan = (high+low)/2 = 95.0
        let data: Vec<Ohlcv> = (0..15).map(|_| bar(100.0, 90.0, 95.0)).collect();
        let out = Ichimoku {
            tenkan_period: 9,
            kijun_period: 9,
            senkou_b_period: 9,
        }
        .compute(&data);
        let tenkan = &out.series[0].values;
        for val in tenkan[8..].iter() {
            assert!((val - 95.0).abs() < 1e-9, "expected 95.0, got {val}");
        }
    }
}
