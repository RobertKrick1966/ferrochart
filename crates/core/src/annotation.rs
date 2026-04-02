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

/// A horizontal price-level line spanning the full chart width.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HorizontalRay {
    /// Price at which to draw the horizontal line.
    pub price: f64,
    /// RGB color.
    pub color: (u8, u8, u8),
    /// Line width in pixels.
    pub width: f64,
}

/// A vertical line at a specific bar index.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VerticalLine {
    /// Bar index at which to draw the vertical line (fractional for precise placement).
    pub bar_index: f64,
    /// RGB color.
    pub color: (u8, u8, u8),
    /// Line width in pixels.
    pub width: f64,
}

/// A price × time rectangle zone.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RectangleZone {
    /// Bar index of the left edge.
    pub start_bar: f64,
    /// Bar index of the right edge.
    pub end_bar: f64,
    /// Price at the top of the rectangle.
    pub top_price: f64,
    /// Price at the bottom of the rectangle.
    pub bottom_price: f64,
    /// RGB border color.
    pub border_color: (u8, u8, u8),
    /// RGBA fill color (R, G, B, A) with alpha 0–255.
    pub fill_color: (u8, u8, u8, u8),
    /// Border line width in pixels.
    pub width: f64,
}

/// A text label at a specific bar and price.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextLabel {
    /// Bar index at which to place the label.
    pub bar_index: f64,
    /// Price at which to place the label.
    pub price: f64,
    /// Text content.
    pub text: String,
    /// RGB color.
    pub color: (u8, u8, u8),
}

/// A ray extending from start through end to the right chart boundary.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Ray {
    /// Bar index of the start point.
    pub start_bar: f64,
    /// Price at the start point.
    pub start_price: f64,
    /// Bar index of the second point (determines direction).
    pub end_bar: f64,
    /// Price at the second point.
    pub end_price: f64,
    /// Line color.
    pub color: (u8, u8, u8),
    /// Line width.
    pub width: f64,
}

/// Measurement annotation showing Δ price, Δ%, and Δ bars between two points.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MeasurementTool {
    /// Bar index of the first anchor.
    pub start_bar: f64,
    /// Price of the first anchor.
    pub start_price: f64,
    /// Bar index of the second anchor.
    pub end_bar: f64,
    /// Price of the second anchor.
    pub end_price: f64,
    /// Color for the measurement display.
    pub color: (u8, u8, u8),
}

/// Ellipse defined by two anchor points (bounding-box corners).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Ellipse {
    /// Bar index of the first anchor (left).
    pub start_bar: f64,
    /// Price of the first anchor.
    pub start_price: f64,
    /// Bar index of the second anchor (right).
    pub end_bar: f64,
    /// Price of the second anchor.
    pub end_price: f64,
    /// Border color.
    pub color: (u8, u8, u8),
    /// Fill color with alpha.
    pub fill_color: (u8, u8, u8, u8),
    /// Border width.
    pub width: f64,
}

/// Andrews Pitchfork: 3 anchor points define median line + 2 parallel lines.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AndrewsPitchfork {
    /// Bar index of anchor 1 (handle).
    pub bar1: f64,
    /// Price of anchor 1.
    pub price1: f64,
    /// Bar index of anchor 2 (first tine).
    pub bar2: f64,
    /// Price of anchor 2.
    pub price2: f64,
    /// Bar index of anchor 3 (second tine).
    pub bar3: f64,
    /// Price of anchor 3.
    pub price3: f64,
    /// Line color.
    pub color: (u8, u8, u8),
    /// Line width.
    pub width: f64,
}

