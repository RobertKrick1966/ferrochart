// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

//! Backtest equity curve — renders cumulative P&L as a sub-panel.

use super::{Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle};
use crate::Ohlcv;

/// Backtest equity curve indicator.
///
/// Accepts pre-computed per-bar returns and renders the cumulative equity
/// as a line in its own sub-panel.
#[derive(Debug, Clone)]
pub struct EquityCurve {
    /// Per-bar returns (e.g. from a backtest). One value per bar.
    /// `NaN` entries mean no position on that bar.
    pub returns: Vec<f64>,
}

impl Indicator for EquityCurve {
    fn name(&self) -> &'static str {
        "Equity"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::SubPanelAuto
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let _ = data; // returns are pre-computed, data length just bounds output
        let n = self.returns.len();
        let mut equity = vec![f64::NAN; n];
        let mut cum = 0.0_f64;

        for (eq, ret) in equity.iter_mut().zip(self.returns.iter()) {
            if ret.is_nan() {
                *eq = cum;
            } else {
                cum += ret;
                *eq = cum;
            }
        }

        IndicatorOutput {
            name: "Equity".to_string(),
            placement: self.placement(),
            series: vec![IndicatorSeries {
                name: "P&L",
                values: equity,
                style_hint: SeriesStyle::Line,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_data(n: usize) -> Vec<Ohlcv> {
        (0..n)
            .map(|i| Ohlcv {
                timestamp: i as i64,
                open: 100.0,
                high: 100.0,
                low: 100.0,
                close: 100.0,
                volume: 1000.0,
                institutional_ratio: 0.0,
            })
            .collect()
    }

    #[test]
    fn equity_cumulates_returns() {
        let ec = EquityCurve {
            returns: vec![1.0, -0.5, 2.0, -1.0, 0.5],
        };
        let output = ec.compute(&dummy_data(5));
        let v = &output.series[0].values;

        assert!((v[0] - 1.0).abs() < 1e-9);
        assert!((v[1] - 0.5).abs() < 1e-9);
        assert!((v[2] - 2.5).abs() < 1e-9);
        assert!((v[3] - 1.5).abs() < 1e-9);
        assert!((v[4] - 2.0).abs() < 1e-9);
    }

    #[test]
    fn equity_nan_returns_hold_value() {
        let ec = EquityCurve {
            returns: vec![1.0, f64::NAN, 2.0],
        };
        let output = ec.compute(&dummy_data(3));
        let v = &output.series[0].values;

        assert!((v[0] - 1.0).abs() < 1e-9);
        assert!((v[1] - 1.0).abs() < 1e-9); // NaN return = no change
        assert!((v[2] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn equity_empty() {
        let ec = EquityCurve { returns: vec![] };
        let output = ec.compute(&[]);
        assert!(output.series[0].values.is_empty());
    }

    #[test]
    fn equity_is_sub_panel() {
        let ec = EquityCurve { returns: vec![1.0] };
        assert_eq!(ec.placement(), IndicatorPlacement::SubPanelAuto);
    }
}
