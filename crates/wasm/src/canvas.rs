// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use ferrochart_core::{Point, Rect};
use ferrochart_render::style::{Color, FillStyle, LineStyle, TextAnchor, TextStyle};
use ferrochart_render::Renderer;

/// Canvas 2D rendering backend for browsers.
///
/// Drawing commands are executed immediately on the canvas context.
/// The [`Renderer::finish`] method returns an empty `Vec` since all
/// rendering has already been applied to the canvas.
pub struct CanvasRenderer {
    ctx: CanvasRenderingContext2d,
    width: f64,
    height: f64,
}

impl CanvasRenderer {
    /// Create a new `CanvasRenderer` from an HTML canvas element.
    ///
    /// # Errors
    ///
    /// Returns a `JsValue` error if the 2D rendering context cannot be obtained.
    pub fn new(canvas: &HtmlCanvasElement) -> Result<Self, JsValue> {
        let ctx = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("failed to get 2d context"))?
            .dyn_into::<CanvasRenderingContext2d>()?;
        let width = f64::from(canvas.width());
        let height = f64::from(canvas.height());
        Ok(Self { ctx, width, height })
    }

    fn set_stroke(&self, color: Color, width: f64) {
        self.ctx.set_stroke_style_str(&color.to_css());
        self.ctx.set_line_width(width);
    }

    fn set_fill(&self, color: Color) {
        self.ctx.set_fill_style_str(&color.to_css());
    }
}

impl Renderer for CanvasRenderer {
    fn draw_line(&mut self, from: Point, to: Point, style: &LineStyle) {
        self.set_stroke(style.color, style.width);
        self.ctx.begin_path();
        self.ctx.move_to(from.x, from.y);
        self.ctx.line_to(to.x, to.y);
        self.ctx.stroke();
    }

    fn draw_rect(&mut self, rect: Rect, fill: &FillStyle) {
        self.set_fill(fill.color);
        self.ctx
            .fill_rect(rect.x, rect.y, rect.width, rect.height);
    }

    fn draw_rect_outline(&mut self, rect: Rect, style: &LineStyle) {
        self.set_stroke(style.color, style.width);
        self.ctx
            .stroke_rect(rect.x, rect.y, rect.width, rect.height);
    }

    fn draw_text(&mut self, text: &str, pos: Point, style: &TextStyle, anchor: TextAnchor) {
        self.set_fill(style.color);
        self.ctx
            .set_font(&format!("{:.0}px {}", style.size, style.font_family));
        let align = match anchor {
            TextAnchor::Start => "left",
            TextAnchor::Middle => "center",
            TextAnchor::End => "right",
        };
        self.ctx.set_text_align(align);
        // Ignore the error from fill_text (only fails if text measurement is unsupported)
        let _ = self.ctx.fill_text(text, pos.x, pos.y);
    }

    fn draw_path(&mut self, points: &[Point], style: &LineStyle) {
        if points.is_empty() {
            return;
        }
        self.set_stroke(style.color, style.width);
        self.ctx.begin_path();
        self.ctx.move_to(points[0].x, points[0].y);
        for p in &points[1..] {
            self.ctx.line_to(p.x, p.y);
        }
        self.ctx.stroke();
    }

    fn set_background(&mut self, color: Color) {
        self.set_fill(color);
        self.ctx.fill_rect(0.0, 0.0, self.width, self.height);
    }

    fn clip(&mut self, rect: Rect) {
        self.ctx.save();
        self.ctx.begin_path();
        self.ctx.rect(rect.x, rect.y, rect.width, rect.height);
        self.ctx.clip();
    }

    fn restore_clip(&mut self) {
        self.ctx.restore();
    }

    fn finish(&self) -> Vec<u8> {
        // Canvas rendering is immediate-mode — all drawing already happened.
        Vec::new()
    }
}