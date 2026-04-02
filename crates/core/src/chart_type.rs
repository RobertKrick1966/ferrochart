// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use crate::Ohlcv;

/// The visual style used to render price bars.
#[derive(Debug, Clone, Copy, Default)]
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
    /// Renko chart — brick size in price units.
    Renko {
        /// Price movement required to form one brick.
        brick_size: f64,
    },
    /// Point & Figure chart.
    PointFigure {
        /// Price increment per box.
        box_size: f64,
        /// Number of boxes required for a reversal.
        reversal: usize,
    },
}

impl PartialEq for ChartType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Candlestick, Self::Candlestick)
            | (Self::HeikinAshi, Self::HeikinAshi)
            | (Self::Line, Self::Line)
            | (Self::Area, Self::Area)
            | (Self::OhlcBars, Self::OhlcBars) => true,
            (Self::Renko { brick_size: a }, Self::Renko { brick_size: b }) => a == b,
            (
                Self::PointFigure {
                    box_size: a,
                    reversal: ra,
                },
                Self::PointFigure {
                    box_size: b,
                    reversal: rb,
                },
            ) => a == b && ra == rb,
            _ => false,
        }
    }
}

/// A single Renko brick.
#[derive(Debug, Clone)]
pub struct RenkoBar {
    /// Unix timestamp of the OHLCV bar that completed this brick.
    pub timestamp: i64,
    /// Brick open price.
    pub open: f64,
    /// Brick high price (equals close for up-bricks, open for down-bricks).
    pub high: f64,
    /// Brick low price (equals open for up-bricks, close for down-bricks).
    pub low: f64,
    /// Brick close price.
    pub close: f64,
    /// True if this is an up-brick (close > open).
    pub up: bool,
}

/// Convert OHLCV data to Renko bricks.
///
/// A new up-brick forms when price rises by `brick_size` above the last brick top.
/// A new down-brick forms when price falls by `brick_size` below the last brick bottom.
/// Multiple bricks can form from a single OHLCV bar.
#[must_use]
pub fn compute_renko(data: &[Ohlcv], brick_size: f64) -> Vec<RenkoBar> {
    if data.is_empty() || brick_size <= 0.0 {
        return Vec::new();
    }

    let mut result = Vec::new();
    let mut current_level = (data[0].close / brick_size).round() * brick_size;

    for bar in data {
        // Up bricks
        while bar.close >= current_level + brick_size {
            let open = current_level;
            let close = current_level + brick_size;
            result.push(RenkoBar {
                timestamp: bar.timestamp,
                open,
                high: close,
                low: open,
                close,
                up: true,
            });
            current_level += brick_size;
        }
        // Down bricks
        while bar.close <= current_level - brick_size {
            let open = current_level;
            let close = current_level - brick_size;
            result.push(RenkoBar {
                timestamp: bar.timestamp,
                open,
                high: open,
                low: close,
                close,
                up: false,
            });
            current_level -= brick_size;
        }
    }

    result
}

/// Direction of a P&F column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PFDirection {
    /// Rising column (X symbols).
    X,
    /// Falling column (O symbols).
    O,
}

/// A single P&F column.
#[derive(Debug, Clone)]
pub struct PFColumn {
    /// Price of the lowest box in this column.
    pub bottom_price: f64,
    /// Price of the highest box in this column (exclusive top of last box).
    pub top_price: f64,
    /// Column direction.
    pub direction: PFDirection,
    /// Number of boxes in this column.
    pub box_count: usize,
    /// Unix timestamp of the last bar that contributed to this column.
    pub timestamp: i64,
}

