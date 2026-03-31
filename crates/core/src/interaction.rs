// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use crate::Point;
use crate::ZoomPanState;

/// Result of processing a drag movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragUpdate {
    pub new_offset: usize,
}

/// Compute the new zoom/pan state after a scroll wheel event.
///
/// `mouse_x` is the horizontal pixel position within the chart area.
/// `chart_left` / `chart_width` define the chart's data area in pixels.
/// `delta_y` is the wheel delta (positive = scroll down = zoom out).
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn compute_zoom(
    state: ZoomPanState,
    mouse_x: f64,
    chart_left: f64,
    chart_width: f64,
    delta_y: f64,
) -> ZoomPanState {
    if state.total_bars == 0 || chart_width <= 0.0 {
        return state;
    }

    let frac = ((mouse_x - chart_left) / chart_width).clamp(0.0, 1.0);
    let anchor = state.offset + (frac * state.visible_bars as f64) as usize;
    let factor = if delta_y > 0.0 { 0.8 } else { 1.25 };
    state.zoom(factor, anchor)
}

/// Compute the new zoom/pan state after a drag movement.
///
/// `dx` is the pixel distance dragged since the drag started.
/// `chart_width` is the pixel width of the chart data area.
/// `drag_start_offset` is the `zoom_pan.offset` when the drag began.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn compute_pan(
    state: ZoomPanState,
    dx: f64,
    chart_width: f64,
    drag_start_offset: usize,
) -> ZoomPanState {
    if state.total_bars == 0 || chart_width <= 0.0 {
        return state;
    }

    let bar_width = chart_width / state.visible_bars.max(1) as f64;
    let bar_delta = (dx / bar_width).round() as isize;
    if bar_delta == 0 {
        return state;
    }

    ZoomPanState {
        offset: drag_start_offset,
        ..state
    }
    .pan(-bar_delta)
}

