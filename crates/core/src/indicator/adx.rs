// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use super::{Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle};
use crate::Ohlcv;

/// Average Directional Index (ADX / DMI) using Wilder's smoothing.
///
/// Outputs three series: ADX (trend strength), +DI (upward pressure) and
/// −DI (downward pressure). A `HorizontalLine` at 25 marks the conventional
/// trend-strength threshold.
///
/// The first `2 × period` output values are NaN.
#[derive(Debug, Clone)]
pub struct Adx {
    /// Lookback period (default 14).
    pub period: usize,
}

impl Default for Adx {
    fn default() -> Self {
        Self { period: 14 }
    }
}

impl Indicator for Adx {
    fn name(&self) -> &'static str {
        "ADX"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::SubPanel {
            y_min: 0.0,
            y_max: 100.0,
        }
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let n = data.len();
        let mut adx_vals = vec![f64::NAN; n];
        let mut plus_di_vals = vec![f64::NAN; n];
        let mut minus_di_vals = vec![f64::NAN; n];

        let p = self.period;
        if p == 0 || n < 2 * p + 1 {
            return self.build_output(adx_vals, plus_di_vals, minus_di_vals, n);
        }

        // Compute raw TR, +DM, -DM for each bar starting at index 1
        let mut tr_raw = vec![0.0_f64; n];
        let mut plus_dm_raw = vec![0.0_f64; n];
        let mut minus_dm_raw = vec![0.0_f64; n];

        for i in 1..n {
            let prev_close = data[i - 1].close;
            let hl = data[i].high - data[i].low;
            let hc = (data[i].high - prev_close).abs();
            let lc = (data[i].low - prev_close).abs();
            tr_raw[i] = hl.max(hc).max(lc);

            let up_move = data[i].high - data[i - 1].high;
            let down_move = data[i - 1].low - data[i].low;
            let up = up_move.max(0.0);
            let down = down_move.max(0.0);
            plus_dm_raw[i] = if up > down { up } else { 0.0 };
            minus_dm_raw[i] = if down > up { down } else { 0.0 };
        }

        // Seed smoothed values with sum of first `period` raw values (indices 1..=period)
        let period_f = p as f64;
        let mut smoothed_tr = tr_raw[1..=p].iter().sum::<f64>();
        let mut smoothed_plus = plus_dm_raw[1..=p].iter().sum::<f64>();
        let mut smoothed_minus = minus_dm_raw[1..=p].iter().sum::<f64>();

        // Compute +DI, -DI at index p
        let (di_plus_seed, di_minus_seed) = di_values(smoothed_tr, smoothed_plus, smoothed_minus);
        plus_di_vals[p] = di_plus_seed;
        minus_di_vals[p] = di_minus_seed;

        // Collect DX values for seeding ADX
        let mut dx_values: Vec<f64> = Vec::with_capacity(p);
        dx_values.push(dx_value(di_plus_seed, di_minus_seed));

        // Extend smoothed values from index p+1..=2p to collect more DX for ADX seed
        for i in (p + 1)..n {
            smoothed_tr = smoothed_tr - smoothed_tr / period_f + tr_raw[i];
            smoothed_plus = smoothed_plus - smoothed_plus / period_f + plus_dm_raw[i];
            smoothed_minus = smoothed_minus - smoothed_minus / period_f + minus_dm_raw[i];

            let (di_p, di_m) = di_values(smoothed_tr, smoothed_plus, smoothed_minus);
            plus_di_vals[i] = di_p;
            minus_di_vals[i] = di_m;

            let dx = dx_value(di_p, di_m);
            dx_values.push(dx);

            if dx_values.len() == p {
                // Seed ADX at index i
                let adx_seed: f64 = dx_values.iter().sum::<f64>() / period_f;
                adx_vals[i] = adx_seed;

                // Continue Wilder's smoothing for ADX for remaining bars
                let mut prev_adx = adx_seed;
                for j in (i + 1)..n {
                    smoothed_tr = smoothed_tr - smoothed_tr / period_f + tr_raw[j];
                    smoothed_plus = smoothed_plus - smoothed_plus / period_f + plus_dm_raw[j];
                    smoothed_minus = smoothed_minus - smoothed_minus / period_f + minus_dm_raw[j];

                    let (dp, dm) = di_values(smoothed_tr, smoothed_plus, smoothed_minus);
                    plus_di_vals[j] = dp;
                    minus_di_vals[j] = dm;

                    let dx_j = dx_value(dp, dm);
                    prev_adx = (prev_adx * (period_f - 1.0) + dx_j) / period_f;
                    adx_vals[j] = prev_adx;
                }
                break;
            }
        }

