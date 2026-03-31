// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use crate::Rect;

/// A single panel in the chart layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Panel {
    pub index: usize,
    pub weight: f64,
    pub rect: Rect,
}

/// Splits total chart area into weighted horizontal panels.
#[derive(Debug, Clone)]
pub struct PanelLayout {
    panels: Vec<Panel>,
}

impl PanelLayout {
    /// Create a layout from weights (e.g. `[60, 20, 10, 10]`).
    ///
    /// Weights are relative — they will be normalized to sum to 1.0.
    /// `total_rect` is the full chart area to subdivide vertically.
    /// `gap` is the pixel spacing between adjacent panels.
    ///
    /// # Panics
    ///
    /// Panics if `weights` is empty or the sum of weights is zero.
    #[must_use]
    pub fn new(weights: &[f64], total_rect: Rect, gap: f64) -> Self {
        assert!(!weights.is_empty(), "weights must not be empty");

        let weight_sum: f64 = weights.iter().sum();
        assert!(weight_sum > f64::EPSILON, "sum of weights must be positive");

        let num_gaps = weights.len().saturating_sub(1);
        #[allow(clippy::cast_precision_loss)]
        let total_gap = gap * num_gaps as f64;
        let available_height = (total_rect.height - total_gap).max(0.0);

        let mut panels = Vec::with_capacity(weights.len());
        let mut y = total_rect.y;

        for (i, &w) in weights.iter().enumerate() {
            let fraction = w / weight_sum;
            let height = available_height * fraction;
            panels.push(Panel {
                index: i,
                weight: w,
                rect: Rect::new(total_rect.x, y, total_rect.width, height),
            });
            y += height + gap;
        }

        Self { panels }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.panels.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.panels.is_empty()
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Panel> {
        self.panels.get(index)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Panel> {
        self.panels.iter()
    }
}

impl<'a> IntoIterator for &'a PanelLayout {
    type Item = &'a Panel;
    type IntoIter = std::slice::Iter<'a, Panel>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl PanelLayout {
    /// Find which panel contains the given Y coordinate.
    /// Returns `None` if Y is in a gap or outside the layout.
    #[must_use]
    pub fn panel_at_y(&self, y: f64) -> Option<&Panel> {
        self.panels
            .iter()
            .find(|p| y >= p.rect.y && y <= p.rect.bottom())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_panel_layout() {
        let total = Rect::new(0.0, 0.0, 800.0, 1000.0);
        let layout = PanelLayout::new(&[60.0, 20.0, 10.0, 10.0], total, 2.0);

        assert_eq!(layout.len(), 4);

        // Total gap = 3 * 2 = 6px, available = 994px
        let available = 994.0;
        let p0 = layout.get(0).unwrap();
        assert!((p0.rect.height - available * 0.6).abs() < 1e-9);

        let p1 = layout.get(1).unwrap();
        assert!((p1.rect.height - available * 0.2).abs() < 1e-9);

        let p2 = layout.get(2).unwrap();
        assert!((p2.rect.height - available * 0.1).abs() < 1e-9);

        let p3 = layout.get(3).unwrap();
        assert!((p3.rect.height - available * 0.1).abs() < 1e-9);
    }

    #[test]
    fn panel_heights_plus_gaps_equal_total() {
        let total = Rect::new(0.0, 0.0, 800.0, 1000.0);
        let gap = 4.0;
        let layout = PanelLayout::new(&[3.0, 2.0, 1.0], total, gap);

        let sum_heights: f64 = layout.iter().map(|p| p.rect.height).sum();
        let total_gaps = gap * 2.0;
        assert!((sum_heights + total_gaps - 1000.0).abs() < 1e-9);
    }

    #[test]
    fn panels_do_not_overlap() {
        let total = Rect::new(0.0, 0.0, 800.0, 500.0);
        let layout = PanelLayout::new(&[50.0, 30.0, 20.0], total, 2.0);

        for pair in layout.panels.windows(2) {
            let bottom_prev = pair[0].rect.bottom();
            let top_next = pair[1].rect.y;
            assert!(top_next >= bottom_prev, "panels overlap");
        }
    }

    #[test]
    fn single_panel_fills_entire_rect() {
        let total = Rect::new(10.0, 20.0, 600.0, 400.0);
        let layout = PanelLayout::new(&[1.0], total, 5.0);

        assert_eq!(layout.len(), 1);
        let p = layout.get(0).unwrap();
        assert!((p.rect.x - 10.0).abs() < f64::EPSILON);
        assert!((p.rect.y - 20.0).abs() < f64::EPSILON);
        assert!((p.rect.width - 600.0).abs() < f64::EPSILON);
        assert!((p.rect.height - 400.0).abs() < f64::EPSILON);
    }

    #[test]
    fn panel_at_y_finds_correct_panel() {
        let total = Rect::new(0.0, 0.0, 800.0, 100.0);
        let layout = PanelLayout::new(&[50.0, 50.0], total, 2.0);

        let p0 = layout.get(0).unwrap();
        let p1 = layout.get(1).unwrap();

        // Point in first panel
        let found = layout.panel_at_y(p0.rect.y + 1.0).unwrap();
        assert_eq!(found.index, 0);

        // Point in second panel
        let found = layout.panel_at_y(p1.rect.y + 1.0).unwrap();
        assert_eq!(found.index, 1);
    }

    #[test]
    fn panel_at_y_returns_none_in_gap() {
        let total = Rect::new(0.0, 0.0, 800.0, 100.0);
        let layout = PanelLayout::new(&[50.0, 50.0], total, 4.0);

        let p0 = layout.get(0).unwrap();
        let gap_y = p0.rect.bottom() + 2.0; // middle of the gap
        assert!(layout.panel_at_y(gap_y).is_none());
    }

    #[test]
    fn panel_at_y_returns_none_outside() {
        let total = Rect::new(0.0, 100.0, 800.0, 200.0);
        let layout = PanelLayout::new(&[1.0], total, 0.0);

        assert!(layout.panel_at_y(50.0).is_none());
        assert!(layout.panel_at_y(350.0).is_none());
    }

    #[test]
    fn equal_weights_produce_equal_heights() {
        let total = Rect::new(0.0, 0.0, 800.0, 300.0);
        let layout = PanelLayout::new(&[1.0, 1.0, 1.0], total, 0.0);

        for panel in &layout {
            assert!((panel.rect.height - 100.0).abs() < 1e-9);
        }
    }

    #[test]
    #[should_panic(expected = "weights must not be empty")]
    fn empty_weights_panics() {
        let _ = PanelLayout::new(&[], Rect::new(0.0, 0.0, 800.0, 600.0), 0.0);
    }

    #[test]
    fn panel_indices_are_sequential() {
        let total = Rect::new(0.0, 0.0, 800.0, 600.0);
        let layout = PanelLayout::new(&[3.0, 2.0, 1.0], total, 2.0);

        for (i, panel) in layout.iter().enumerate() {
            assert_eq!(panel.index, i);
        }
    }
}
