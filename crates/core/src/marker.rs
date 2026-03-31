// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

/// Shape of a chart marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MarkerShape {
    ArrowUp,
    ArrowDown,
    Circle,
    Diamond,
}

/// Position of the marker relative to the candle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MarkerPosition {
    /// Above the high (for sell signals, bearish patterns).
    AboveBar,
    /// Below the low (for buy signals, bullish patterns).
    BelowBar,
}

/// A single marker on the chart.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Marker {
    /// Bar index in the data array.
    pub bar_index: usize,
    pub shape: MarkerShape,
    pub position: MarkerPosition,
    /// RGBA color (r, g, b, a).
    pub color: (u8, u8, u8, u8),
    /// Short label displayed next to the marker.
    pub label: String,
}

/// Collection of markers for a chart.
#[derive(Debug, Clone, Default)]
pub struct MarkerSet {
    markers: Vec<Marker>,
}

impl MarkerSet {
    #[must_use]
    pub fn new() -> Self {
        Self {
            markers: Vec::new(),
        }
    }

    pub fn add(&mut self, marker: Marker) {
        self.markers.push(marker);
    }

    pub fn clear(&mut self) {
        self.markers.clear();
    }

    /// Get markers that fall within the given bar index range.
    #[must_use]
    pub fn in_range(&self, start: usize, end: usize) -> Vec<&Marker> {
        self.markers
            .iter()
            .filter(|m| m.bar_index >= start && m.bar_index < end)
            .collect()
    }

    /// Find the marker closest to the given bar index (within `tolerance` bars).
    #[must_use]
    pub fn nearest(&self, bar_index: usize, tolerance: usize) -> Option<&Marker> {
        self.markers
            .iter()
            .filter(|m| m.bar_index.abs_diff(bar_index) <= tolerance)
            .min_by_key(|m| m.bar_index.abs_diff(bar_index))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.markers.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.markers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_marker(index: usize, label: &str) -> Marker {
        Marker {
            bar_index: index,
            shape: MarkerShape::ArrowUp,
            position: MarkerPosition::BelowBar,
            color: (0, 255, 0, 255),
            label: label.to_string(),
        }
    }

    #[test]
    fn marker_set_add_and_len() {
        let mut set = MarkerSet::new();
        assert!(set.is_empty());
        set.add(test_marker(5, "Hammer"));
        set.add(test_marker(10, "Doji"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn marker_set_clear() {
        let mut set = MarkerSet::new();
        set.add(test_marker(0, "test"));
        set.clear();
        assert!(set.is_empty());
    }

    #[test]
    fn marker_set_in_range() {
        let mut set = MarkerSet::new();
        set.add(test_marker(5, "A"));
        set.add(test_marker(15, "B"));
        set.add(test_marker(25, "C"));

        let visible = set.in_range(10, 20);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].label, "B");
    }

    #[test]
    fn marker_set_in_range_empty() {
        let set = MarkerSet::new();
        assert!(set.in_range(0, 100).is_empty());
    }

    #[test]
    fn marker_set_in_range_boundaries() {
        let mut set = MarkerSet::new();
        set.add(test_marker(10, "start"));
        set.add(test_marker(19, "end-1"));
        set.add(test_marker(20, "end"));

        let visible = set.in_range(10, 20);
        assert_eq!(visible.len(), 2); // 10 included, 20 excluded
    }

    #[test]
    fn marker_set_nearest() {
        let mut set = MarkerSet::new();
        set.add(test_marker(10, "A"));
        set.add(test_marker(20, "B"));

        assert_eq!(set.nearest(11, 2).unwrap().label, "A");
        assert_eq!(set.nearest(19, 2).unwrap().label, "B");
        assert!(set.nearest(15, 2).is_none());
    }

    #[test]
    fn marker_set_nearest_exact() {
        let mut set = MarkerSet::new();
        set.add(test_marker(5, "exact"));
        assert_eq!(set.nearest(5, 0).unwrap().label, "exact");
    }

    #[test]
    fn marker_shapes() {
        // Ensure all shapes are distinct
        assert_ne!(MarkerShape::ArrowUp, MarkerShape::ArrowDown);
        assert_ne!(MarkerShape::Circle, MarkerShape::Diamond);
    }

    #[test]
    fn marker_positions() {
        assert_ne!(MarkerPosition::AboveBar, MarkerPosition::BelowBar);
    }
}
