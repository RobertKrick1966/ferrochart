// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use crate::TimeRange;

/// Minimum number of visible bars (prevents zooming in too far).
const MIN_VISIBLE_BARS: usize = 5;

/// Tracks the user's current zoom level and horizontal scroll position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoomPanState {
    /// Number of bars visible at current zoom level.
    pub visible_bars: usize,
    /// Index of the first visible bar.
    pub offset: usize,
    /// Total number of bars in the dataset.
    pub total_bars: usize,
    /// Extra empty bars beyond the data (for trendline visibility).
    /// Defaults to 0. Set to e.g. `visible_bars / 3` to allow right-scrolling.
    pub future_bars: usize,
}

impl ZoomPanState {
    /// Create a new state showing the last `visible_bars` bars.
    #[must_use]
    pub fn new(total_bars: usize, visible_bars: usize) -> Self {
        let visible = visible_bars.clamp(MIN_VISIBLE_BARS, total_bars.max(MIN_VISIBLE_BARS));
        let offset = total_bars.saturating_sub(visible);
        Self {
            visible_bars: visible,
            offset,
            total_bars,
            future_bars: 0,
        }
    }

    /// Create a state that allows scrolling past the last bar.
    #[must_use]
    pub fn with_future_bars(mut self, future_bars: usize) -> Self {
        self.future_bars = future_bars;
        self
    }

    /// The currently visible time range.
    #[must_use]
    pub fn visible_range(&self) -> TimeRange {
        let end = (self.offset + self.visible_bars).min(self.total_bars);
        TimeRange::new(self.offset, end)
    }

    /// Zoom in/out by a factor. `factor > 1.0` zooms in (fewer bars),
    /// `factor < 1.0` zooms out (more bars).
    /// `anchor` is the bar index that should stay roughly fixed on screen.
    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn zoom(&self, factor: f64, anchor: usize) -> Self {
        let new_visible = (self.visible_bars as f64 / factor).round() as usize;
        let new_visible = new_visible.clamp(MIN_VISIBLE_BARS, self.total_bars);

        // Keep the anchor at the same relative position.
        let anchor_frac = if self.visible_bars > 0 {
            (anchor.saturating_sub(self.offset)) as f64 / self.visible_bars as f64
        } else {
            0.5
        };

        let new_offset = anchor.saturating_sub((new_visible as f64 * anchor_frac).round() as usize);

        Self {
            visible_bars: new_visible,
            offset: new_offset,
            total_bars: self.total_bars,
            future_bars: self.future_bars,
        }
        .clamped()
    }

    /// Pan by a signed number of bars (positive = scroll right).
    #[must_use]
    pub fn pan(&self, delta: isize) -> Self {
        let new_offset = if delta >= 0 {
            self.offset.saturating_add(delta.cast_unsigned())
        } else {
            self.offset.saturating_sub(delta.unsigned_abs())
        };

        Self {
            offset: new_offset,
            ..*self
        }
        .clamped()
    }

    /// Jump to show the latest bars (scroll to end).
    #[must_use]
    pub fn scroll_to_end(&self) -> Self {
        Self {
            offset: self.total_bars.saturating_sub(self.visible_bars),
            ..*self
        }
    }

    fn clamped(mut self) -> Self {
        self.visible_bars = self
            .visible_bars
            .clamp(MIN_VISIBLE_BARS, self.total_bars.max(MIN_VISIBLE_BARS));
        let max_offset = (self.total_bars + self.future_bars).saturating_sub(self.visible_bars);
        self.offset = self.offset.min(max_offset);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_shows_last_bars() {
        let state = ZoomPanState::new(100, 20);
        assert_eq!(state.visible_bars, 20);
        assert_eq!(state.offset, 80);
        assert_eq!(state.total_bars, 100);
    }

    #[test]
    fn new_clamps_visible_bars_to_total() {
        let state = ZoomPanState::new(10, 50);
        assert_eq!(state.visible_bars, 10);
        assert_eq!(state.offset, 0);
    }

    #[test]
    fn new_enforces_minimum_visible() {
        let state = ZoomPanState::new(100, 2);
        assert_eq!(state.visible_bars, MIN_VISIBLE_BARS);
    }

    #[test]
    fn visible_range() {
        let state = ZoomPanState::new(100, 20);
        let range = state.visible_range();
        assert_eq!(range.start, 80);
        assert_eq!(range.end, 100);
        assert_eq!(range.len(), 20);
    }

    #[test]
    fn pan_right() {
        let state = ZoomPanState {
            visible_bars: 20,
            offset: 50,
            total_bars: 100, future_bars: 0,
        };
        let panned = state.pan(10);
        assert_eq!(panned.offset, 60);
        assert_eq!(panned.visible_bars, 20);
    }

    #[test]
    fn pan_right_clamps_at_end() {
        let state = ZoomPanState {
            visible_bars: 20,
            offset: 75,
            total_bars: 100, future_bars: 0,
        };
        let panned = state.pan(20);
        assert_eq!(panned.offset, 80); // max = 100 - 20
    }

    #[test]
    fn pan_left() {
        let state = ZoomPanState {
            visible_bars: 20,
            offset: 50,
            total_bars: 100, future_bars: 0,
        };
        let panned = state.pan(-10);
        assert_eq!(panned.offset, 40);
    }

    #[test]
    fn pan_left_clamps_at_zero() {
        let state = ZoomPanState {
            visible_bars: 20,
            offset: 5,
            total_bars: 100, future_bars: 0,
        };
        let panned = state.pan(-20);
        assert_eq!(panned.offset, 0);
    }

    #[test]
    fn zoom_in_reduces_visible_bars() {
        let state = ZoomPanState {
            visible_bars: 100,
            offset: 0,
            total_bars: 200, future_bars: 0,
        };
        let zoomed = state.zoom(2.0, 50);
        assert_eq!(zoomed.visible_bars, 50);
    }

    #[test]
    fn zoom_out_increases_visible_bars() {
        let state = ZoomPanState {
            visible_bars: 50,
            offset: 50,
            total_bars: 200, future_bars: 0,
        };
        let zoomed = state.zoom(0.5, 75);
        assert_eq!(zoomed.visible_bars, 100);
    }

    #[test]
    fn zoom_in_respects_minimum() {
        let state = ZoomPanState {
            visible_bars: 6,
            offset: 0,
            total_bars: 100, future_bars: 0,
        };
        let zoomed = state.zoom(100.0, 3);
        assert_eq!(zoomed.visible_bars, MIN_VISIBLE_BARS);
    }

    #[test]
    fn zoom_out_respects_maximum() {
        let state = ZoomPanState {
            visible_bars: 90,
            offset: 0,
            total_bars: 100, future_bars: 0,
        };
        let zoomed = state.zoom(0.01, 50);
        assert_eq!(zoomed.visible_bars, 100);
    }

    #[test]
    fn scroll_to_end() {
        let state = ZoomPanState {
            visible_bars: 20,
            offset: 10,
            total_bars: 100, future_bars: 0,
        };
        let end = state.scroll_to_end();
        assert_eq!(end.offset, 80);
        assert_eq!(end.visible_bars, 20);
    }

    #[test]
    fn zoom_preserves_anchor_position() {
        let state = ZoomPanState {
            visible_bars: 100,
            offset: 0,
            total_bars: 200, future_bars: 0,
        };
        // Anchor is bar 50, which is at 50% of the visible range
        let zoomed = state.zoom(2.0, 50);
        // After zoom: 50 bars visible. Anchor should still be near the middle.
        let range = zoomed.visible_range();
        assert!(range.start <= 50 && range.end > 50);
    }
}