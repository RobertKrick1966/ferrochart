// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

//! CUSUM (Cumulative Sum) filter — path-dependent event sampler.
//!
//! Based on López de Prado, *Advances in Financial Machine Learning*, Ch. 2.
//! Accumulates positive and negative return pressure separately.
//! Fires an event when either exceeds the threshold, then resets.

use crate::Ohlcv;

use super::{Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle};

/// Fixed-threshold CUSUM filter.
///
/// Produces a sub-panel with two series (`S+` and `S−`) plus event markers.
#[derive(Debug, Clone)]
pub struct Cusum {
    /// Threshold for triggering a CUSUM event (as fractional return, e.g. 0.03 = 3%).
    pub threshold: f64,
}

impl Indicator for Cusum {
    fn name(&self) -> &'static str {
        "CUSUM"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::SubPanelAuto
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let n = data.len();
        let mut s_pos = vec![0.0_f64; n];
        let mut s_neg = vec![0.0_f64; n];
        let mut events = vec![0.0_f64; n]; // +1.0 up, -1.0 down, 0.0 none

        let mut cum_pos = 0.0_f64;
        let mut cum_neg = 0.0_f64;

        for i in 1..n {
            let ret = (data[i].close - data[i - 1].close) / data[i - 1].close;
            cum_pos = (cum_pos + ret).max(0.0);
            cum_neg = (cum_neg + ret).min(0.0);

            if cum_pos >= self.threshold {
                events[i] = 1.0;
                cum_pos = 0.0;
                cum_neg = 0.0;
            } else if cum_neg <= -self.threshold {
                events[i] = -1.0;
                cum_pos = 0.0;
                cum_neg = 0.0;
            }

            s_pos[i] = cum_pos;
            s_neg[i] = cum_neg;
        }

        IndicatorOutput {
            name: format!("CUSUM({:.1}%)", self.threshold * 100.0),
            placement: self.placement(),
            series: vec![
                IndicatorSeries {
                    name: "S+",
                    values: s_pos,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "S−",
                    values: s_neg,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "Event",
                    values: events,
                    style_hint: SeriesStyle::Histogram,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_data(prices: &[f64]) -> Vec<Ohlcv> {
        prices
            .iter()
            .enumerate()
            .map(|(i, &p)| Ohlcv {
                timestamp: i as i64,
                open: p,
                high: p,
                low: p,
                close: p,
                volume: 1000.0,
                institutional_ratio: 0.0,
            })
            .collect()
    }

    #[test]
    fn cusum_no_event_below_threshold() {
        // Small price moves, should not trigger
        let data = make_data(&[100.0, 100.5, 101.0, 100.8, 101.2]);
        let cusum = Cusum { threshold: 0.05 };
        let output = cusum.compute(&data);

        let events = &output.series[2].values;
        assert!(events.iter().all(|&e| e == 0.0));
    }

    #[test]
    fn cusum_upward_event() {
        // Big jump: 100 -> 106 = +6% in one bar, threshold 5%
        let data = make_data(&[100.0, 106.0]);
        let cusum = Cusum { threshold: 0.05 };
        let output = cusum.compute(&data);

        assert!((output.series[2].values[1] - 1.0).abs() < f64::EPSILON); // upward event
    }

    #[test]
    fn cusum_downward_event() {
        // Big drop: 100 -> 94 = -6%, threshold 5%
        let data = make_data(&[100.0, 94.0]);
        let cusum = Cusum { threshold: 0.05 };
        let output = cusum.compute(&data);

        assert!((output.series[2].values[1] - (-1.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn cusum_resets_after_event() {
        // 100 -> 106 (event) -> 106.5 (no event, reset)
        let data = make_data(&[100.0, 106.0, 106.5]);
        let cusum = Cusum { threshold: 0.05 };
        let output = cusum.compute(&data);

        // Event at bar 1
        assert!((output.series[2].values[1] - 1.0).abs() < f64::EPSILON);
        // S+ resets, bar 2 has small cumulative
        assert!(output.series[0].values[2] < 0.05);
        // No event at bar 2
        assert!(output.series[2].values[2].abs() < f64::EPSILON);
    }

    #[test]
    fn cusum_cumulative_buildup() {
        // Gradual rise: 1% per bar, threshold 3%
        // After 3 bars: cum_pos ~= 3.03% > 3% -> event
        let data = make_data(&[100.0, 101.0, 102.01, 103.0301]);
        let cusum = Cusum { threshold: 0.03 };
        let output = cusum.compute(&data);

        let s_pos = &output.series[0].values;
        let events = &output.series[2].values;

        // Bar 1: ~1%, bar 2: ~2%, bar 3: ~3% -> event
        assert!(s_pos[1] > 0.009);
        assert!(s_pos[2] > 0.019);
        assert!((events[3] - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cusum_output_shape() {
        let data = make_data(&[100.0, 101.0, 102.0, 103.0, 104.0]);
        let cusum = Cusum { threshold: 0.05 };
        let output = cusum.compute(&data);

        assert_eq!(output.series.len(), 3);
        assert_eq!(output.series[0].name, "S+");
        assert_eq!(output.series[1].name, "S−");
        assert_eq!(output.series[2].name, "Event");
        assert_eq!(output.placement, IndicatorPlacement::SubPanelAuto);
        for s in &output.series {
            assert_eq!(s.values.len(), 5);
        }
    }

    #[test]
    fn cusum_first_bar_is_zero() {
        let data = make_data(&[100.0, 110.0]);
        let cusum = Cusum { threshold: 0.05 };
        let output = cusum.compute(&data);

        assert!(output.series[0].values[0].abs() < f64::EPSILON);
        assert!(output.series[1].values[0].abs() < f64::EPSILON);
        assert!(output.series[2].values[0].abs() < f64::EPSILON);
    }

    #[test]
    fn cusum_empty_data() {
        let cusum = Cusum { threshold: 0.05 };
        let output = cusum.compute(&[]);
        assert_eq!(output.series[0].values.len(), 0);
    }
}
