// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use super::{
    Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle, closes,
    compute_sma,
};
use crate::Ohlcv;

/// Bollinger Bands (middle = SMA, upper/lower = SMA ± `std_dev` × σ).
#[derive(Debug, Clone)]
pub struct BollingerBands {
    pub period: usize,
    pub std_dev: f64,
}

impl Indicator for BollingerBands {
    fn name(&self) -> &'static str {
        "Bollinger"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::Overlay
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let c = closes(data);
        let middle = compute_sma(&c, self.period);
        let n = c.len();
        let mut upper = vec![f64::NAN; n];
        let mut lower = vec![f64::NAN; n];

        for i in (self.period.saturating_sub(1))..n {
            if middle[i].is_nan() {
                continue;
            }
            let start = i + 1 - self.period;
            let slice = &c[start..=i];
            let mean = middle[i];
            let variance =
                slice.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / self.period as f64;
            let sd = variance.sqrt() * self.std_dev;
            upper[i] = mean + sd;
            lower[i] = mean - sd;
        }

        IndicatorOutput {
            name: format!("BB({},{})", self.period, self.std_dev),
            placement: self.placement(),
            series: vec![
                IndicatorSeries {
                    name: "upper",
                    values: upper,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "middle",
                    values: middle,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "lower",
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

    fn bar(close: f64) -> Ohlcv {
        Ohlcv {
            timestamp: 0,
            open: close,
            high: close,
            low: close,
            close,
            volume: 0.0,
            institutional_ratio: 0.0,
        }
    }

    #[test]
    fn bollinger_basic() {
        let data: Vec<Ohlcv> = [20.0, 22.0, 24.0, 23.0, 21.0]
            .iter()
            .map(|&c| bar(c))
            .collect();
        let out = BollingerBands {
            period: 3,
            std_dev: 2.0,
        }
        .compute(&data);

        assert_eq!(out.series.len(), 3);
        let upper = &out.series[0].values;
        let middle = &out.series[1].values;
        let lower = &out.series[2].values;

        // First 2 are NaN
        assert!(upper[0].is_nan());
        assert!(upper[1].is_nan());

        // At index 2: SMA(20,22,24) = 22.0
        assert!((middle[2] - 22.0).abs() < 1e-9);
        // Upper > middle > lower
        assert!(upper[2] > middle[2]);
        assert!(lower[2] < middle[2]);
    }

    #[test]
    fn bollinger_constant_prices_zero_bandwidth() {
        let data: Vec<Ohlcv> = [100.0; 5].iter().map(|&c| bar(c)).collect();
        let out = BollingerBands {
            period: 3,
            std_dev: 2.0,
        }
        .compute(&data);

        let upper = &out.series[0].values;
        let middle = &out.series[1].values;
        let lower = &out.series[2].values;

        // With constant prices, std dev = 0, so upper == middle == lower
        for i in 2..5 {
            assert!((upper[i] - 100.0).abs() < 1e-9);
            assert!((middle[i] - 100.0).abs() < 1e-9);
            assert!((lower[i] - 100.0).abs() < 1e-9);
        }
    }

    #[test]
    fn bollinger_upper_always_above_lower() {
        let data: Vec<Ohlcv> = [10.0, 12.0, 8.0, 15.0, 9.0, 13.0, 11.0]
            .iter()
            .map(|&c| bar(c))
            .collect();
        let out = BollingerBands {
            period: 3,
            std_dev: 2.0,
        }
        .compute(&data);

        let upper = &out.series[0].values;
        let lower = &out.series[2].values;
        for i in 0..data.len() {
            if !upper[i].is_nan() {
                assert!(upper[i] >= lower[i]);
            }
        }
    }

    #[test]
    fn bollinger_empty_data() {
        let out = BollingerBands {
            period: 20,
            std_dev: 2.0,
        }
        .compute(&[]);
        assert!(out.series[0].values.is_empty());
    }
}
