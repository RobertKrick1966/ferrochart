// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

//! Level-of-Detail decimation for large datasets.
//!
//! When the number of visible bars exceeds the pixel width of the chart,
//! each candle is sub-pixel. Decimation aggregates groups of bars into
//! representative bars for efficient rendering.
//!
//! - **Min/Max decimation** for OHLCV data: preserves true highs/lows.
//! - **LTTB** (Largest Triangle Three Buckets) for indicator line series.

use crate::Ohlcv;

/// Determine how many bars to decimate to, or `None` if no decimation needed.
///
/// Returns a target count when bars are less than ~2px wide.
#[must_use]
pub fn decimate_target(visible_bars: usize, chart_pixel_width: f64) -> Option<usize> {
    if visible_bars == 0 || chart_pixel_width <= 0.0 {
        return None;
    }
    let pixels_per_bar = chart_pixel_width / visible_bars as f64;
    if pixels_per_bar < 2.0 {
        // Target: one bar per ~2 pixels (room for wick + body)
        Some(((chart_pixel_width / 2.0) as usize).max(1))
    } else {
        None
    }
}

/// Decimate OHLCV data using min/max grouping.
///
/// Groups consecutive bars into `target` buckets. Each bucket preserves:
/// - `open` of the first bar
/// - `close` of the last bar
/// - `high` = max of all highs
/// - `low` = min of all lows
/// - `volume` = sum of all volumes
/// - `institutional_ratio` = volume-weighted average
///
/// Returns the original slice if no decimation is needed.
/// O(n) time, O(target) space.
#[must_use]
pub fn min_max_decimate(data: &[Ohlcv], target: usize) -> Vec<Ohlcv> {
    let n = data.len();
    if n <= target || target == 0 {
        return data.to_vec();
    }

    let group_size = n as f64 / target as f64;
    let mut result = Vec::with_capacity(target);

    for i in 0..target {
        let start = (f64::from(i as u32) * group_size) as usize;
        let end = (f64::from((i + 1) as u32) * group_size) as usize;
        let end = end.min(n);
        let group = &data[start..end];

        if group.is_empty() {
            continue;
        }

        let mut high = f64::MIN;
        let mut low = f64::MAX;
        let mut vol_sum = 0.0_f64;
        let mut inst_weighted = 0.0_f64;

        for bar in group {
            if bar.high > high {
                high = bar.high;
            }
            if bar.low < low {
                low = bar.low;
            }
            vol_sum += bar.volume;
            inst_weighted += bar.institutional_ratio * bar.volume;
        }

        let first = &group[0];
        let last = &group[group.len() - 1];

        result.push(Ohlcv {
            timestamp: first.timestamp,
            open: first.open,
            high,
            low,
            close: last.close,
            volume: vol_sum,
            institutional_ratio: if vol_sum > 0.0 {
                inst_weighted / vol_sum
            } else {
                0.0
            },
        });
    }

    result
}

/// Decimate a single-valued series using LTTB (Largest Triangle Three Buckets).
///
/// Preserves visual shape better than naive downsampling for line data.
/// `NaN` values are treated as gaps.
/// O(n) time, O(target) space.
#[must_use]
pub fn lttb_decimate(values: &[f64], target: usize) -> Vec<f64> {
    let n = values.len();
    if n <= target || target < 3 {
        return values.to_vec();
    }

    let mut result = Vec::with_capacity(target);
    let bucket_size = (n - 2) as f64 / (target - 2) as f64;

    // Always keep first point
    result.push(values[0]);

    let mut prev_selected = 0_usize;

    for bucket_idx in 1..(target - 1) {
        let bucket_start = ((bucket_idx - 1) as f64 * bucket_size) as usize + 1;
        let bucket_end = ((bucket_idx) as f64 * bucket_size) as usize + 1;
        let bucket_end = bucket_end.min(n);

        // Next bucket average (for triangle area calculation)
        let next_start = bucket_end;
        let next_end = (((bucket_idx + 1) as f64 * bucket_size) as usize + 1).min(n);
        let mut avg_x = 0.0_f64;
        let mut avg_y = 0.0_f64;
        let mut count = 0_u32;
        for (j, &v) in values.iter().enumerate().take(next_end).skip(next_start) {
            if !v.is_nan() {
                avg_x += j as f64;
                avg_y += v;
                count += 1;
            }
        }
        if count > 0 {
            avg_x /= f64::from(count);
            avg_y /= f64::from(count);
        }

        // Find point in current bucket with largest triangle area
        let prev_x = prev_selected as f64;
        let prev_y = if values[prev_selected].is_nan() {
            0.0
        } else {
            values[prev_selected]
        };

        let mut max_area = -1.0_f64;
        let mut best_idx = bucket_start;

        for (j, &v) in values
            .iter()
            .enumerate()
            .take(bucket_end)
            .skip(bucket_start)
        {
            if v.is_nan() {
                continue;
            }
            let area =
                ((prev_x - avg_x) * (v - prev_y) - (prev_x - j as f64) * (avg_y - prev_y)).abs();
            if area > max_area {
                max_area = area;
                best_idx = j;
            }
        }

        result.push(values[best_idx]);
        prev_selected = best_idx;
    }

    // Always keep last point
    result.push(values[n - 1]);

    result
}