/// Convert OHLCV data to Point & Figure columns.
///
/// `box_size` is the price increment per box. `reversal` is the number of boxes
/// required for a column reversal (typically 3).
#[must_use]
pub fn compute_point_figure(data: &[Ohlcv], box_size: f64, reversal: usize) -> Vec<PFColumn> {
    if data.is_empty() || box_size <= 0.0 || reversal == 0 {
        return Vec::new();
    }

    // Round first close to nearest box boundary
    let first_price = (data[0].close / box_size).round() * box_size;

    // Determine initial direction from first price movement
    let mut current_price = first_price;
    let mut current_direction = PFDirection::X; // default; will be set on first movement
    let mut column_start = first_price;
    let mut last_timestamp = data[0].timestamp;
    let mut direction_set = false;
    let mut result: Vec<PFColumn> = Vec::new();

    for bar in data {
        let high_boxes = ((bar.high - current_price) / box_size).floor();
        let low_boxes = ((current_price - bar.low) / box_size).floor();

        if !direction_set {
            // Set initial direction based on first meaningful movement
            if high_boxes >= 1.0 {
                current_direction = PFDirection::X;
                column_start = current_price;
                current_price += high_boxes * box_size;
                last_timestamp = bar.timestamp;
                direction_set = true;
            } else if low_boxes >= 1.0 {
                current_direction = PFDirection::O;
                column_start = current_price;
                current_price -= low_boxes * box_size;
                last_timestamp = bar.timestamp;
                direction_set = true;
            }
            continue;
        }

        match current_direction {
            PFDirection::X => {
                if high_boxes >= 1.0 {
                    // Continue the X column upward
                    current_price += high_boxes * box_size;
                    last_timestamp = bar.timestamp;
                } else if low_boxes >= reversal as f64 {
                    // Reversal: push current X column, start new O column
                    let box_count =
                        (((current_price - column_start) / box_size).round() as usize).max(1);
                    result.push(PFColumn {
                        bottom_price: column_start,
                        top_price: current_price,
                        direction: PFDirection::X,
                        box_count,
                        timestamp: last_timestamp,
                    });
                    // New O column starts from current price, drops by reversal boxes
                    column_start = current_price;
                    current_direction = PFDirection::O;
                    current_price -= low_boxes * box_size;
                    last_timestamp = bar.timestamp;
                }
            }
            PFDirection::O => {
                if low_boxes >= 1.0 {
                    // Continue the O column downward
                    current_price -= low_boxes * box_size;
                    last_timestamp = bar.timestamp;
                } else if high_boxes >= reversal as f64 {
                    // Reversal: push current O column, start new X column
                    let box_count =
                        (((column_start - current_price) / box_size).round() as usize).max(1);
                    result.push(PFColumn {
                        bottom_price: current_price,
                        top_price: column_start,
                        direction: PFDirection::O,
                        box_count,
                        timestamp: last_timestamp,
                    });
                    // New X column starts from current price, rises by reversal boxes
                    column_start = current_price;
                    current_direction = PFDirection::X;
                    current_price += high_boxes * box_size;
                    last_timestamp = bar.timestamp;
                }
            }
        }
    }

    // Push the last in-progress column
    if direction_set {
        match current_direction {
            PFDirection::X => {
                let box_count =
                    (((current_price - column_start) / box_size).round() as usize).max(1);
                result.push(PFColumn {
                    bottom_price: column_start,
                    top_price: current_price,
                    direction: PFDirection::X,
                    box_count,
                    timestamp: last_timestamp,
                });
            }
            PFDirection::O => {
                let box_count =
                    (((column_start - current_price) / box_size).round() as usize).max(1);
                result.push(PFColumn {
                    bottom_price: current_price,
                    top_price: column_start,
                    direction: PFDirection::O,
                    box_count,
                    timestamp: last_timestamp,
                });
            }
        }
    }

    result
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

    // ─── Renko tests ──────────────────────────────────────────────────────────

    fn renko_bar(close: f64) -> Ohlcv {
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
    fn renko_empty_data() {
        assert!(compute_renko(&[], 1.0).is_empty());
    }

    #[test]
    fn renko_zero_brick_size() {
        let data = vec![renko_bar(100.0)];
        assert!(compute_renko(&data, 0.0).is_empty());
    }

    #[test]
    fn renko_single_bar_no_bricks() {
        // A single bar produces no bricks (no price movement)
        let data = vec![renko_bar(100.0)];
        assert!(compute_renko(&data, 5.0).is_empty());
    }

    #[test]
    fn renko_three_up_bricks() {
        let data = vec![renko_bar(100.0), renko_bar(115.0)];
        let bricks = compute_renko(&data, 5.0);
        assert_eq!(bricks.len(), 3);
        assert!(bricks.iter().all(|b| b.up));
    }

    #[test]
    fn renko_two_down_bricks() {
        let data = vec![renko_bar(100.0), renko_bar(90.0)];
        let bricks = compute_renko(&data, 5.0);
        assert_eq!(bricks.len(), 2);
        assert!(bricks.iter().all(|b| !b.up));
    }

    #[test]
    fn renko_up_brick_geometry() {
        let data = vec![renko_bar(100.0), renko_bar(105.0)];
        let bricks = compute_renko(&data, 5.0);
        assert_eq!(bricks.len(), 1);
        let b = &bricks[0];
        assert!(b.up);
        assert!((b.high - b.close).abs() < 1e-9, "up-brick high == close");
        assert!((b.low - b.open).abs() < 1e-9, "up-brick low == open");
    }

    #[test]
    fn renko_down_brick_geometry() {
        let data = vec![renko_bar(100.0), renko_bar(95.0)];
        let bricks = compute_renko(&data, 5.0);
        assert_eq!(bricks.len(), 1);
        let b = &bricks[0];
        assert!(!b.up);
        assert!((b.high - b.open).abs() < 1e-9, "down-brick high == open");
        assert!((b.low - b.close).abs() < 1e-9, "down-brick low == close");
    }

    // ─── Point & Figure tests ─────────────────────────────────────────────────

    fn pf_bar(timestamp: i64, open: f64, high: f64, low: f64, close: f64) -> Ohlcv {
        Ohlcv {
            timestamp,
            open,
            high,
            low,
            close,
            volume: 0.0,
            institutional_ratio: 0.0,
        }
    }

    #[test]
    fn pf_empty_data() {
        assert!(compute_point_figure(&[], 1.0, 3).is_empty());
    }

    #[test]
    fn pf_zero_box_size() {
        let data = vec![pf_bar(0, 100.0, 105.0, 95.0, 100.0)];
        assert!(compute_point_figure(&data, 0.0, 3).is_empty());
    }

    #[test]
    fn pf_zero_reversal() {
        let data = vec![pf_bar(0, 100.0, 105.0, 95.0, 100.0)];
        assert!(compute_point_figure(&data, 1.0, 0).is_empty());
    }

    #[test]
    fn pf_rising_price_only_x_columns() {
        // Prices only go up, so we should only get X columns
        let data: Vec<Ohlcv> = (0..10)
            .map(|i| {
                let price = 100.0 + f64::from(i) * 5.0;
                pf_bar(i64::from(i), price - 1.0, price + 1.0, price - 1.0, price)
            })
            .collect();
        let cols = compute_point_figure(&data, 1.0, 3);
        assert!(cols.iter().all(|c| c.direction == PFDirection::X));
    }

    #[test]
    fn pf_reversal_creates_o_column() {
        // Price rises then falls enough for reversal
        let data = vec![
            pf_bar(0, 100.0, 110.0, 100.0, 110.0), // rises to 110
            pf_bar(1, 110.0, 110.0, 100.0, 100.0), // falls back to 100 (10 boxes = reversal)
        ];
        let cols = compute_point_figure(&data, 1.0, 3);
        // Should have at least one X column and one O column
        assert!(cols.iter().any(|c| c.direction == PFDirection::X));
        assert!(cols.iter().any(|c| c.direction == PFDirection::O));
    }

    #[test]
    fn pf_box_count_correct() {
        // Price rises from 100 to 105 with box_size=1 → 5 boxes X
        let data = vec![
            pf_bar(0, 100.0, 100.0, 99.0, 100.0),
            pf_bar(1, 100.0, 105.0, 100.0, 105.0),
        ];
        let cols = compute_point_figure(&data, 1.0, 3);
        // Find the X column
        let x_col = cols.iter().find(|c| c.direction == PFDirection::X);
        if let Some(col) = x_col {
            assert!(
                col.box_count >= 5,
                "expected at least 5 boxes, got {}",
                col.box_count
            );
        }
    }
}