        self.build_output(adx_vals, plus_di_vals, minus_di_vals, n)
    }
}

/// Compute +DI and -DI from smoothed values.
fn di_values(smoothed_tr: f64, smoothed_plus: f64, smoothed_minus: f64) -> (f64, f64) {
    if smoothed_tr < f64::EPSILON {
        return (0.0, 0.0);
    }
    let plus_di = 100.0 * smoothed_plus / smoothed_tr;
    let minus_di = 100.0 * smoothed_minus / smoothed_tr;
    (plus_di, minus_di)
}

/// Compute DX from +DI and -DI.
fn dx_value(plus_di: f64, minus_di: f64) -> f64 {
    let sum = plus_di + minus_di;
    if sum < f64::EPSILON {
        return 0.0;
    }
    100.0 * (plus_di - minus_di).abs() / sum
}

impl Adx {
    fn build_output(
        &self,
        adx_vals: Vec<f64>,
        plus_di_vals: Vec<f64>,
        minus_di_vals: Vec<f64>,
        n: usize,
    ) -> IndicatorOutput {
        IndicatorOutput {
            name: format!("ADX({})", self.period),
            placement: self.placement(),
            series: vec![
                IndicatorSeries {
                    name: "ADX",
                    values: adx_vals,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "+DI",
                    values: plus_di_vals,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "-DI",
                    values: minus_di_vals,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "threshold",
                    values: vec![25.0; n],
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

    fn rising_data(n: usize) -> Vec<Ohlcv> {
        (0..n)
            .map(|i| {
                bar(
                    100.0 + f64::from(i as u32),
                    99.0 + f64::from(i as u32),
                    99.5 + f64::from(i as u32),
                )
            })
            .collect()
    }

    #[test]
    fn adx_empty_data() {
        let out = Adx::default().compute(&[]);
        assert!(out.series[0].values.is_empty());
    }

    #[test]
    fn adx_series_count_and_placement() {
        let data = rising_data(50);
        let out = Adx { period: 14 }.compute(&data);
        assert_eq!(out.series.len(), 4);
        assert_eq!(out.series[0].name, "ADX");
        assert_eq!(out.series[1].name, "+DI");
        assert_eq!(out.series[2].name, "-DI");
        assert_eq!(out.series[3].name, "threshold");
        assert!(
            matches!(out.placement, IndicatorPlacement::SubPanel { y_min, y_max }
                if (y_min - 0.0).abs() < f64::EPSILON && (y_max - 100.0).abs() < f64::EPSILON)
        );
    }

    #[test]
    fn adx_nan_prefix() {
        let data = rising_data(60);
        let out = Adx { period: 14 }.compute(&data);
        let adx = &out.series[0].values;
        // First 2*period values should be NaN
        for val in &adx[..27] {
            assert!(val.is_nan(), "expected NaN in prefix, got {val}");
        }
        assert!(!adx[27].is_nan(), "expected valid ADX at index 27");
    }

    #[test]
    fn adx_in_range() {
        let data = rising_data(60);
        let out = Adx { period: 14 }.compute(&data);
        for series in &out.series[..3] {
            for &v in series.values.iter().filter(|v| !v.is_nan()) {
                assert!(v >= 0.0 && v <= 100.0, "ADX/DI out of range: {v}");
            }
        }
    }

    #[test]
    fn adx_strong_trend() {
        // With consistently rising data +DI should exceed -DI
        let data = rising_data(60);
        let out = Adx { period: 14 }.compute(&data);
        let plus_di = &out.series[1].values;
        let minus_di = &out.series[2].values;
        // Check last value
        let last = plus_di.len() - 1;
        if !plus_di[last].is_nan() && !minus_di[last].is_nan() {
            assert!(
                plus_di[last] > minus_di[last],
                "+DI should exceed -DI in uptrend"
            );
        }
    }

    #[test]
    fn adx_threshold_reference_line() {
        let data = rising_data(60);
        let out = Adx { period: 14 }.compute(&data);
        assert!((out.series[3].values[0] - 25.0).abs() < f64::EPSILON);
    }
}
