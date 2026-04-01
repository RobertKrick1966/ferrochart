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

    /// Removes all annotations.
    pub fn clear(&mut self) {
        self.trend_lines.clear();
        self.corridors.clear();
        self.fibonaccis.clear();
    }

    /// Returns `true` if there are no annotations of any kind.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.trend_lines.is_empty() && self.corridors.is_empty() && self.fibonaccis.is_empty()
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
