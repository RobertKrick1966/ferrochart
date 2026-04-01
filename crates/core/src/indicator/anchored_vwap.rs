// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

//! Anchored VWAP — Volume-Weighted Average Price from a user-specified anchor bar.
//!
//! Computes `cumulative(typical_price * volume) / cumulative(volume)` starting
//! at the anchor bar. Values before the anchor are `NaN`.

use crate::Ohlcv;

use super::{Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle};

/// Anchored VWAP overlay indicator.
///
/// Renders as a line on the price panel, starting from `anchor_bar`.
#[derive(Debug, Clone)]
pub struct AnchoredVwap {
    /// Bar index (in the full dataset) from which to begin the VWAP computation.
    pub anchor_bar: usize,
}

impl Indicator for AnchoredVwap {
    fn name(&self) -> &'static str {
        "AnchoredVWAP"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::Overlay
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let n = data.len();
        let mut values = vec![f64::NAN; n];

        let mut cum_pv = 0.0_f64;
        let mut cum_vol = 0.0_f64;

        for i in self.anchor_bar..n {
            let bar = &data[i];
            let typical_price = (bar.high + bar.low + bar.close) / 3.0;
            cum_pv += typical_price * bar.volume;
            cum_vol += bar.volume;

            if cum_vol > 0.0 {
                values[i] = cum_pv / cum_vol;
            }
        }

        IndicatorOutput {
            name: format!("VWAP({})", self.anchor_bar),
            placement: self.placement(),
            series: vec![IndicatorSeries {
                name: "VWAP",
                values,
                style_hint: SeriesStyle::Line,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_data(bars: &[(f64, f64, f64, f64, f64)]) -> Vec<Ohlcv> {
        bars.iter()
            .enumerate()
            .map(|(i, &(o, h, l, c, v))| Ohlcv {
                timestamp: i as i64,
                open: o,
                high: h,
                low: l,
                close: c,
                volume: v,
                institutional_ratio: 0.0,
            })
            .collect()
    }

    #[test]
    fn vwap_basic_computation() {
        // Bar 0: TP = (110+90+100)/3 = 100, vol = 1000 → VWAP = 100
        // Bar 1: TP = (120+100+110)/3 = 110, vol = 2000 → cum_pv = 100*1000+110*2000 = 320000, cum_vol = 3000, VWAP = 106.67
        let data = make_data(&[
            (100.0, 110.0, 90.0, 100.0, 1000.0),
            (100.0, 120.0, 100.0, 110.0, 2000.0),
        ]);
        let vwap = AnchoredVwap { anchor_bar: 0 };
        let output = vwap.compute(&data);
        let v = &output.series[0].values;

        assert!((v[0] - 100.0).abs() < 0.01);
        assert!((v[1] - 106.667).abs() < 0.01);
    }

    #[test]
    fn vwap_anchor_skips_early_bars() {
        let data = make_data(&[
            (100.0, 110.0, 90.0, 100.0, 1000.0),
            (100.0, 120.0, 100.0, 110.0, 2000.0),
            (110.0, 130.0, 105.0, 120.0, 1500.0),
        ]);
        let vwap = AnchoredVwap { anchor_bar: 1 };
        let output = vwap.compute(&data);
        let v = &output.series[0].values;

        assert!(v[0].is_nan());
        assert!(!v[1].is_nan());
        assert!(!v[2].is_nan());
    }

    #[test]
    fn vwap_anchor_beyond_data() {
        let data = make_data(&[(100.0, 110.0, 90.0, 100.0, 1000.0)]);
        let vwap = AnchoredVwap { anchor_bar: 10 };
        let output = vwap.compute(&data);

        assert!(output.series[0].values.iter().all(|v| v.is_nan()));
    }

    #[test]
    fn vwap_empty_data() {
        let vwap = AnchoredVwap { anchor_bar: 0 };
        let output = vwap.compute(&[]);
        assert!(output.series[0].values.is_empty());
    }

    #[test]
    fn vwap_zero_volume() {
        let data = make_data(&[
            (100.0, 110.0, 90.0, 100.0, 0.0),
            (100.0, 120.0, 100.0, 110.0, 0.0),
        ]);
        let vwap = AnchoredVwap { anchor_bar: 0 };
        let output = vwap.compute(&data);

        // Zero volume → NaN (no VWAP defined)
        assert!(output.series[0].values[0].is_nan());
        assert!(output.series[0].values[1].is_nan());
    }

    #[test]
    fn vwap_is_overlay() {
        let vwap = AnchoredVwap { anchor_bar: 0 };
        assert_eq!(vwap.placement(), IndicatorPlacement::Overlay);
    }

    #[test]
    fn vwap_single_bar_equals_typical_price() {
        let data = make_data(&[(100.0, 120.0, 80.0, 110.0, 5000.0)]);
        let vwap = AnchoredVwap { anchor_bar: 0 };
        let output = vwap.compute(&data);

        let tp = (120.0 + 80.0 + 110.0) / 3.0; // 103.33
        assert!((output.series[0].values[0] - tp).abs() < 0.01);
    }
}
