// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use super::{Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle};
use crate::Ohlcv;

/// Session Volume-Weighted Average Price — resets at the start of each trading day.
///
/// Returns all `NAN` when the bar interval is daily or longer (≥ 86 400 seconds),
/// since intra-day session boundaries cannot be determined.
#[derive(Debug, Clone)]
pub struct SessionVwap;

impl Indicator for SessionVwap {
    fn name(&self) -> &'static str {
        "SessionVWAP"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::Overlay
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let n = data.len();
        let mut values = vec![f64::NAN; n];

        // Guard: need at least 2 bars to detect interval
        if n < 2 {
            return self.build_output(values);
        }

        let interval = (data[1].timestamp - data[0].timestamp).abs();
        // Daily or coarser: session boundaries undefined
        if interval >= 86_400 {
            return self.build_output(values);
        }

        let mut cum_tp_vol = 0.0_f64;
        let mut cum_vol = 0.0_f64;

        for (i, bar) in data.iter().enumerate() {
            let current_day = bar.timestamp / 86_400;
            let prev_day = if i > 0 {
                data[i - 1].timestamp / 86_400
            } else {
                current_day
            };

            // New session: reset accumulators
            if i == 0 || current_day != prev_day {
                cum_tp_vol = 0.0;
                cum_vol = 0.0;
            }

            let typical_price = (bar.high + bar.low + bar.close) / 3.0;
            cum_tp_vol += typical_price * bar.volume;
            cum_vol += bar.volume;

            values[i] = if cum_vol > 0.0 {
                cum_tp_vol / cum_vol
            } else {
                typical_price
            };
        }

        self.build_output(values)
    }
}

impl SessionVwap {
    fn build_output(&self, values: Vec<f64>) -> IndicatorOutput {
        IndicatorOutput {
            name: "SessionVWAP".to_string(),
            placement: self.placement(),
            series: vec![IndicatorSeries {
                name: "SessionVWAP",
                values,
                style_hint: SeriesStyle::Line,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intraday_bar(timestamp: i64, high: f64, low: f64, close: f64, volume: f64) -> Ohlcv {
        Ohlcv {
            timestamp,
            open: close,
            high,
            low,
            close,
            volume,
            institutional_ratio: 0.0,
        }
    }

    #[test]
    fn session_vwap_empty_data() {
        let out = SessionVwap.compute(&[]);
        assert!(out.series[0].values.is_empty());
    }

    #[test]
    fn session_vwap_daily_data_returns_all_nan() {
        // Daily bars (86400 second intervals)
        let base: i64 = 1_700_000_000;
        let data = vec![
            intraday_bar(base, 110.0, 90.0, 100.0, 1000.0),
            intraday_bar(base + 86_400, 115.0, 95.0, 105.0, 1200.0),
            intraday_bar(base + 172_800, 120.0, 100.0, 110.0, 900.0),
        ];
        let out = SessionVwap.compute(&data);
        for v in &out.series[0].values {
            assert!(v.is_nan(), "expected NaN for daily data, got {v}");
        }
    }

    #[test]
    fn session_vwap_series_count_and_name() {
        // Single intraday bar
        let data = vec![intraday_bar(1_700_000_000, 110.0, 90.0, 100.0, 1000.0)];
        let out = SessionVwap.compute(&data);
        assert_eq!(out.series.len(), 1);
        assert_eq!(out.series[0].name, "SessionVWAP");
    }

    #[test]
    fn session_vwap_single_session_known_value() {
        // Two intraday bars on the same day (3600 second interval)
        let base: i64 = 1_700_000_000; // some day
        let data = vec![
            intraday_bar(base, 110.0, 90.0, 100.0, 1000.0),
            intraday_bar(base + 3600, 120.0, 100.0, 110.0, 2000.0),
        ];
        let out = SessionVwap.compute(&data);
        let v = &out.series[0].values;

        // bar 0: tp = (110+90+100)/3 = 100, vol=1000 → vwap = 100
        let tp0 = (110.0 + 90.0 + 100.0) / 3.0;
        assert!((v[0] - tp0).abs() < 1e-9);

        // bar 1: tp = (120+100+110)/3 ≈ 110, cumulative
        let tp1 = (120.0 + 100.0 + 110.0) / 3.0;
        let expected = (tp0 * 1000.0 + tp1 * 2000.0) / 3000.0;
        assert!((v[1] - expected).abs() < 1e-9);
    }

    #[test]
    fn session_vwap_resets_on_new_day() {
        // Three bars: two on day 0 (3600s apart), one on day 1.
        // Interval = 3600 < 86400, so VWAP is enabled.
        let base: i64 = 3600; // early morning day 0
        let data = vec![
            intraday_bar(base, 110.0, 90.0, 100.0, 1000.0),
            intraday_bar(base + 3600, 115.0, 95.0, 105.0, 500.0),
            intraday_bar(86_400 + 3600, 120.0, 100.0, 110.0, 2000.0), // day 1
        ];
        let out = SessionVwap.compute(&data);
        let v = &out.series[0].values;

        // day 1 bar should be computed with only its own TP*vol (reset)
        let tp2 = (120.0 + 100.0 + 110.0) / 3.0;
        assert!(!v[2].is_nan(), "expected valid VWAP on day 1, got NaN");
        assert!((v[2] - tp2).abs() < 1e-9, "expected VWAP reset on new day, got {}", v[2]);
    }

    #[test]
    fn session_vwap_placement_is_overlay() {
        assert_eq!(SessionVwap.placement(), IndicatorPlacement::Overlay);
    }
}
