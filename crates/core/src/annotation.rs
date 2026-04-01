// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

/// A trendline drawn between two points on the chart.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TrendLine {
    /// Start bar index (can be fractional for precise placement).
    pub start_bar: f64,
    /// Start price.
    pub start_price: f64,
    /// End bar index.
    pub end_bar: f64,
    /// End price.
    pub end_price: f64,
    /// RGB color.
    pub color: (u8, u8, u8),
    /// Line width in pixels.
    pub width: f64,
    /// Whether to extend the line beyond the endpoints.
    pub extend_right: bool,
}

/// Fibonacci retracement levels drawn between a high and low point.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FibonacciRetracement {
    /// Bar index of the high point.
    pub high_bar: usize,
    /// Price at the high point.
    pub high_price: f64,
    /// Bar index of the low point.
    pub low_bar: usize,
    /// Price at the low point.
    pub low_price: f64,
    /// RGB color.
    pub color: (u8, u8, u8),
}

impl FibonacciRetracement {
    /// Standard Fibonacci levels.
    pub const LEVELS: [f64; 7] = [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0];

    /// Compute the price at each Fibonacci level.
    #[must_use]
    pub fn level_prices(&self) -> Vec<(f64, f64)> {
        let range = self.high_price - self.low_price;
        Self::LEVELS
            .iter()
            .map(|&level| (level, self.high_price - range * level))
            .collect()
    }
}

/// A corridor: two parallel trendlines.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Corridor {
    /// The primary trendline.
    pub line: TrendLine,
    /// Price offset for the parallel line (positive = above, negative = below).
    pub offset: f64,
}

/// Which barrier was hit in a triple barrier label.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BarrierOutcome {
    /// Take-profit barrier was hit first.
    TakeProfit,
    /// Stop-loss barrier was hit first.
    StopLoss,
    /// Time horizon expired without hitting TP or SL.
    TimeExpired,
}

/// Triple barrier overlay: visualises TP, SL and time-limit around an entry bar.
///
/// Based on López de Prado, *AFML* Ch. 3.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TripleBarrier {
    /// Bar index of the trade entry.
    pub entry_bar: usize,
    /// Entry price (typically close of entry bar).
    pub entry_price: f64,
    /// Take-profit price level (above entry for long).
    pub tp_price: f64,
    /// Stop-loss price level (below entry for long).
    pub sl_price: f64,
    /// Maximum number of bars until time barrier.
    pub horizon: usize,
    /// Bar index where the trade exited (if known).
    pub exit_bar: Option<usize>,
    /// Which barrier was hit.
    pub outcome: Option<BarrierOutcome>,
    /// RGB color for the overlay.
    pub color: (u8, u8, u8),
}

/// ML confidence band overlay on the price panel.
///
/// Renders a semi-transparent band between `upper` and `lower` values per bar,
/// typically representing prediction confidence intervals.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConfidenceBand {
    /// Upper bound values, one per bar. `NaN` entries are skipped.
    pub upper: Vec<f64>,
    /// Lower bound values, one per bar. `NaN` entries are skipped.
    pub lower: Vec<f64>,
    /// RGB color for the band fill.
    pub color: (u8, u8, u8),
    /// Alpha for the band fill (0–255).
    pub alpha: u8,
}

/// Walk-forward validation zone — marks a train or test time range.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WalkForwardZone {
    /// First bar index of the zone.
    pub start_bar: usize,
    /// Last bar index of the zone (exclusive).
    pub end_bar: usize,
    /// `true` = training zone, `false` = validation/test zone.
    pub is_train: bool,
    /// Optional fold label (e.g. "Fold 1").
    pub label: String,
    /// RGB color override. If `None`, uses default train (blue) / val (orange).
    pub color: Option<(u8, u8, u8)>,
}

/// A news or event marker at a specific bar.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NewsEvent {
    /// Bar index of the event.
    pub bar_index: usize,
    /// Short label (e.g. "Earnings", "FDA", "FOMC").
    pub label: String,
    /// Impact score: -1.0 (bearish) to +1.0 (bullish), 0.0 = neutral.
    pub impact: f64,
    /// Urgency: 0 = low, 1 = medium, 2 = high, 3 = critical.
    pub urgency: u8,
    /// RGB color override. If `None`, color is derived from `impact`.
    pub color: Option<(u8, u8, u8)>,
}