/// Decimate an indicator series to `target` length.
///
/// Uses LTTB for line data and max-abs bucketing for histogram data.
#[must_use]
pub fn decimate_series(values: &[f64], target: usize, is_histogram: bool) -> Vec<f64> {
    let n = values.len();
    if n <= target || target == 0 {
        return values.to_vec();
    }

    if !is_histogram {
        return lttb_decimate(values, target);
    }

    // Histogram: pick max-abs value per bucket
    let bucket_size = n as f64 / target as f64;
    let mut result = Vec::with_capacity(target);

    for i in 0..target {
        let start = (f64::from(i as u32) * bucket_size) as usize;
        let end = (f64::from((i + 1) as u32) * bucket_size) as usize;
        let end = end.min(n);

        let mut best = 0.0_f64;
        for &v in &values[start..end] {
            if v.abs() > best.abs() {
                best = v;
            }
        }
        result.push(best);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bars(n: usize) -> Vec<Ohlcv> {
        (0..n)
            .map(|i| {
                let base = 100.0 + (i as f64 * 0.1).sin() * 10.0;
                Ohlcv {
                    timestamp: i as i64,
                    open: base,
                    high: base + 2.0,
                    low: base - 2.0,
                    close: base + 1.0,
                    volume: 1000.0 + i as f64,
                    institutional_ratio: 0.0,
                }
            })
            .collect()
    }

    #[test]
    fn decimate_target_returns_none_when_enough_space() {
        assert!(decimate_target(100, 900.0).is_none());
        assert!(decimate_target(400, 900.0).is_none());
    }

    #[test]
    fn decimate_target_returns_some_when_subpixel() {
        let target = decimate_target(10_000, 900.0);
        assert!(target.is_some());
        assert!(target.unwrap() <= 450);
    }

    #[test]
    fn min_max_no_decimation_needed() {
        let data = make_bars(10);
        let result = min_max_decimate(&data, 20);
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn min_max_reduces_to_target() {
        let data = make_bars(1000);
        let result = min_max_decimate(&data, 100);
        assert_eq!(result.len(), 100);
    }

    #[test]
    fn min_max_preserves_extremes() {
        let data = make_bars(1000);
        let result = min_max_decimate(&data, 50);

        let orig_high = data.iter().map(|b| b.high).fold(f64::MIN, f64::max);
        let orig_low = data.iter().map(|b| b.low).fold(f64::MAX, f64::min);
        let dec_high = result.iter().map(|b| b.high).fold(f64::MIN, f64::max);
        let dec_low = result.iter().map(|b| b.low).fold(f64::MAX, f64::min);

        assert!(
            (orig_high - dec_high).abs() < 1e-9,
            "high must be preserved"
        );
        assert!((orig_low - dec_low).abs() < 1e-9, "low must be preserved");
    }

    #[test]
    fn min_max_preserves_total_volume() {
        let data = make_bars(500);
        let result = min_max_decimate(&data, 50);

        let orig_vol: f64 = data.iter().map(|b| b.volume).sum();
        let dec_vol: f64 = result.iter().map(|b| b.volume).sum();

        assert!(
            (orig_vol - dec_vol).abs() < 1.0,
            "total volume must be preserved"
        );
    }

    #[test]
    fn min_max_first_open_last_close() {
        let data = make_bars(100);
        let result = min_max_decimate(&data, 10);

        // First decimated bar should have first bar's open
        assert!((result[0].open - data[0].open).abs() < 1e-9);
        // Last decimated bar should have last bar's close
        assert!((result[9].close - data[99].close).abs() < 1e-9);
    }

    #[test]
    fn lttb_reduces_to_target() {
        let values: Vec<f64> = (0..1000).map(|i| (f64::from(i) * 0.1).sin()).collect();
        let result = lttb_decimate(&values, 50);
        assert_eq!(result.len(), 50);
    }

    #[test]
    fn lttb_preserves_endpoints() {
        let values: Vec<f64> = (0..100).map(f64::from).collect();
        let result = lttb_decimate(&values, 10);
        assert!((result[0] - 0.0).abs() < 1e-9);
        assert!((result[9] - 99.0).abs() < 1e-9);
    }

    #[test]
    fn lttb_no_decimation_needed() {
        let values = vec![1.0, 2.0, 3.0];
        let result = lttb_decimate(&values, 10);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn decimate_series_histogram() {
        let values: Vec<f64> = (0..100).map(|i| if i == 42 { 10.0 } else { 1.0 }).collect();
        let result = decimate_series(&values, 10, true);
        assert_eq!(result.len(), 10);
        // The bucket containing index 42 should have value 10.0
        assert!(result.iter().any(|&v| (v - 10.0).abs() < 1e-9));
    }

    #[test]
    fn empty_data() {
        assert!(min_max_decimate(&[], 10).is_empty());
        assert!(lttb_decimate(&[], 10).is_empty());
        assert!(decimate_target(0, 900.0).is_none());
    }

    #[test]
    fn large_dataset_decimation() {
        let data = make_bars(200_000);
        let result = min_max_decimate(&data, 450);
        assert_eq!(result.len(), 450);
    }
}
