// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use std::fmt::Write;

use ferrochart_core::{Point, Rect};

use crate::style::{Color, FillStyle, LineStyle, TextAnchor, TextStyle};
use crate::Renderer;

/// SVG rendering backend.
///
/// Collects drawing commands as SVG elements and produces a complete SVG document.
pub struct SvgRenderer {
    width: f64,
    height: f64,
    elements: Vec<String>,
    background: Option<Color>,
    clip_id: usize,
}

impl SvgRenderer {
    #[must_use]
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            elements: Vec::new(),
            background: None,
            clip_id: 0,
        }
    }
}

impl Renderer for SvgRenderer {
    fn draw_line(&mut self, from: Point, to: Point, style: &LineStyle) {
        self.elements.push(format!(
            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{:.1}" />"#,
            from.x,
            from.y,
            to.x,
            to.y,
            style.color.to_css(),
            style.width
        ));
    }

    fn draw_rect(&mut self, rect: Rect, fill: &FillStyle) {
        self.elements.push(format!(
            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" />"#,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            fill.color.to_css()
        ));
    }

    fn draw_rect_outline(&mut self, rect: Rect, style: &LineStyle) {
        self.elements.push(format!(
            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="none" stroke="{}" stroke-width="{:.1}" />"#,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            style.color.to_css(),
            style.width
        ));
    }

    fn draw_text(&mut self, text: &str, pos: Point, style: &TextStyle, anchor: TextAnchor) {
        let anchor_str = match anchor {
            TextAnchor::Start => "start",
            TextAnchor::Middle => "middle",
            TextAnchor::End => "end",
        };
        self.elements.push(format!(
            r#"<text x="{:.2}" y="{:.2}" fill="{}" font-size="{:.1}" font-family="{}" text-anchor="{}">{}</text>"#,
            pos.x,
            pos.y,
            style.color.to_css(),
            style.size,
            style.font_family,
            anchor_str,
            text
        ));
    }

    fn draw_path(&mut self, points: &[Point], style: &LineStyle) {
        if points.is_empty() {
            return;
        }
        let mut d = format!("M{:.2},{:.2}", points[0].x, points[0].y);
        for p in &points[1..] {
            let _ = write!(d, " L{:.2},{:.2}", p.x, p.y);
        }
        self.elements.push(format!(
            r#"<path d="{d}" fill="none" stroke="{}" stroke-width="{:.1}" />"#,
            style.color.to_css(),
            style.width
        ));
    }

    fn set_background(&mut self, color: Color) {
        self.background = Some(color);
    }

    fn clip(&mut self, rect: Rect) {
        self.clip_id += 1;
        let id = format!("clip{}", self.clip_id);
        self.elements.push(format!(
            r#"<defs><clipPath id="{id}"><rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" /></clipPath></defs>"#,
            rect.x, rect.y, rect.width, rect.height
        ));
        self.elements.push(format!(r#"<g clip-path="url(#{id})">"#));
    }

    fn restore_clip(&mut self) {
        self.elements.push("</g>".to_string());
    }

    fn finish(&self) -> Vec<u8> {
        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{:.0}" height="{:.0}" viewBox="0 0 {:.0} {:.0}">"#,
            self.width, self.height, self.width, self.height
        );
        svg.push('\n');

        if let Some(bg) = &self.background {
            let _ = write!(
                svg,
                r#"<rect width="100%" height="100%" fill="{}" />"#,
                bg.to_css()
            );
            svg.push('\n');
        }

        for el in &self.elements {
            svg.push_str(el);
            svg.push('\n');
        }

        svg.push_str("</svg>\n");
        svg.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_svg_has_root_element() {
        let r = SvgRenderer::new(800.0, 600.0);
        let out = String::from_utf8(r.finish()).unwrap();
        assert!(out.starts_with("<svg"));
        assert!(out.contains("width=\"800\""));
        assert!(out.contains("height=\"600\""));
        assert!(out.trim_end().ends_with("</svg>"));
    }

    #[test]
    fn background_appears_first() {
        let mut r = SvgRenderer::new(100.0, 100.0);
        r.set_background(Color::WHITE);
        r.draw_line(
            Point { x: 0.0, y: 0.0 },
            Point { x: 100.0, y: 100.0 },
            &LineStyle::default(),
        );
        let out = String::from_utf8(r.finish()).unwrap();
        let bg_pos = out.find("fill=\"rgb(255,255,255)\"").unwrap();
        let line_pos = out.find("<line").unwrap();
        assert!(bg_pos < line_pos);
    }

    #[test]
    fn draw_line_produces_line_element() {
        let mut r = SvgRenderer::new(100.0, 100.0);
        r.draw_line(
            Point { x: 10.0, y: 20.0 },
            Point { x: 90.0, y: 80.0 },
            &LineStyle {
                color: Color::RED,
                width: 2.0,
            },
        );
        let out = String::from_utf8(r.finish()).unwrap();
        assert!(out.contains("<line"));
        assert!(out.contains("x1=\"10.00\""));
        assert!(out.contains("x2=\"90.00\""));
    }

    #[test]
    fn draw_rect_produces_rect_element() {
        let mut r = SvgRenderer::new(100.0, 100.0);
        r.draw_rect(
            Rect::new(10.0, 20.0, 30.0, 40.0),
            &FillStyle {
                color: Color::GREEN,
            },
        );
        let out = String::from_utf8(r.finish()).unwrap();
        assert!(out.contains("<rect"));
        assert!(out.contains("width=\"30.00\""));
        assert!(out.contains("height=\"40.00\""));
    }

    #[test]
    fn draw_rect_outline_has_no_fill() {
        let mut r = SvgRenderer::new(100.0, 100.0);
        r.draw_rect_outline(Rect::new(0.0, 0.0, 50.0, 50.0), &LineStyle::default());
        let out = String::from_utf8(r.finish()).unwrap();
        assert!(out.contains(r#"fill="none""#));
        assert!(out.contains("stroke="));
    }

    #[test]
    fn draw_text_with_anchors() {
        let style = TextStyle::default();
        for (anchor, expected) in [
            (TextAnchor::Start, "start"),
            (TextAnchor::Middle, "middle"),
            (TextAnchor::End, "end"),
        ] {
            let mut r = SvgRenderer::new(100.0, 100.0);
            r.draw_text("test", Point { x: 50.0, y: 50.0 }, &style, anchor);
            let out = String::from_utf8(r.finish()).unwrap();
            assert!(out.contains(&format!("text-anchor=\"{expected}\"")));
        }
    }

    #[test]
    fn draw_path_produces_path_element() {
        let mut r = SvgRenderer::new(100.0, 100.0);
        r.draw_path(
            &[
                Point { x: 0.0, y: 0.0 },
                Point { x: 50.0, y: 25.0 },
                Point { x: 100.0, y: 50.0 },
            ],
            &LineStyle::default(),
        );
        let out = String::from_utf8(r.finish()).unwrap();
        assert!(out.contains("<path"));
        assert!(out.contains("M0.00,0.00"));
        assert!(out.contains("L50.00,25.00"));
    }

    #[test]
    fn draw_path_empty_does_nothing() {
        let mut r = SvgRenderer::new(100.0, 100.0);
        r.draw_path(&[], &LineStyle::default());
        let out = String::from_utf8(r.finish()).unwrap();
        assert!(!out.contains("<path"));
    }
}