// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

//! Volume Profile — price-level volume distribution.
//!
//! Aggregates volume into equal-width price buckets across the visible data.
//! Each bar's volume is proportionally distributed across the buckets its
//! `[low, high]` range touches.

use crate::Ohlcv;

/// A single price bucket in the volume profile.
#[derive(Debug, Clone, Copy)]
pub struct VolumeProfileBucket {
    /// Lower bound of this price bucket.
    pub price_low: f64,
    /// Upper bound of this price bucket.
    pub price_high: f64,
    /// Total volume accumulated in this bucket.
    pub volume: f64,
}

/// Volume distribution across price levels.
///
/// Not a time-series indicator — this is a histogram of volume by price.
#[derive(Debug, Clone)]
pub struct VolumeProfile {
    /// Ordered price buckets from lowest to highest.
    pub buckets: Vec<VolumeProfileBucket>,
    /// Maximum volume across all buckets (for width normalization).
    pub max_volume: f64,
}

impl VolumeProfile {
    /// Compute volume profile from OHLCV data.
    ///
    /// Divides the price range into `num_buckets` equal-width buckets and
    /// distributes each bar's volume proportionally across the buckets its
    /// `[low, high]` range touches.
    #[must_use]
    pub fn compute(data: &[Ohlcv], num_buckets: usize) -> Self {
        if data.is_empty() || num_buckets == 0 {
            return Self {
                buckets: Vec::new(),
                max_volume: 0.0,
            };
        }

        let mut price_min = f64::MAX;
        let mut price_max = f64::MIN;
        for bar in data {
            if bar.low < price_min {
                price_min = bar.low;
            }
            if bar.high > price_max {
                price_max = bar.high;
            }
        }

        let price_span = price_max - price_min;
        if price_span < f64::EPSILON {
            // All bars at same price — single bucket
            return Self {
                buckets: vec![VolumeProfileBucket {
                    price_low: price_min,
                    price_high: price_max,
                    volume: data.iter().map(|b| b.volume).sum(),
                }],
                max_volume: data.iter().map(|b| b.volume).sum(),
            };
        }

        let bucket_width = price_span / num_buckets as f64;
        let mut volumes = vec![0.0_f64; num_buckets];

        // Build buckets
        let buckets: Vec<VolumeProfileBucket> = (0..num_buckets)
            .map(|i| VolumeProfileBucket {
                price_low: price_min + bucket_width * i as f64,
                price_high: price_min + bucket_width * (i + 1) as f64,
                volume: 0.0,
            })
            .collect();

        // Distribute volume proportionally
        for bar in data {
            let bar_low = bar.low.max(price_min);
            let bar_high = bar.high.min(price_max);
            let bar_span = bar_high - bar_low;

            if bar_span < f64::EPSILON {
                // Doji or point bar — add to single bucket
                let idx = ((bar_low - price_min) / bucket_width) as usize;
                let idx = idx.min(num_buckets - 1);
                volumes[idx] += bar.volume;
                continue;
            }

            // Find bucket range this bar touches
            let first = ((bar_low - price_min) / bucket_width) as usize;
            let last = ((bar_high - price_min) / bucket_width).ceil() as usize;
            let first = first.min(num_buckets - 1);
            let last = last.min(num_buckets);

            for (vol, idx) in volumes[first..last].iter_mut().zip(first..last) {
                let b_low = price_min + bucket_width * idx as f64;
                let b_high = b_low + bucket_width;
                let overlap_low = bar_low.max(b_low);
                let overlap_high = bar_high.min(b_high);
                let overlap = (overlap_high - overlap_low).max(0.0);
                let fraction = overlap / bar_span;
                *vol += bar.volume * fraction;
            }
        }

        let max_volume = volumes.iter().copied().fold(0.0_f64, f64::max);

        let buckets = buckets
            .into_iter()
            .zip(volumes)
            .map(|(mut b, v)| {
                b.volume = v;
                b
            })
            .collect();

        Self {
            buckets,
            max_volume,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_data(bars: &[(f64, f64, f64, f64)]) -> Vec<Ohlcv> {
        bars.iter()
            .enumerate()
            .map(|(i, &(h, l, c, v))| Ohlcv {
                timestamp: i as i64,
                open: c,
                high: h,
                low: l,
                close: c,
                volume: v,
                institutional_ratio: 0.0,
            })
            .collect()
    }

    #[test]
    fn empty_data_produces_empty_profile() {
        let vp = VolumeProfile::compute(&[], 10);
        assert!(vp.buckets.is_empty());
        assert_eq!(vp.max_volume, 0.0);
    }

    #[test]
    fn zero_buckets_produces_empty_profile() {
        let data = make_data(&[(110.0, 90.0, 100.0, 1000.0)]);
        let vp = VolumeProfile::compute(&data, 0);
        assert!(vp.buckets.is_empty());
    }

    #[test]
    fn single_bar_distributes_across_buckets() {
        // Bar: high=110, low=90, volume=1000, 2 buckets
        // Bucket 0: [90, 100), Bucket 1: [100, 110]
        // Each gets 50% of volume
        let data = make_data(&[(110.0, 90.0, 100.0, 1000.0)]);
        let vp = VolumeProfile::compute(&data, 2);

        assert_eq!(vp.buckets.len(), 2);
        assert!((vp.buckets[0].volume - 500.0).abs() < 1.0);
        assert!((vp.buckets[1].volume - 500.0).abs() < 1.0);
    }

    #[test]
    fn total_volume_preserved() {
        let data = make_data(&[
            (110.0, 90.0, 100.0, 1000.0),
            (120.0, 95.0, 115.0, 2000.0),
            (105.0, 85.0, 95.0, 1500.0),
        ]);
        let vp = VolumeProfile::compute(&data, 10);

        let total: f64 = vp.buckets.iter().map(|b| b.volume).sum();
        let expected: f64 = data.iter().map(|b| b.volume).sum();
        assert!((total - expected).abs() < 1.0);
    }

    #[test]
    fn max_volume_correct() {
        let data = make_data(&[(110.0, 100.0, 105.0, 5000.0), (120.0, 110.0, 115.0, 1000.0)]);
        let vp = VolumeProfile::compute(&data, 2);

        // First bucket gets most volume (5000 from bar 0)
        assert!(vp.max_volume > 0.0);
        assert!(vp.buckets.iter().all(|b| b.volume <= vp.max_volume));
    }

    #[test]
    fn buckets_cover_full_range() {
        let data = make_data(&[(200.0, 100.0, 150.0, 1000.0)]);
        let vp = VolumeProfile::compute(&data, 5);

        assert_eq!(vp.buckets.len(), 5);
        assert!((vp.buckets[0].price_low - 100.0).abs() < 0.01);
        assert!((vp.buckets[4].price_high - 200.0).abs() < 0.01);
    }

    #[test]
    fn same_price_bars_single_bucket() {
        let data = make_data(&[(100.0, 100.0, 100.0, 500.0), (100.0, 100.0, 100.0, 300.0)]);
        let vp = VolumeProfile::compute(&data, 10);

        assert_eq!(vp.buckets.len(), 1);
        assert!((vp.buckets[0].volume - 800.0).abs() < 0.01);
    }
}