/// Horizontal histogram overlay on the price panel (e.g. GEX profile).
///
/// Renders horizontal bars at price levels, similar to Volume Profile
/// but driven by external data (not computed from OHLCV volume).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HorizontalHistogram {
    /// Price levels and their values (e.g. gamma exposure at each strike).
    pub levels: Vec<(f64, f64)>,
    /// Label for the histogram (e.g. "GEX").
    pub label: String,
    /// RGB color for the bars.
    pub color: (u8, u8, u8),
    /// Alpha for the bars (0-255).
    pub alpha: u8,
}

/// A horizontal price level line (e.g. Max Pain, support/resistance).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HorizontalLevel {
    /// Price at which to draw the line.
    pub price: f64,
    /// Label (e.g. "Max Pain $150.00").
    pub label: String,
    /// RGB color.
    pub color: (u8, u8, u8),
    /// Line width in pixels.
    pub width: f64,
}

/// Collection of annotations on a chart.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Annotations {
    /// All trend lines on the chart.
    pub trend_lines: Vec<TrendLine>,
    /// All corridors (parallel channel pairs) on the chart.
    pub corridors: Vec<Corridor>,
    /// All Fibonacci retracement overlays on the chart.
    pub fibonaccis: Vec<FibonacciRetracement>,
    /// All triple barrier overlays on the chart.
    pub triple_barriers: Vec<TripleBarrier>,
    /// ML confidence bands on the price panel.
    pub confidence_bands: Vec<ConfidenceBand>,
    /// Walk-forward train/validation zones.
    pub walk_forward_zones: Vec<WalkForwardZone>,
    /// News/event markers.
    pub news_events: Vec<NewsEvent>,
    /// Horizontal histograms (GEX profile, etc.) on the price panel.
    pub horizontal_histograms: Vec<HorizontalHistogram>,
    /// Horizontal price level lines (Max Pain, support/resistance).
    pub horizontal_levels: Vec<HorizontalLevel>,
}

impl Annotations {
    /// Creates an empty annotations collection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a trend line to the collection.
    pub fn add_trend_line(&mut self, line: TrendLine) {
        self.trend_lines.push(line);
    }

    /// Adds a corridor to the collection.
    pub fn add_corridor(&mut self, corridor: Corridor) {
        self.corridors.push(corridor);
    }

    /// Adds a Fibonacci retracement to the collection.
    pub fn add_fibonacci(&mut self, fib: FibonacciRetracement) {
        self.fibonaccis.push(fib);
    }

    /// Adds a triple barrier overlay to the collection.
    pub fn add_triple_barrier(&mut self, tb: TripleBarrier) {
        self.triple_barriers.push(tb);
    }

    /// Adds an ML confidence band to the collection.
    pub fn add_confidence_band(&mut self, band: ConfidenceBand) {
        self.confidence_bands.push(band);
    }

    /// Adds a walk-forward zone to the collection.
    pub fn add_walk_forward_zone(&mut self, zone: WalkForwardZone) {
        self.walk_forward_zones.push(zone);
    }

    /// Adds a news event marker to the collection.
    pub fn add_news_event(&mut self, event: NewsEvent) {
        self.news_events.push(event);
    }

    /// Adds a horizontal histogram (e.g. GEX profile) to the collection.
    pub fn add_horizontal_histogram(&mut self, hist: HorizontalHistogram) {
        self.horizontal_histograms.push(hist);
    }

    /// Adds a horizontal price level line (e.g. Max Pain) to the collection.
    pub fn add_horizontal_level(&mut self, level: HorizontalLevel) {
        self.horizontal_levels.push(level);
    }

    /// Removes all annotations.
    pub fn clear(&mut self) {
        self.trend_lines.clear();
        self.corridors.clear();
        self.fibonaccis.clear();
        self.triple_barriers.clear();
        self.confidence_bands.clear();
        self.walk_forward_zones.clear();
        self.news_events.clear();
        self.horizontal_histograms.clear();
        self.horizontal_levels.clear();
    }

    /// Returns `true` if there are no annotations of any kind.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.trend_lines.is_empty()
            && self.corridors.is_empty()
            && self.fibonaccis.is_empty()
            && self.triple_barriers.is_empty()
            && self.confidence_bands.is_empty()
            && self.walk_forward_zones.is_empty()
            && self.news_events.is_empty()
            && self.horizontal_histograms.is_empty()
            && self.horizontal_levels.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fibonacci_levels_count() {
        let fib = FibonacciRetracement {
            high_bar: 10,
            high_price: 200.0,
            low_bar: 0,
            low_price: 100.0,
            color: (255, 165, 0),
        };
        let levels = fib.level_prices();
        assert_eq!(levels.len(), 7);
    }

