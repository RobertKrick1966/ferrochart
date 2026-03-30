/// A single OHLCV bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ohlcv {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// A generic indexed series (thin wrapper over `Vec<T>`).
#[derive(Debug, Clone)]
pub struct Series<T> {
    values: Vec<T>,
}

impl<T> Series<T> {
    #[must_use]
    pub fn new(values: Vec<T>) -> Self {
        Self { values }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.values.get(index)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.values.iter()
    }
}

impl<'a, T> IntoIterator for &'a Series<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T> Series<T> {

    /// Return a sub-slice. Panics if `range` is out of bounds.
    #[must_use]
    pub fn slice(&self, range: std::ops::Range<usize>) -> &[T] {
        &self.values[range]
    }
}

/// Inclusive price range `[min, max]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PriceRange {
    pub min: f64,
    pub max: f64,
}

impl PriceRange {
    #[must_use]
    pub fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }

    /// Distance between max and min.
    #[must_use]
    pub fn span(&self) -> f64 {
        self.max - self.min
    }

    /// Expand the range symmetrically by a fraction (e.g. 0.05 = 5% padding).
    #[must_use]
    pub fn with_padding(self, fraction: f64) -> Self {
        let pad = self.span() * fraction;
        Self {
            min: self.min - pad,
            max: self.max + pad,
        }
    }

    /// Compute the price range from a slice of OHLCV bars.
    /// Returns `None` if the slice is empty.
    #[must_use]
    pub fn from_ohlcv(data: &[Ohlcv]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        for bar in data {
            if bar.low < min {
                min = bar.low;
            }
            if bar.high > max {
                max = bar.high;
            }
        }
        Some(Self { min, max })
    }
}

/// Half-open index range `[start, end)` into a data series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    pub start: usize,
    pub end: usize,
}

impl TimeRange {
    #[must_use]
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bars() -> Vec<Ohlcv> {
        vec![
            Ohlcv { timestamp: 1, open: 100.0, high: 110.0, low: 95.0, close: 105.0, volume: 1000.0 },
            Ohlcv { timestamp: 2, open: 105.0, high: 120.0, low: 100.0, close: 115.0, volume: 1500.0 },
            Ohlcv { timestamp: 3, open: 115.0, high: 118.0, low: 90.0, close: 92.0, volume: 2000.0 },
        ]
    }

    // --- Series tests ---

    #[test]
    fn series_len_and_empty() {
        let s: Series<f64> = Series::new(vec![1.0, 2.0, 3.0]);
        assert_eq!(s.len(), 3);
        assert!(!s.is_empty());

        let empty: Series<f64> = Series::new(vec![]);
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn series_get() {
        let s = Series::new(vec![10, 20, 30]);
        assert_eq!(s.get(0), Some(&10));
        assert_eq!(s.get(2), Some(&30));
        assert_eq!(s.get(3), None);
    }

    #[test]
    fn series_iter() {
        let s = Series::new(vec![1.0, 2.0, 3.0]);
        let sum: f64 = s.iter().sum();
        assert!((sum - 6.0).abs() < f64::EPSILON);
    }

    #[test]
    fn series_slice() {
        let s = Series::new(vec![10, 20, 30, 40, 50]);
        assert_eq!(s.slice(1..4), &[20, 30, 40]);
    }

    #[test]
    #[should_panic(expected = "range end index 5 out of range")]
    fn series_slice_out_of_bounds() {
        let s = Series::new(vec![1, 2, 3]);
        let _ = s.slice(0..5);
    }

    // --- PriceRange tests ---

    #[test]
    fn price_range_from_ohlcv() {
        let bars = sample_bars();
        let range = PriceRange::from_ohlcv(&bars).unwrap();
        assert!((range.min - 90.0).abs() < f64::EPSILON);
        assert!((range.max - 120.0).abs() < f64::EPSILON);
    }

    #[test]
    fn price_range_from_empty() {
        assert!(PriceRange::from_ohlcv(&[]).is_none());
    }

    #[test]
    fn price_range_span() {
        let r = PriceRange::new(100.0, 200.0);
        assert!((r.span() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn price_range_with_padding() {
        let r = PriceRange::new(100.0, 200.0);
        let padded = r.with_padding(0.1);
        assert!((padded.min - 90.0).abs() < f64::EPSILON);
        assert!((padded.max - 210.0).abs() < f64::EPSILON);
    }

    #[test]
    fn price_range_zero_span_padding() {
        let r = PriceRange::new(100.0, 100.0);
        let padded = r.with_padding(0.1);
        assert!((padded.min - 100.0).abs() < f64::EPSILON);
        assert!((padded.max - 100.0).abs() < f64::EPSILON);
    }

    // --- TimeRange tests ---

    #[test]
    fn time_range_len() {
        let tr = TimeRange::new(5, 15);
        assert_eq!(tr.len(), 10);
        assert!(!tr.is_empty());
    }

    #[test]
    fn time_range_empty() {
        assert!(TimeRange::new(5, 5).is_empty());
        assert!(TimeRange::new(10, 5).is_empty());
    }

    #[test]
    fn time_range_zero_start() {
        let tr = TimeRange::new(0, 100);
        assert_eq!(tr.len(), 100);
    }
}
