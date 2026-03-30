use powerchart_core::{
    CandleGeometry, Ohlcv, PanelLayout, Point, PriceRange, Rect, TimeRange, Transform, Viewport,
};

use crate::style::{Color, FillStyle, LineStyle, TextAnchor, TextStyle};
use crate::Renderer;

/// Configuration for chart rendering.
#[derive(Debug, Clone)]
pub struct ChartConfig {
    pub width: f64,
    pub height: f64,
    pub background: Color,
    pub bullish_color: Color,
    pub bearish_color: Color,
    pub wick_color: Color,
    pub axis_color: Color,
    pub grid_color: Color,
    pub text_color: Color,
    pub body_ratio: f64,
    pub margin: ChartMargin,
    pub font_size: f64,
}

/// Margins around the chart area.
#[derive(Debug, Clone, Copy)]
pub struct ChartMargin {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Default for ChartConfig {
    fn default() -> Self {
        Self {
            width: 900.0,
            height: 500.0,
            background: Color::rgb(22, 26, 37),
            bullish_color: Color::GREEN,
            bearish_color: Color::RED,
            wick_color: Color::LIGHT_GRAY,
            axis_color: Color::GRAY,
            grid_color: Color::rgba(128, 128, 128, 40),
            text_color: Color::LIGHT_GRAY,
            body_ratio: 0.7,
            margin: ChartMargin {
                top: 20.0,
                right: 70.0,
                bottom: 55.0,
                left: 10.0,
            },
            font_size: 11.0,
        }
    }
}

/// Render a candlestick chart into the given renderer.
#[allow(clippy::cast_precision_loss)]
pub fn render_candlestick_chart(
    renderer: &mut dyn Renderer,
    data: &[Ohlcv],
    config: &ChartConfig,
) {
    if data.is_empty() {
        return;
    }

    renderer.set_background(config.background);

    let chart_rect = Rect::new(
        config.margin.left,
        config.margin.top,
        config.width - config.margin.left - config.margin.right,
        config.height - config.margin.top - config.margin.bottom,
    );

    let time_range = TimeRange::new(0, data.len());
    let price_range = PriceRange::from_ohlcv(data)
        .unwrap_or(PriceRange::new(0.0, 100.0))
        .with_padding(0.05);

    // Inset the viewport so bars don't touch the frame edges
    let data_rect = inset_rect_horizontal(&chart_rect, data.len());

    let viewport = Viewport {
        rect: data_rect,
        time_range,
        price_range,
    };
    let transform = Transform::from_viewport(&viewport);

    // Grid lines + Y-axis labels
    draw_y_axis(renderer, &chart_rect, &price_range, &transform, config);

    // X-axis labels
    draw_x_axis(renderer, data, &chart_rect, &transform, config);

    // Candlesticks
    let candles = CandleGeometry::compute_all(data, 0, &transform, config.body_ratio);
    draw_candles(renderer, &candles, config);

    // Chart border
    renderer.draw_rect_outline(
        chart_rect,
        &LineStyle {
            color: config.axis_color,
            width: 1.0,
        },
    );
}

fn draw_candles(renderer: &mut dyn Renderer, candles: &[CandleGeometry], config: &ChartConfig) {
    let wick_style = LineStyle {
        color: config.wick_color,
        width: 1.0,
    };

    for c in candles {
        let body_color = if c.bullish {
            config.bullish_color
        } else {
            config.bearish_color
        };

        // Wick
        renderer.draw_line(
            Point {
                x: c.x,
                y: c.wick_top,
            },
            Point {
                x: c.x,
                y: c.wick_bottom,
            },
            &wick_style,
        );

        // Body
        let body_height = (c.body_bottom - c.body_top).max(1.0);
        renderer.draw_rect(
            Rect::new(c.x - c.body_width / 2.0, c.body_top, c.body_width, body_height),
            &FillStyle { color: body_color },
        );
    }
}

#[allow(clippy::cast_precision_loss)]
fn draw_y_axis(
    renderer: &mut dyn Renderer,
    chart_rect: &Rect,
    price_range: &PriceRange,
    transform: &Transform,
    config: &ChartConfig,
) {
    let text_style = TextStyle {
        color: config.text_color,
        size: config.font_size,
        font_family: "monospace".to_string(),
    };
    let grid_style = LineStyle {
        color: config.grid_color,
        width: 1.0,
    };

    let num_labels: i32 = 8;
    let step = price_range.span() / f64::from(num_labels);

    // Skip first and last to keep spacing from panel edges
    for i in 1..num_labels {
        let price = price_range.min + step * f64::from(i);
        let y = transform.price_y(price);

        // Grid line
        renderer.draw_line(
            Point {
                x: chart_rect.x,
                y,
            },
            Point {
                x: chart_rect.right(),
                y,
            },
            &grid_style,
        );

        // Price label
        renderer.draw_text(
            &format!("{price:.2}"),
            Point {
                x: chart_rect.right() + 5.0,
                y: y + 4.0,
            },
            &text_style,
            TextAnchor::Start,
        );
    }
}

#[allow(clippy::cast_precision_loss)]
fn draw_x_axis(
    renderer: &mut dyn Renderer,
    data: &[Ohlcv],
    chart_rect: &Rect,
    transform: &Transform,
    config: &ChartConfig,
) {
    let text_style = TextStyle {
        color: config.text_color,
        size: config.font_size,
        font_family: "monospace".to_string(),
    };

    let total = data.len();
    let label_count = 8.min(total);
    if label_count == 0 {
        return;
    }
    let step = total / label_count;

    // Day labels
    for i in (0..total).step_by(step.max(1)) {
        let x = transform.bar_x(i);
        let timestamp = data[i].timestamp;
        let label = format_timestamp(timestamp);

        renderer.draw_text(
            &label,
            Point {
                x,
                y: chart_rect.bottom() + 15.0,
            },
            &text_style,
            TextAnchor::Middle,
        );
    }

    // Month/year labels — one per month, centered in that month's bar range
    draw_month_labels(renderer, data, chart_rect, transform, config);
}

/// Draw month/year labels centered below each month's range of bars.
#[allow(clippy::cast_precision_loss)]
fn draw_month_labels(
    renderer: &mut dyn Renderer,
    data: &[Ohlcv],
    chart_rect: &Rect,
    transform: &Transform,
    config: &ChartConfig,
) {
    let text_style = TextStyle {
        color: config.text_color,
        size: config.font_size,
        font_family: "monospace".to_string(),
    };

    // Group bars by (year, month) and find the center x of each group
    let mut month_spans: Vec<(i64, u32, usize, usize)> = Vec::new(); // (year, month, first_idx, last_idx)

    for (i, bar) in data.iter().enumerate() {
        let (year, month, _day) = days_to_ymd(bar.timestamp / 86_400);
        if let Some(last) = month_spans.last_mut()
            && last.0 == year && last.1 == month
        {
            last.3 = i;
            continue;
        }
        month_spans.push((year, month, i, i));
    }

    for &(year, month, first, last) in &month_spans {
        let x_first = transform.bar_x(first);
        let x_last = transform.bar_x(last);
        let x_center = f64::midpoint(x_first, x_last);

        let month_name = month_abbrev(month);
        let label = format!("{month_name} {year}");

        renderer.draw_text(
            &label,
            Point {
                x: x_center,
                y: chart_rect.bottom() + 30.0,
            },
            &text_style,
            TextAnchor::Middle,
        );
    }
}

const fn month_abbrev(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

/// Draw volume Y-axis labels on the right side.
#[allow(clippy::cast_precision_loss)]
fn draw_volume_axis(
    renderer: &mut dyn Renderer,
    panel_rect: &Rect,
    vol_range: &PriceRange,
    transform: &Transform,
    config: &ChartConfig,
) {
    let text_style = TextStyle {
        color: config.text_color,
        size: config.font_size,
        font_family: "monospace".to_string(),
    };

    let num_labels: i32 = 4;
    let step = vol_range.span() / f64::from(num_labels);

    // Skip first and last to keep spacing from panel edges
    for i in 1..num_labels {
        let vol = vol_range.min + step * f64::from(i);
        let y = transform.price_y(vol);

        // Format volume with K/M suffix
        let label = format_volume(vol);

        renderer.draw_text(
            &label,
            Point {
                x: panel_rect.right() + 5.0,
                y: y + 4.0,
            },
            &text_style,
            TextAnchor::Start,
        );
    }
}

/// Format volume with K/M suffix for readability.
fn format_volume(vol: f64) -> String {
    if vol >= 1_000_000.0 {
        format!("{:.1}M", vol / 1_000_000.0)
    } else if vol >= 1_000.0 {
        format!("{:.1}K", vol / 1_000.0)
    } else {
        format!("{vol:.0}")
    }
}

/// Inset a rect horizontally by half a bar width so bars don't clip the frame.
#[allow(clippy::cast_precision_loss)]
fn inset_rect_horizontal(rect: &Rect, num_bars: usize) -> Rect {
    // Estimate bar width, then inset by half of it
    let bar_width = if num_bars > 1 {
        rect.width / (num_bars - 1) as f64
    } else {
        rect.width
    };
    let inset = bar_width * 0.5;
    Rect::new(rect.x + inset, rect.y, rect.width - 2.0 * inset, rect.height)
}

/// Format a unix timestamp as just the day number.
fn format_timestamp(ts: i64) -> String {
    let days = ts / 86_400;
    let (_year, _month, day) = days_to_ymd(days);
    format!("{day}")
}

/// Convert days since epoch to (year, month, day).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = i64::from(yoe) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Render a multi-panel chart (e.g. price + volume).
///
/// # Panics
///
/// Panics if the internal panel layout cannot be constructed (should not happen).
#[allow(clippy::cast_precision_loss)]
pub fn render_with_volume(
    renderer: &mut dyn Renderer,
    data: &[Ohlcv],
    config: &ChartConfig,
) {
    if data.is_empty() {
        return;
    }

    renderer.set_background(config.background);

    let total_rect = Rect::new(
        config.margin.left,
        config.margin.top,
        config.width - config.margin.left - config.margin.right,
        config.height - config.margin.top - config.margin.bottom,
    );

    let layout = PanelLayout::new(&[70.0, 30.0], total_rect, 4.0);
    let price_panel = layout.get(0).unwrap();
    let volume_panel = layout.get(1).unwrap();

    let time_range = TimeRange::new(0, data.len());
    let price_data_rect = inset_rect_horizontal(&price_panel.rect, data.len());
    let vol_data_rect = inset_rect_horizontal(&volume_panel.rect, data.len());

    // --- Price panel ---
    let price_range = PriceRange::from_ohlcv(data)
        .unwrap_or(PriceRange::new(0.0, 100.0))
        .with_padding(0.05);
    let price_vp = Viewport {
        rect: price_data_rect,
        time_range,
        price_range,
    };
    let price_transform = Transform::from_viewport(&price_vp);

    draw_y_axis(renderer, &price_panel.rect, &price_range, &price_transform, config);
    let candles = CandleGeometry::compute_all(data, 0, &price_transform, config.body_ratio);
    draw_candles(renderer, &candles, config);
    renderer.draw_rect_outline(
        price_panel.rect,
        &LineStyle {
            color: config.axis_color,
            width: 1.0,
        },
    );

    // --- Volume panel ---
    let max_vol = data
        .iter()
        .map(|b| b.volume)
        .fold(0.0_f64, f64::max);
    let vol_range = PriceRange::new(0.0, max_vol * 1.1);
    let vol_vp = Viewport {
        rect: vol_data_rect,
        time_range,
        price_range: vol_range,
    };
    let vol_transform = Transform::from_viewport(&vol_vp);

    // Volume Y-axis labels
    draw_volume_axis(renderer, &volume_panel.rect, &vol_range, &vol_transform, config);

    for (i, bar) in data.iter().enumerate() {
        let x = vol_transform.bar_x(i);
        let bar_w = vol_transform.bar_width() * config.body_ratio;
        let top_y = vol_transform.price_y(bar.volume);
        let bottom_y = vol_transform.price_y(0.0);
        let height = (bottom_y - top_y).max(0.0);
        let color = if bar.close >= bar.open {
            config.bullish_color
        } else {
            config.bearish_color
        };
        renderer.draw_rect(
            Rect::new(x - bar_w / 2.0, top_y, bar_w, height),
            &FillStyle { color },
        );
    }
    renderer.draw_rect_outline(
        volume_panel.rect,
        &LineStyle {
            color: config.axis_color,
            width: 1.0,
        },
    );

    // X-axis labels below volume panel
    draw_x_axis(renderer, data, &volume_panel.rect, &price_transform, config);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_data() -> Vec<Ohlcv> {
        vec![
            Ohlcv { timestamp: 1_700_000_000, open: 100.0, high: 110.0, low: 95.0, close: 108.0, volume: 5000.0 },
            Ohlcv { timestamp: 1_700_086_400, open: 108.0, high: 115.0, low: 105.0, close: 112.0, volume: 6000.0 },
            Ohlcv { timestamp: 1_700_172_800, open: 112.0, high: 118.0, low: 100.0, close: 102.0, volume: 8000.0 },
            Ohlcv { timestamp: 1_700_259_200, open: 102.0, high: 108.0, low: 98.0, close: 106.0, volume: 4000.0 },
            Ohlcv { timestamp: 1_700_345_600, open: 106.0, high: 120.0, low: 104.0, close: 118.0, volume: 7000.0 },
        ]
    }

    #[test]
    fn render_produces_valid_svg() {
        let mut r = crate::SvgRenderer::new(900.0, 500.0);
        render_candlestick_chart(&mut r, &sample_data(), &ChartConfig::default());
        let out = String::from_utf8(r.finish()).unwrap();
        assert!(out.starts_with("<svg"));
        assert!(out.contains("<rect")); // candle bodies
        assert!(out.contains("<line")); // wicks
        assert!(out.contains("<text")); // axis labels
    }

    #[test]
    fn render_empty_data_does_not_panic() {
        let mut r = crate::SvgRenderer::new(900.0, 500.0);
        render_candlestick_chart(&mut r, &[], &ChartConfig::default());
        let out = String::from_utf8(r.finish()).unwrap();
        assert!(out.contains("<svg"));
    }

    #[test]
    fn render_with_volume_produces_two_panels() {
        let mut r = crate::SvgRenderer::new(900.0, 500.0);
        render_with_volume(&mut r, &sample_data(), &ChartConfig::default());
        let out = String::from_utf8(r.finish()).unwrap();
        // Should have multiple rect elements (candles + volume bars + panel borders)
        let rect_count = out.matches("<rect").count();
        assert!(rect_count > 10, "expected many rects, got {rect_count}");
    }

    #[test]
    fn days_to_ymd_epoch() {
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn days_to_ymd_known_date() {
        // 2023-11-15 = 19676 days since epoch
        assert_eq!(days_to_ymd(19676), (2023, 11, 15));
    }

    #[test]
    fn format_timestamp_produces_day_only() {
        let label = format_timestamp(1_700_000_000);
        // 2023-11-14 → just "14"
        assert_eq!(label, "14");
    }

    #[test]
    fn format_volume_with_suffix() {
        assert_eq!(format_volume(500.0), "500");
        assert_eq!(format_volume(5_000.0), "5.0K");
        assert_eq!(format_volume(1_500_000.0), "1.5M");
    }

    #[test]
    fn inset_rect_shrinks_width() {
        let r = Rect::new(0.0, 0.0, 900.0, 500.0);
        let inset = inset_rect_horizontal(&r, 10);
        assert!(inset.x > r.x);
        assert!(inset.width < r.width);
        // Symmetric: right edge moves in equally
        assert!((inset.right() - (r.right() - (inset.x - r.x))).abs() < 1e-9);
    }
}