/// Check if a point is within the chart data area.
#[must_use]
pub fn is_in_chart_area(point: Point, left: f64, right: f64, top: f64, bottom: f64) -> bool {
    point.x >= left && point.x <= right && point.y >= top && point.y <= bottom
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_state() -> ZoomPanState {
        ZoomPanState {
            visible_bars: 50,
            offset: 25,
            total_bars: 100,
            future_bars: 0,
        }
    }

    // --- compute_zoom ---

    #[test]
    fn zoom_in_reduces_visible_bars() {
        let state = base_state();
        // delta_y < 0 means scroll up = zoom in
        let zoomed = compute_zoom(state, 400.0, 0.0, 800.0, -1.0);
        assert!(zoomed.visible_bars < state.visible_bars);
    }

    #[test]
    fn zoom_out_increases_visible_bars() {
        let state = base_state();
        // delta_y > 0 means scroll down = zoom out
        let zoomed = compute_zoom(state, 400.0, 0.0, 800.0, 1.0);
        assert!(zoomed.visible_bars > state.visible_bars);
    }

    #[test]
    fn zoom_at_left_edge() {
        let state = base_state();
        let zoomed = compute_zoom(state, 0.0, 0.0, 800.0, -1.0);
        // Anchor at left: offset should stay near 25
        assert!(zoomed.visible_bars < state.visible_bars);
        assert!(zoomed.offset >= state.offset.saturating_sub(5));
    }

    #[test]
    fn zoom_at_right_edge() {
        let state = base_state();
        let zoomed = compute_zoom(state, 800.0, 0.0, 800.0, -1.0);
        // Anchor at right: offset should increase
        assert!(zoomed.visible_bars < state.visible_bars);
    }

    #[test]
    fn zoom_with_zero_total_bars_is_noop() {
        let state = ZoomPanState {
            visible_bars: 5,
            offset: 0,
            total_bars: 0,
            future_bars: 0,
        };
        let zoomed = compute_zoom(state, 400.0, 0.0, 800.0, -1.0);
        assert_eq!(zoomed, state);
    }

    #[test]
    fn zoom_with_zero_chart_width_is_noop() {
        let state = base_state();
        let zoomed = compute_zoom(state, 0.0, 0.0, 0.0, -1.0);
        assert_eq!(zoomed, state);
    }

    #[test]
    fn zoom_in_clamps_at_minimum() {
        let state = ZoomPanState {
            visible_bars: 5,
            offset: 0,
            total_bars: 100,
            future_bars: 0,
        };
        // Zoom in aggressively — should not go below 5
        let zoomed = compute_zoom(state, 400.0, 0.0, 800.0, -1.0);
        assert!(zoomed.visible_bars >= 5);
    }

    #[test]
    fn zoom_out_clamps_at_total() {
        let state = ZoomPanState {
            visible_bars: 95,
            offset: 0,
            total_bars: 100,
            future_bars: 0,
        };
        // Keep zooming out
        let mut s = state;
        for _ in 0..20 {
            s = compute_zoom(s, 400.0, 0.0, 800.0, 1.0);
        }
        assert_eq!(s.visible_bars, 100);
        assert_eq!(s.offset, 0);
    }

    // --- compute_pan ---

    #[test]
    fn pan_right_increases_offset() {
        let state = base_state();
        let panned = compute_pan(state, -50.0, 800.0, state.offset);
        // Dragging left = panning right = later bars
        assert!(panned.offset > state.offset);
    }

    #[test]
    fn pan_left_decreases_offset() {
        let state = base_state();
        let panned = compute_pan(state, 50.0, 800.0, state.offset);
        // Dragging right = panning left = earlier bars
        assert!(panned.offset < state.offset);
    }

    #[test]
    fn pan_clamps_at_zero() {
        let state = ZoomPanState {
            visible_bars: 50,
            offset: 2,
            total_bars: 100,
            future_bars: 0,
        };
        let panned = compute_pan(state, 500.0, 800.0, state.offset);
        assert_eq!(panned.offset, 0);
    }

    #[test]
    fn pan_clamps_at_end() {
        let state = ZoomPanState {
            visible_bars: 50,
            offset: 48,
            total_bars: 100,
            future_bars: 0,
        };
        let panned = compute_pan(state, -500.0, 800.0, state.offset);
        assert_eq!(panned.offset, 50); // max = 100 - 50
    }

    #[test]
    fn pan_tiny_movement_is_noop() {
        let state = base_state();
        let panned = compute_pan(state, 1.0, 800.0, state.offset);
        // 1px drag with bar_width ~16px rounds to 0 bars
        assert_eq!(panned.offset, state.offset);
    }

    #[test]
    fn pan_with_zero_total_is_noop() {
        let state = ZoomPanState {
            visible_bars: 5,
            offset: 0,
            total_bars: 0,
            future_bars: 0,
        };
        let panned = compute_pan(state, 100.0, 800.0, 0);
        assert_eq!(panned, state);
    }

    #[test]
    fn pan_uses_drag_start_offset() {
        let state = ZoomPanState {
            visible_bars: 50,
            offset: 30,
            total_bars: 100,
            future_bars: 0,
        };
        // Simulate drag that started at offset 10
        let panned = compute_pan(state, -80.0, 800.0, 10);
        // Should pan from offset 10, not from 30
        assert!(panned.offset > 10);
    }

    // --- is_in_chart_area ---

    #[test]
    fn point_inside_chart() {
        assert!(is_in_chart_area(
            Point { x: 50.0, y: 50.0 },
            10.0,
            100.0,
            10.0,
            100.0
        ));
    }

    #[test]
    fn point_on_edge() {
        assert!(is_in_chart_area(
            Point { x: 10.0, y: 10.0 },
            10.0,
            100.0,
            10.0,
            100.0
        ));
        assert!(is_in_chart_area(
            Point { x: 100.0, y: 100.0 },
            10.0,
            100.0,
            10.0,
            100.0
        ));
    }

    #[test]
    fn point_outside_chart() {
        assert!(!is_in_chart_area(
            Point { x: 5.0, y: 50.0 },
            10.0,
            100.0,
            10.0,
            100.0
        ));
        assert!(!is_in_chart_area(
            Point { x: 50.0, y: 5.0 },
            10.0,
            100.0,
            10.0,
            100.0
        ));
        assert!(!is_in_chart_area(
            Point { x: 105.0, y: 50.0 },
            10.0,
            100.0,
            10.0,
            100.0
        ));
    }
}
