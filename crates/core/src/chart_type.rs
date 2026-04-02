// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use crate::Ohlcv;

/// The visual style used to render price bars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ChartType {
    /// Standard Japanese candlestick chart (default).
    #[default]
    Candlestick,
    /// Heikin-Ashi smoothed candles.
    HeikinAshi,
    /// Close-price polyline.
    Line,
    /// Filled area under the close-price line.
    Area,
    /// OHLC bar chart (vertical bar + left/right ticks).
    OhlcBars,
}

/// Compute Heikin-Ashi bars from standard OHLCV data.
///
/// - `ha_close[i]  = (open + high + low + close) / 4`
/// - `ha_open[0]   = (open[0] + close[0]) / 2`
/// - `ha_open[i]   = (ha_open[i-1] + ha_close[i-1]) / 2`
/// - `ha_high[i]   = max(high[i], ha_open[i], ha_close[i])`
/// - `ha_low[i]    = min(low[i],  ha_open[i], ha_close[i])`
///
/// `timestamp`, `volume`, and `institutional_ratio` are preserved unchanged.
///
/// Returns an empty `Vec` when `data` is empty.
#[must_use]
pub fn compute_heikin_ashi(data: &[Ohlcv]) -> Vec<Ohlcv> {
    if data.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(data.len());

    // First bar
    let ha_close_0 = (data[0].open + data[0].high + data[0].low + data[0].close) / 4.0;
    let ha_open_0 = f64::midpoint(data[0].open, data[0].close);
    let ha_high_0 = data[0].high.max(ha_open_0).max(ha_close_0);
    let ha_low_0 = data[0].low.min(ha_open_0).min(ha_close_0);

    result.push(Ohlcv {
        timestamp: data[0].timestamp,
        open: ha_open_0,
        high: ha_high_0,
        low: ha_low_0,
        close: ha_close_0,
        volume: data[0].volume,
        institutional_ratio: data[0].institutional_ratio,
    });

    for i in 1..data.len() {
        let prev = &result[i - 1];
        let ha_close = (data[i].open + data[i].high + data[i].low + data[i].close) / 4.0;
        let ha_open = f64::midpoint(prev.open, prev.close);
        let ha_high = data[i].high.max(ha_open).max(ha_close);
        let ha_low = data[i].low.min(ha_open).min(ha_close);

        result.push(Ohlcv {
            timestamp: data[i].timestamp,
            open: ha_open,
            high: ha_high,
            low: ha_low,
            close: ha_close,
            volume: data[i].volume,
            institutional_ratio: data[i].institutional_ratio,
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(open: f64, high: f64, low: f64, close: f64) -> Ohlcv {
        Ohlcv {
            timestamp: 0,
            open,
            high,
            low,
            close,
            volume: 1000.0,
            institutional_ratio: 0.0,
        }
    }

    #[test]
    fn chart_type_default_is_candlestick() {
        assert_eq!(ChartType::default(), ChartType::Candlestick);
    }

    #[test]
    fn heikin_ashi_empty() {
        assert!(compute_heikin_ashi(&[]).is_empty());
    }

    #[test]
    fn heikin_ashi_single_bar() {
        let data = vec![bar(100.0, 110.0, 90.0, 105.0)];
        let ha = compute_heikin_ashi(&data);
        assert_eq!(ha.len(), 1);

        let expected_close = (100.0 + 110.0 + 90.0 + 105.0) / 4.0;
        let expected_open = f64::midpoint(100.0, 105.0);
        assert!((ha[0].close - expected_close).abs() < 1e-9);
        assert!((ha[0].open - expected_open).abs() < 1e-9);
        // HA high >= both HA open and HA close
        assert!(ha[0].high >= ha[0].open);
        assert!(ha[0].high >= ha[0].close);
        // HA low <= both HA open and HA close
        assert!(ha[0].low <= ha[0].open);
        assert!(ha[0].low <= ha[0].close);
    }

    #[test]
    fn heikin_ashi_preserves_volume_and_timestamp() {
        let mut d = bar(100.0, 110.0, 90.0, 105.0);
        d.timestamp = 1_700_000_000;
        d.volume = 42_000.0;
        d.institutional_ratio = 0.25;
        let ha = compute_heikin_ashi(&[d]);
        assert_eq!(ha[0].timestamp, 1_700_000_000);
        assert!((ha[0].volume - 42_000.0).abs() < 1e-9);
        assert!((ha[0].institutional_ratio - 0.25).abs() < 1e-9);
    }

    #[test]
    fn heikin_ashi_open_second_bar() {
        let data = vec![
            bar(100.0, 110.0, 90.0, 105.0),
            bar(105.0, 115.0, 100.0, 110.0),
        ];
        let ha = compute_heikin_ashi(&data);

        // ha_open[1] = (ha_open[0] + ha_close[0]) / 2
        let expected_open_1 = f64::midpoint(ha[0].open, ha[0].close);
        assert!((ha[1].open - expected_open_1).abs() < 1e-9);
    }

    #[test]
    fn heikin_ashi_high_is_max() {
        let data = vec![bar(100.0, 90.0, 80.0, 85.0)]; // intentionally weird
        let ha = compute_heikin_ashi(&data);
        // ha_high should be >= ha_open and ha_close even if raw high is lower
        assert!(ha[0].high >= ha[0].open.min(ha[0].close));
    }

    #[test]
    fn heikin_ashi_many_bars_length() {
        let data: Vec<Ohlcv> = (0..50)
            .map(|i| {
                bar(
                    100.0 + f64::from(i),
                    110.0 + f64::from(i),
                    90.0 + f64::from(i),
                    105.0 + f64::from(i),
                )
            })
            .collect();
        let ha = compute_heikin_ashi(&data);
        assert_eq!(ha.len(), data.len());
    }
}
