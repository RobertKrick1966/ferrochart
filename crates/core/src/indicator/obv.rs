// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use super::{Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle};
use crate::Ohlcv;

/// On-Balance Volume — cumulative volume indicator tracking buying/selling pressure.
#[derive(Debug, Clone)]
pub struct Obv;

impl Indicator for Obv {
    fn name(&self) -> &'static str {
        "OBV"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::SubPanelAuto
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let n = data.len();
        let mut values = vec![0.0_f64; n];

        for i in 1..n {
            if data[i].close > data[i - 1].close {
                values[i] = values[i - 1] + data[i].volume;
            } else if data[i].close < data[i - 1].close {
                values[i] = values[i - 1] - data[i].volume;
            } else {
                values[i] = values[i - 1];
            }
        }

        IndicatorOutput {
            name: "OBV".to_string(),
            placement: self.placement(),
            series: vec![IndicatorSeries {
                name: "OBV",
                values,
                style_hint: SeriesStyle::Line,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(close: f64, volume: f64) -> Ohlcv {
        Ohlcv {
            timestamp: 0,
            open: close,
            high: close,
            low: close,
            close,
            volume,
            institutional_ratio: 0.0,
        }
    }

    #[test]
    fn obv_empty_data() {
        let out = Obv.compute(&[]);
        assert!(out.series[0].values.is_empty());
    }

    #[test]
    fn obv_no_nan_values() {
        let data = vec![
            bar(100.0, 1000.0),
            bar(101.0, 1200.0),
            bar(100.5, 900.0),
            bar(100.5, 800.0),
        ];
        let out = Obv.compute(&data);
        for v in &out.series[0].values {
            assert!(!v.is_nan());
        }
    }

    #[test]
    fn obv_series_count_and_name() {
        let data = vec![bar(100.0, 1000.0), bar(101.0, 1200.0)];
        let out = Obv.compute(&data);
        assert_eq!(out.series.len(), 1);
        assert_eq!(out.series[0].name, "OBV");
    }

    #[test]
    fn obv_known_values() {
        // close up: add vol; close down: subtract vol; flat: unchanged
        let data = vec![
            bar(100.0, 1000.0), // obv[0] = 0
            bar(101.0, 500.0),  // close up: obv[1] = 500
            bar(100.0, 300.0),  // close down: obv[2] = 200
            bar(100.0, 400.0),  // flat: obv[3] = 200
            bar(102.0, 700.0),  // close up: obv[4] = 900
        ];
        let out = Obv.compute(&data);
        let v = &out.series[0].values;
        assert!((v[0] - 0.0).abs() < 1e-9);
        assert!((v[1] - 500.0).abs() < 1e-9);
        assert!((v[2] - 200.0).abs() < 1e-9);
        assert!((v[3] - 200.0).abs() < 1e-9);
        assert!((v[4] - 900.0).abs() < 1e-9);
    }

    #[test]
    fn obv_placement_is_sub_panel_auto() {
        assert_eq!(Obv.placement(), IndicatorPlacement::SubPanelAuto);
    }
}