    #[test]
    fn fibonacci_level_prices() {
        let fib = FibonacciRetracement {
            high_bar: 10,
            high_price: 200.0,
            low_bar: 0,
            low_price: 100.0,
            color: (255, 165, 0),
        };
        let levels = fib.level_prices();
        // Level 0.0 → high price (200)
        assert!((levels[0].1 - 200.0).abs() < 1e-9);
        // Level 1.0 → low price (100)
        assert!((levels[6].1 - 100.0).abs() < 1e-9);
        // Level 0.5 → midpoint (150)
        assert!((levels[3].1 - 150.0).abs() < 1e-9);
        // Level 0.618 → 200 - 100*0.618 = 138.2
        assert!((levels[4].1 - 138.2).abs() < 1e-9);
    }

    #[test]
    fn fibonacci_inverted_range() {
        // Low above high (downtrend retracement)
        let fib = FibonacciRetracement {
            high_bar: 0,
            high_price: 100.0,
            low_bar: 10,
            low_price: 200.0,
            color: (0, 0, 255),
        };
        let levels = fib.level_prices();
        // Level 0.0 → 100, Level 1.0 → 200
        assert!((levels[0].1 - 100.0).abs() < 1e-9);
        assert!((levels[6].1 - 200.0).abs() < 1e-9);
    }

    #[test]
    fn annotations_add_and_clear() {
        let mut ann = Annotations::new();
        assert!(ann.is_empty());

        ann.add_trend_line(TrendLine {
            start_bar: 0.0,
            start_price: 100.0,
            end_bar: 10.0,
            end_price: 110.0,
            color: (255, 255, 0),
            width: 1.5,
            extend_right: false,
        });
        ann.add_fibonacci(FibonacciRetracement {
            high_bar: 10,
            high_price: 200.0,
            low_bar: 0,
            low_price: 100.0,
            color: (255, 165, 0),
        });
        assert!(!ann.is_empty());

        ann.clear();
        assert!(ann.is_empty());
    }

    #[test]
    fn trend_line_extend_right() {
        let line = TrendLine {
            start_bar: 5.0,
            start_price: 100.0,
            end_bar: 15.0,
            end_price: 120.0,
            color: (0, 255, 0),
            width: 2.0,
            extend_right: true,
        };
        // Slope = (120-100) / (15-5) = 2.0 per bar
        let slope = (line.end_price - line.start_price) / (line.end_bar - line.start_bar);
        assert!((slope - 2.0).abs() < 1e-9);
        // At bar 25: 120 + 2*(25-15) = 140
        let projected = line.end_price + slope * (25.0 - line.end_bar);
        assert!((projected - 140.0).abs() < 1e-9);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn annotations_serde_roundtrip() {
        let mut ann = Annotations::new();
        ann.add_trend_line(TrendLine {
            start_bar: 5.0,
            start_price: 100.0,
            end_bar: 15.0,
            end_price: 120.0,
            color: (255, 255, 0),
            width: 2.0,
            extend_right: true,
        });
        ann.add_corridor(Corridor {
            line: TrendLine {
                start_bar: 2.0,
                start_price: 90.0,
                end_bar: 20.0,
                end_price: 130.0,
                color: (0, 200, 255),
                width: 1.0,
                extend_right: false,
            },
            offset: 5.0,
        });
        ann.add_fibonacci(FibonacciRetracement {
            high_bar: 10,
            high_price: 200.0,
            low_bar: 3,
            low_price: 100.0,
            color: (255, 165, 0),
        });

        let json = serde_json::to_string(&ann).unwrap();
        let restored: Annotations = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.trend_lines.len(), 1);
        assert_eq!(restored.corridors.len(), 1);
        assert_eq!(restored.fibonaccis.len(), 1);
        assert!((restored.trend_lines[0].start_price - 100.0).abs() < 1e-9);
        assert!(restored.trend_lines[0].extend_right);
        assert!((restored.corridors[0].offset - 5.0).abs() < 1e-9);
        assert_eq!(restored.fibonaccis[0].high_bar, 10);
    }
}
