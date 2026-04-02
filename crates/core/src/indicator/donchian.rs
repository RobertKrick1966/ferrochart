// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use super::{Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle};
use crate::Ohlcv;

/// Donchian Channels — highest high, lowest low, and midpoint over a rolling window.
#[derive(Debug, Clone)]
pub struct Donchian {
    /// Look-back period (default 20).
    pub period: usize,
}

impl Default for Donchian {
    fn default() -> Self {
        Self { period: 20 }
    }
}

impl Indicator for Donchian {
    fn name(&self) -> &'static str {
        "Donchian"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::Overlay
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let n = data.len();
        let mut upper = vec![f64::NAN; n];
        let mut mid = vec![f64::NAN; n];
        let mut lower = vec![f64::NAN; n];

        if self.period == 0 || n < self.period {
            return self.build_output(upper, mid, lower);
        }

        for i in (self.period - 1)..n {
            let start = i + 1 - self.period;
            let high = data[start..=i]
                .iter()
                .map(|b| b.high)
                .fold(f64::NEG_INFINITY, f64::max);
            let low = data[start..=i]
                .iter()
                .map(|b| b.low)
                .fold(f64::INFINITY, f64::min);
            upper[i] = high;
            lower[i] = low;
            mid[i] = f64::midpoint(high, low);
        }

        self.build_output(upper, mid, lower)
    }
}

impl Donchian {
    fn build_output(
        &self,
        upper: Vec<f64>,
        mid: Vec<f64>,
        lower: Vec<f64>,
    ) -> IndicatorOutput {
        IndicatorOutput {
            name: format!("Donchian({})", self.period),
            placement: self.placement(),
            series: vec![
                IndicatorSeries {
                    name: "Upper",
                    values: upper,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "Mid",
                    values: mid,
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

    fn bar(high: f64, low: f64) -> Ohlcv {
        Ohlcv {
            timestamp: 0,
            open: f64::midpoint(high, low),
            high,
            low,
            close: f64::midpoint(high, low),
            volume: 0.0,
            institutional_ratio: 0.0,
        }
    }

    #[test]
    fn donchian_empty_data() {
        let out = Donchian::default().compute(&[]);
        assert!(out.series[0].values.is_empty());
    }

    #[test]
    fn donchian_nan_prefix() {
        let data: Vec<Ohlcv> = (0..25).map(|i| bar(100.0 + f64::from(i), 90.0 + f64::from(i))).collect();
        let out = Donchian { period: 5 }.compute(&data);
        let upper = &out.series[0].values;
        for val in &upper[..4] {
            assert!(val.is_nan());
        }
        assert!(!upper[4].is_nan());
    }

    #[test]
    fn donchian_series_count_and_names() {
        let data: Vec<Ohlcv> = (0..25).map(|i| bar(100.0 + f64::from(i), 90.0 + f64::from(i))).collect();
        let out = Donchian::default().compute(&data);
        assert_eq!(out.series.len(), 3);
        assert_eq!(out.series[0].name, "Upper");
        assert_eq!(out.series[1].name, "Mid");
        assert_eq!(out.series[2].name, "Lower");
    }

    #[test]
    fn donchian_known_values() {
        // Bars: highs 10,20,30,40,50; lows 1,2,3,4,5 — period 3
        let data = vec![
            bar(10.0, 1.0),
            bar(20.0, 2.0),
            bar(30.0, 3.0),
            bar(40.0, 4.0),
            bar(50.0, 5.0),
        ];
        let out = Donchian { period: 3 }.compute(&data);
        let upper = &out.series[0].values;
        let lower = &out.series[2].values;
        let mid = &out.series[1].values;

        // index 2: window [0..=2]: high=30, low=1
        assert!((upper[2] - 30.0).abs() < 1e-9);
        assert!((lower[2] - 1.0).abs() < 1e-9);
        assert!((mid[2] - 15.5).abs() < 1e-9);

        // index 4: window [2..=4]: high=50, low=3
        assert!((upper[4] - 50.0).abs() < 1e-9);
        assert!((lower[4] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn donchian_upper_always_ge_lower() {
        let data: Vec<Ohlcv> = (0..30)
            .map(|i| {
                let h = 100.0 + f64::from(i % 7) * 3.0;
                let l = h - 5.0;
                bar(h, l)
            })
            .collect();
        let out = Donchian { period: 5 }.compute(&data);
        let upper = &out.series[0].values;
        let lower = &out.series[2].values;
        for i in 0..data.len() {
            if !upper[i].is_nan() {
                assert!(upper[i] >= lower[i]);
            }
        }
    }

    #[test]
    fn donchian_placement_is_overlay() {
        assert_eq!(Donchian::default().placement(), IndicatorPlacement::Overlay);
    }
}
