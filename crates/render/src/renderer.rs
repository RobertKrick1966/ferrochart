// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use ferrochart_core::{Point, Rect};

use crate::style::{Color, FillStyle, LineStyle, TextAnchor, TextStyle};

/// Abstract rendering interface.
///
/// Backends (SVG, Canvas, etc.) implement this trait.
/// All coordinates are in pixel space.
pub trait Renderer {
    /// Draw a line between two points.
    fn draw_line(&mut self, from: Point, to: Point, style: &LineStyle);

    /// Draw a filled rectangle.
    fn draw_rect(&mut self, rect: Rect, fill: &FillStyle);

    /// Draw a rectangle outline (stroke only).
    fn draw_rect_outline(&mut self, rect: Rect, style: &LineStyle);

    /// Draw text at a position.
    fn draw_text(&mut self, text: &str, pos: Point, style: &TextStyle, anchor: TextAnchor);

    /// Draw a polyline (connected line segments).
    fn draw_path(&mut self, points: &[Point], style: &LineStyle);

    /// Set the background color.
    fn set_background(&mut self, color: Color);

    /// Save state and clip to the given rectangle.
    /// All subsequent drawing is clipped until `restore_clip()` is called.
    fn clip(&mut self, rect: Rect);

    /// Restore state before the last `clip()` call.
    fn restore_clip(&mut self);

    /// Finalize and return the rendered output as bytes.
    fn finish(&self) -> Vec<u8>;
}