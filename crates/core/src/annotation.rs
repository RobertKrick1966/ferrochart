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

/// Collection of annotations on a chart.
#[derive(Debug, Clone, Default)]
pub struct Annotations {
    pub trend_lines: Vec<TrendLine>,
    pub fibonaccis: Vec<FibonacciRetracement>,
}

impl Annotations {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_trend_line(&mut self, line: TrendLine) {
        self.trend_lines.push(line);
    }

    pub fn add_fibonacci(&mut self, fib: FibonacciRetracement) {
        self.fibonaccis.push(fib);
    }

    pub fn clear(&mut self) {
        self.trend_lines.clear();
        self.fibonaccis.clear();
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.trend_lines.is_empty() && self.fibonaccis.is_empty()
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
            start_bar: 0.0, start_price: 100.0,
            end_bar: 10.0, end_price: 110.0,
            color: (255, 255, 0), width: 1.5, extend_right: false,
        });
        ann.add_fibonacci(FibonacciRetracement {
            high_bar: 10, high_price: 200.0,
            low_bar: 0, low_price: 100.0,
            color: (255, 165, 0),
        });
        assert!(!ann.is_empty());

        ann.clear();
        assert!(ann.is_empty());
    }

    #[test]
    fn trend_line_extend_right() {
        let line = TrendLine {
            start_bar: 5.0, start_price: 100.0,
            end_bar: 15.0, end_price: 120.0,
            color: (0, 255, 0), width: 2.0, extend_right: true,
        };
        // Slope = (120-100) / (15-5) = 2.0 per bar
        let slope = (line.end_price - line.start_price) / (line.end_bar - line.start_bar);
        assert!((slope - 2.0).abs() < 1e-9);
        // At bar 25: 120 + 2*(25-15) = 140
        let projected = line.end_price + slope * (25.0 - line.end_bar);
        assert!((projected - 140.0).abs() < 1e-9);
    }
}