/// Gann Fan: 8 fan lines from a single anchor point at Gann angles.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GannFan {
    /// Bar index of the anchor point.
    pub anchor_bar: f64,
    /// Price of the anchor point.
    pub anchor_price: f64,
    /// Price units per bar for the 1×1 (45°) line.
    pub scale: f64,
    /// Base color (lines get varying opacity).
    pub color: (u8, u8, u8),
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
    /// Horizontal rays spanning the full chart width.
    pub horizontal_rays: Vec<HorizontalRay>,
    /// Vertical lines at specific bar indices.
    pub vertical_lines: Vec<VerticalLine>,
    /// Price × time rectangle zones.
    pub rectangle_zones: Vec<RectangleZone>,
    /// Text labels at specific bar and price positions.
    pub text_labels: Vec<TextLabel>,
    /// Ray annotations extending from start through end to the right boundary.
    pub rays: Vec<Ray>,
    /// Measurement tool annotations showing Δ price, Δ%, and Δ bars.
    pub measurements: Vec<MeasurementTool>,
    /// Ellipse annotations defined by two bounding-box corners.
    pub ellipses: Vec<Ellipse>,
    /// Andrews Pitchfork annotations.
    pub pitchforks: Vec<AndrewsPitchfork>,
    /// Gann Fan annotations.
    pub gann_fans: Vec<GannFan>,
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

    /// Adds a horizontal ray (full-width price line) to the collection.
    pub fn add_horizontal_ray(&mut self, ray: HorizontalRay) {
        self.horizontal_rays.push(ray);
    }

    /// Adds a vertical line at a specific bar index to the collection.
    pub fn add_vertical_line(&mut self, line: VerticalLine) {
        self.vertical_lines.push(line);
    }

    /// Adds a price × time rectangle zone to the collection.
    pub fn add_rectangle_zone(&mut self, zone: RectangleZone) {
        self.rectangle_zones.push(zone);
    }

    /// Adds a text label at a specific bar and price to the collection.
    pub fn add_text_label(&mut self, label: TextLabel) {
        self.text_labels.push(label);
    }

    /// Adds a ray extending from start through end to the right boundary.
    pub fn add_ray(&mut self, ray: Ray) {
        self.rays.push(ray);
    }

    /// Adds a measurement tool annotation.
    pub fn add_measurement(&mut self, measurement: MeasurementTool) {
        self.measurements.push(measurement);
    }

    /// Adds an ellipse annotation.
    pub fn add_ellipse(&mut self, ellipse: Ellipse) {
        self.ellipses.push(ellipse);
    }

    /// Adds an Andrews Pitchfork annotation.
    pub fn add_pitchfork(&mut self, pitchfork: AndrewsPitchfork) {
        self.pitchforks.push(pitchfork);
    }

    /// Adds a Gann Fan annotation.
    pub fn add_gann_fan(&mut self, fan: GannFan) {
        self.gann_fans.push(fan);
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
        self.horizontal_rays.clear();
        self.vertical_lines.clear();
        self.rectangle_zones.clear();
        self.text_labels.clear();
        self.rays.clear();
        self.measurements.clear();
        self.ellipses.clear();
        self.pitchforks.clear();
        self.gann_fans.clear();
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
            && self.horizontal_rays.is_empty()
            && self.vertical_lines.is_empty()
            && self.rectangle_zones.is_empty()
            && self.text_labels.is_empty()
            && self.rays.is_empty()
            && self.measurements.is_empty()
            && self.ellipses.is_empty()
            && self.pitchforks.is_empty()
            && self.gann_fans.is_empty()
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

    #[test]
    fn horizontal_ray_add_and_clear() {
        let mut ann = Annotations::new();
        ann.add_horizontal_ray(HorizontalRay {
            price: 150.0,
            color: (255, 0, 0),
            width: 1.0,
        });
        assert!(!ann.is_empty());
        assert_eq!(ann.horizontal_rays.len(), 1);
        ann.clear();
        assert!(ann.is_empty());
    }

    #[test]
    fn vertical_line_add_and_clear() {
        let mut ann = Annotations::new();
        ann.add_vertical_line(VerticalLine {
            bar_index: 42.0,
            color: (0, 255, 0),
            width: 1.5,
        });
        assert!(!ann.is_empty());
        assert_eq!(ann.vertical_lines.len(), 1);
        ann.clear();
        assert!(ann.is_empty());
    }

    #[test]
    fn rectangle_zone_add_and_clear() {
        let mut ann = Annotations::new();
        ann.add_rectangle_zone(RectangleZone {
            start_bar: 10.0,
            end_bar: 20.0,
            top_price: 120.0,
            bottom_price: 100.0,
            border_color: (255, 255, 0),
            fill_color: (255, 255, 0, 30),
            width: 1.0,
        });
        assert!(!ann.is_empty());
        assert_eq!(ann.rectangle_zones.len(), 1);
        ann.clear();
        assert!(ann.is_empty());
    }

    #[test]
    fn text_label_add_and_clear() {
        let mut ann = Annotations::new();
        ann.add_text_label(TextLabel {
            bar_index: 5.0,
            price: 110.0,
            text: "Signal".to_string(),
            color: (200, 200, 200),
        });
        assert!(!ann.is_empty());
        assert_eq!(ann.text_labels.len(), 1);
        ann.clear();
        assert!(ann.is_empty());
    }

    #[test]
    fn drawing_tools_is_empty_checks_all_new_fields() {
        let mut ann = Annotations::new();
        // Verify is_empty() respects all new annotation types
        assert!(ann.is_empty());
        ann.add_horizontal_ray(HorizontalRay {
            price: 100.0,
            color: (0, 0, 0),
            width: 1.0,
        });
        assert!(!ann.is_empty());
        ann.clear();
        ann.add_vertical_line(VerticalLine {
            bar_index: 1.0,
            color: (0, 0, 0),
            width: 1.0,
        });
        assert!(!ann.is_empty());
        ann.clear();
        ann.add_rectangle_zone(RectangleZone {
            start_bar: 0.0,
            end_bar: 5.0,
            top_price: 110.0,
            bottom_price: 90.0,
            border_color: (0, 0, 0),
            fill_color: (0, 0, 0, 20),
            width: 1.0,
        });
        assert!(!ann.is_empty());
        ann.clear();
        ann.add_text_label(TextLabel {
            bar_index: 0.0,
            price: 100.0,
            text: "x".to_string(),
            color: (0, 0, 0),
        });
        assert!(!ann.is_empty());
        ann.clear();
        assert!(ann.is_empty());
    }

    #[test]
    fn ray_add_and_clear() {
        let mut ann = Annotations::new();
        assert!(ann.rays.is_empty());
        ann.add_ray(Ray {
            start_bar: 1.0,
            start_price: 100.0,
            end_bar: 10.0,
            end_price: 110.0,
            color: (0, 255, 0),
            width: 1.5,
        });
        assert_eq!(ann.rays.len(), 1);
        assert!(!ann.is_empty());
        ann.clear();
        assert!(ann.rays.is_empty());
        assert!(ann.is_empty());
    }

    #[test]
    fn measurement_add_and_clear() {
        let mut ann = Annotations::new();
        ann.add_measurement(MeasurementTool {
            start_bar: 0.0,
            start_price: 100.0,
            end_bar: 20.0,
            end_price: 120.0,
            color: (255, 200, 0),
        });
        assert_eq!(ann.measurements.len(), 1);
        let m = &ann.measurements[0];
        assert!((m.end_price - m.start_price - 20.0).abs() < f64::EPSILON);
        ann.clear();
        assert!(ann.measurements.is_empty());
    }

    #[test]
    fn ellipse_add_and_clear() {
        let mut ann = Annotations::new();
        ann.add_ellipse(Ellipse {
            start_bar: 5.0,
            start_price: 90.0,
            end_bar: 15.0,
            end_price: 110.0,
            color: (0, 200, 100),
            fill_color: (0, 200, 100, 25),
            width: 1.5,
        });
        assert_eq!(ann.ellipses.len(), 1);
        ann.clear();
        assert!(ann.ellipses.is_empty());
    }

    #[test]
    fn pitchfork_add_and_clear() {
        let mut ann = Annotations::new();
        ann.add_pitchfork(AndrewsPitchfork {
            bar1: 2.0,
            price1: 95.0,
            bar2: 10.0,
            price2: 110.0,
            bar3: 18.0,
            price3: 100.0,
            color: (255, 165, 0),
            width: 1.5,
        });
        assert_eq!(ann.pitchforks.len(), 1);
        let p = &ann.pitchforks[0];
        // midpoint of bar2/bar3 = 14.0, midpoint of price2/price3 = 105.0
        let mid_bar = f64::midpoint(p.bar2, p.bar3);
        let mid_price = f64::midpoint(p.price2, p.price3);
        assert!((mid_bar - 14.0).abs() < f64::EPSILON);
        assert!((mid_price - 105.0).abs() < f64::EPSILON);
        ann.clear();
        assert!(ann.pitchforks.is_empty());
    }

    #[test]
    fn gann_fan_add_and_clear() {
        let mut ann = Annotations::new();
        ann.add_gann_fan(GannFan {
            anchor_bar: 5.0,
            anchor_price: 100.0,
            scale: 2.0,
            color: (200, 100, 255),
        });
        assert_eq!(ann.gann_fans.len(), 1);
        assert!(!ann.is_empty());
        ann.clear();
        assert!(ann.gann_fans.is_empty());
        assert!(ann.is_empty());
    }

    #[test]
    fn all_prio2_tools_is_empty_coverage() {
        // Verify is_empty() checks all 5 new fields
        let mut ann = Annotations::new();
        ann.add_ray(Ray {
            start_bar: 0.0,
            start_price: 100.0,
            end_bar: 5.0,
            end_price: 105.0,
            color: (0, 0, 0),
            width: 1.0,
        });
        assert!(!ann.is_empty());
        ann.clear();
        ann.add_measurement(MeasurementTool {
            start_bar: 0.0,
            start_price: 100.0,
            end_bar: 5.0,
            end_price: 105.0,
            color: (0, 0, 0),
        });
        assert!(!ann.is_empty());
        ann.clear();
        ann.add_ellipse(Ellipse {
            start_bar: 0.0,
            start_price: 90.0,
            end_bar: 10.0,
            end_price: 110.0,
            color: (0, 0, 0),
            fill_color: (0, 0, 0, 20),
            width: 1.0,
        });
        assert!(!ann.is_empty());
        ann.clear();
        ann.add_pitchfork(AndrewsPitchfork {
            bar1: 0.0,
            price1: 90.0,
            bar2: 5.0,
            price2: 110.0,
            bar3: 10.0,
            price3: 95.0,
            color: (0, 0, 0),
            width: 1.0,
        });
        assert!(!ann.is_empty());
        ann.clear();
        ann.add_gann_fan(GannFan {
            anchor_bar: 0.0,
            anchor_price: 100.0,
            scale: 1.0,
            color: (0, 0, 0),
        });
        assert!(!ann.is_empty());
        ann.clear();
        assert!(ann.is_empty());
    }
}
