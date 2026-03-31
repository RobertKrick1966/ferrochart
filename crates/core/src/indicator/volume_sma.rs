// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use super::{
    Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle, compute_sma,
};
use crate::Ohlcv;

/// Extract volume values from OHLCV data.
fn volumes(data: &[Ohlcv]) -> Vec<f64> {
    data.iter().map(|b| b.volume).collect()
}

/// Simple Moving Average applied to Volume (Average Volume).
///
/// Displays as a line in the volume sub-panel, showing the average
/// volume over the given period.  Useful for spotting volume spikes
/// relative to the rolling mean.
#[derive(Debug, Clone)]
pub struct VolumeSma {
    pub period: usize,
}

impl Indicator for VolumeSma {
    fn name(&self) -> &'static str {
        "VolSMA"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::SubPanelAuto
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let values = compute_sma(&volumes(data), self.period);
        IndicatorOutput {
            name: format!("VolSMA({})", self.period),
            placement: self.placement(),
            series: vec![IndicatorSeries {
                name: "VolSMA",
                values,
                style_hint: SeriesStyle::Line,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Ohlcv;

    fn bar(volume: f64) -> Ohlcv {
        Ohlcv {
            timestamp: 0,
            open: 100.0,
            high: 100.0,
            low: 100.0,
            close: 100.0,
            volume,
            institutional_ratio: 0.0,
        }
    }

    #[test]
    fn volume_sma_basic() {
        let data: Vec<Ohlcv> = [100.0, 200.0, 300.0, 400.0, 500.0]
            .iter()
            .map(|&v| bar(v))
            .collect();
        let out = VolumeSma { period: 3 }.compute(&data);
        let v = &out.series[0].values;
        assert!(v[0].is_nan());
        assert!(v[1].is_nan());
        assert!((v[2] - 200.0).abs() < 1e-9);
        assert!((v[3] - 300.0).abs() < 1e-9);
        assert!((v[4] - 400.0).abs() < 1e-9);
    }

    #[test]
    fn volume_sma_period_equals_length() {
        let data: Vec<Ohlcv> = [1000.0, 2000.0, 3000.0].iter().map(|&v| bar(v)).collect();
        let out = VolumeSma { period: 3 }.compute(&data);
        let v = &out.series[0].values;
        assert!(v[0].is_nan());
        assert!(v[1].is_nan());
        assert!((v[2] - 2000.0).abs() < 1e-9);
    }

    #[test]
    fn volume_sma_period_larger_than_data() {
        let data: Vec<Ohlcv> = [100.0, 200.0].iter().map(|&v| bar(v)).collect();
        let out = VolumeSma { period: 5 }.compute(&data);
        assert!(out.series[0].values.iter().all(|v| v.is_nan()));
    }

    #[test]
    fn volume_sma_empty_data() {
        let out = VolumeSma { period: 3 }.compute(&[]);
        assert!(out.series[0].values.is_empty());
    }

    #[test]
    fn volume_sma_period_one() {
        let data: Vec<Ohlcv> = [500.0, 1000.0, 1500.0].iter().map(|&v| bar(v)).collect();
        let out = VolumeSma { period: 1 }.compute(&data);
        let v = &out.series[0].values;
        assert!((v[0] - 500.0).abs() < 1e-9);
        assert!((v[1] - 1000.0).abs() < 1e-9);
        assert!((v[2] - 1500.0).abs() < 1e-9);
    }

    #[test]
    fn volume_sma_placement_is_sub_panel_auto() {
        assert_eq!(
            VolumeSma { period: 20 }.placement(),
            IndicatorPlacement::SubPanelAuto
        );
    }

    #[test]
    fn volume_sma_name_includes_period() {
        let out = VolumeSma { period: 20 }.compute(&[bar(100.0)]);
        assert_eq!(out.name, "VolSMA(20)");
    }

    #[test]
    fn volume_sma_uses_volume_not_close() {
        // Verify it uses volume field, not close
        let data = vec![
            Ohlcv {
                timestamp: 0,
                open: 50.0,
                high: 60.0,
                low: 40.0,
                close: 55.0,
                volume: 1000.0,
                institutional_ratio: 0.0,
            },
            Ohlcv {
                timestamp: 1,
                open: 55.0,
                high: 65.0,
                low: 45.0,
                close: 60.0,
                volume: 2000.0,
                institutional_ratio: 0.0,
            },
            Ohlcv {
                timestamp: 2,
                open: 60.0,
                high: 70.0,
                low: 50.0,
                close: 65.0,
                volume: 3000.0,
                institutional_ratio: 0.0,
            },
        ];
        let out = VolumeSma { period: 3 }.compute(&data);
        let v = &out.series[0].values;
        // Average of 1000, 2000, 3000 = 2000, not average of closes
        assert!((v[2] - 2000.0).abs() < 1e-9);
    }
}
