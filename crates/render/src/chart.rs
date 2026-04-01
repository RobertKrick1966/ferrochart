// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use ferrochart_core::{
    Annotations, CandleGeometry, IndicatorOutput, IndicatorPlacement, Marker, MarkerPosition,
    MarkerShape, Ohlcv, PanelLayout, Point, PriceRange, Rect, SeriesStyle, TimeRange, Transform,
    Viewport,
};

use crate::Renderer;
use crate::style::{Color, FillStyle, LineStyle, TextAnchor, TextStyle};

/// Configuration for chart rendering.
#[derive(Debug, Clone)]
pub struct ChartConfig {
    pub width: f64,
    pub height: f64,
    pub background: Color,
    pub bullish_color: Color,
    pub bearish_color: Color,
    /// Color for the institutional portion of a split candle body.
    pub institutional_color: Color,
    pub wick_color: Color,
    pub axis_color: Color,
    pub grid_color: Color,
    pub text_color: Color,
    pub body_ratio: f64,
    pub margin: ChartMargin,
    pub font_size: f64,
    pub indicator_colors: Vec<Color>,
    /// Y-axis scale factor (1.0 = auto-fit, <1.0 = zoom in, >1.0 = zoom out).
    pub price_scale: f64,
    /// Custom panel weights. If `None`, defaults are computed dynamically.
    pub panel_weights: Option<Vec<f64>>,
    /// If set, use this many bar slots for X-axis spacing instead of `data.len()`.
    /// Enables future space: data occupies the left portion, right is empty.
    pub visible_bar_slots: Option<usize>,
    /// Offset of the visible data in the full dataset (for annotation coordinate mapping).
    pub visible_offset: usize,
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
            institutional_color: Color::rgb(0, 120, 255), // blue
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
            price_scale: 1.0,
            panel_weights: None,
            visible_bar_slots: None,
            visible_offset: 0,
            indicator_colors: vec![
                Color::rgb(255, 235, 59), // yellow
                Color::rgb(0, 188, 212),  // cyan
                Color::rgb(233, 30, 99),  // pink
                Color::rgb(255, 152, 0),  // orange
                Color::rgb(103, 58, 183), // purple
                Color::rgb(76, 175, 80),  // light green
                Color::rgb(33, 150, 243), // blue
                Color::rgb(255, 87, 34),  // deep orange
            ],
        }
    }
}

impl ChartConfig {
    /// Dark theme (default).
    #[must_use]
    pub fn dark() -> Self {
        Self::default()
    }

    /// Light theme with white background.
    #[must_use]
    pub fn light() -> Self {
        Self {
            background: Color::rgb(255, 255, 255),
            bullish_color: Color::rgb(38, 166, 91),
            bearish_color: Color::rgb(214, 48, 49),
            wick_color: Color::rgb(80, 80, 80),
            axis_color: Color::rgb(180, 180, 180),
            grid_color: Color::rgba(0, 0, 0, 20),
            text_color: Color::rgb(60, 60, 60),
            indicator_colors: vec![
                Color::rgb(41, 128, 185), // blue
                Color::rgb(231, 76, 60),  // red
                Color::rgb(39, 174, 96),  // green
                Color::rgb(243, 156, 18), // orange
                Color::rgb(142, 68, 173), // purple
                Color::rgb(22, 160, 133), // teal
                Color::rgb(211, 84, 0),   // dark orange
                Color::rgb(44, 62, 80),   // dark blue
            ],
            ..Self::default()
        }
    }
}

/// Render a candlestick chart into the given renderer.
#[allow(clippy::cast_precision_loss)]
pub fn render_candlestick_chart(renderer: &mut dyn Renderer, data: &[Ohlcv], config: &ChartConfig) {
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

        // Body (split when institutional_ratio > 0)
        let body_height = (c.body_bottom - c.body_top).max(1.0);
        let body_x = c.x - c.body_width / 2.0;

        if c.institutional_ratio > 0.0 {
            // Bottom portion: institutional color
            let inst_height = body_height * c.institutional_ratio;
            let retail_height = body_height - inst_height;

            // Top part: regular bull/bear color
            renderer.draw_rect(
                Rect::new(body_x, c.body_top, c.body_width, retail_height),
                &FillStyle { color: body_color },
            );
            // Bottom part: institutional color
            renderer.draw_rect(
                Rect::new(
                    body_x,
                    c.body_top + retail_height,
                    c.body_width,
                    inst_height,
                ),
                &FillStyle {
                    color: config.institutional_color,
                },
            );
        } else {
            renderer.draw_rect(
                Rect::new(body_x, c.body_top, c.body_width, body_height),
                &FillStyle { color: body_color },
            );
        }
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
            Point { x: chart_rect.x, y },
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

    let interval = detect_interval(data);

    // Time labels
    for i in (0..total).step_by(step.max(1)) {
        let x = transform.bar_x(i);
        let timestamp = data[i].timestamp;
        let label = format_timestamp(timestamp, interval);

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

    // Second-row labels: month/year for daily, date for intraday
    if interval < 86_400 {
        draw_date_labels(renderer, data, chart_rect, transform, config);
    } else {
        draw_month_labels(renderer, data, chart_rect, transform, config);
    }
}

/// Draw date labels for intraday data (one per day, centered).
#[allow(clippy::cast_precision_loss)]
fn draw_date_labels(
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

    let mut day_spans: Vec<(i64, u32, u32, usize, usize)> = Vec::new();

    for (i, bar) in data.iter().enumerate() {
        let days = bar.timestamp / 86_400;
        let (year, month, day) = days_to_ymd(days);
        if let Some(last) = day_spans.last_mut()
            && last.0 == year
            && last.1 == month
            && last.2 == day
        {
            last.4 = i;
            continue;
        }
        day_spans.push((year, month, day, i, i));
    }

    for &(year, month, day, first, last) in &day_spans {
        let x_center = f64::midpoint(transform.bar_x(first), transform.bar_x(last));
        let label = format!("{day:02} {m} {year}", m = month_abbrev(month));

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
            && last.0 == year
            && last.1 == month
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

/// Draw volume Y-axis labels and grid lines on the right side.
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
    let grid_style = LineStyle {
        color: config.grid_color,
        width: 1.0,
    };

    let num_labels: i32 = 4;
    let step = vol_range.span() / f64::from(num_labels);

    // Skip first and last to keep spacing from panel edges
    for i in 1..num_labels {
        let vol = vol_range.min + step * f64::from(i);
        let y = transform.price_y(vol);

        renderer.draw_line(
            Point { x: panel_rect.x, y },
            Point {
                x: panel_rect.right(),
                y,
            },
            &grid_style,
        );

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
    Rect::new(
        rect.x + inset,
        rect.y,
        rect.width - 2.0 * inset,
        rect.height,
    )
}

/// Detect the average interval between bars in seconds.
#[allow(clippy::cast_possible_wrap)]
fn detect_interval(data: &[Ohlcv]) -> i64 {
    if data.len() < 2 {
        return 86_400;
    }
    let total = data.last().unwrap().timestamp - data.first().unwrap().timestamp;
    total / (data.len() as i64 - 1).max(1)
}

/// Format a unix timestamp based on the data interval.
fn format_timestamp(ts: i64, interval: i64) -> String {
    if interval < 3600 {
        // Sub-hourly: show HH:MM
        let h = (ts % 86_400) / 3600;
        let m = (ts % 3600) / 60;
        format!("{h:02}:{m:02}")
    } else if interval < 86_400 {
        // Hourly: show HH:00
        let h = (ts % 86_400) / 3600;
        format!("{h:02}:00")
    } else {
        // Daily+: show day number
        let days = ts / 86_400;
        let (_year, _month, day) = days_to_ymd(days);
        format!("{day}")
    }
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
pub fn render_with_volume(renderer: &mut dyn Renderer, data: &[Ohlcv], config: &ChartConfig) {
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

    draw_y_axis(
        renderer,
        &price_panel.rect,
        &price_range,
        &price_transform,
        config,
    );
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
    let max_vol = data.iter().map(|b| b.volume).fold(0.0_f64, f64::max);
    let vol_range = PriceRange::new(0.0, max_vol * 1.1);
    let vol_vp = Viewport {
        rect: vol_data_rect,
        time_range,
        price_range: vol_range,
    };
    let vol_transform = Transform::from_viewport(&vol_vp);

    // Volume Y-axis labels
    draw_volume_axis(
        renderer,
        &volume_panel.rect,
        &vol_range,
        &vol_transform,
        config,
    );

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

fn default_panel_weights(num_sub_panels: usize) -> Vec<f64> {
    let mut weights = vec![55.0, 20.0];
    weights.extend(std::iter::repeat_n(15.0, num_sub_panels));
    weights
}

/// Describes what a rendered panel contains.
#[derive(Debug, Clone)]
pub enum PanelKind {
    Price,
    Volume,
    Indicator(String),
}

/// Info about a rendered panel (for hit-testing/tooltip).
#[derive(Debug, Clone)]
pub struct PanelInfo {
    pub rect: Rect,
    pub kind: PanelKind,
}

/// Result of rendering, containing panel layout info and transforms.
#[derive(Debug, Clone, Default)]
pub struct ChartLayoutInfo {
    pub panels: Vec<PanelInfo>,
    /// The price transform used for the main chart panel (for coordinate mapping).
    pub price_transform: Option<Transform>,
    /// Number of bar slots used for X-axis spacing.
    pub bar_slots: usize,
}

/// Render a full chart with candlesticks, volume, and indicators.
///
/// # Panics
///
/// Panics if the internal panel layout cannot be constructed.
#[allow(clippy::cast_precision_loss)]
pub fn render_full_chart(
    renderer: &mut dyn Renderer,
    data: &[Ohlcv],
    indicators: &[IndicatorOutput],
    config: &ChartConfig,
) -> ChartLayoutInfo {
    render_full_chart_with_markers(
        renderer,
        data,
        indicators,
        &[],
        &Annotations::default(),
        config,
    )
}

/// Render a full chart with candlesticks, volume, indicators, markers, and annotations.
///
/// # Panics
///
/// Panics if the internal panel layout cannot be constructed.
#[allow(clippy::cast_precision_loss)]
pub fn render_full_chart_with_markers(
    renderer: &mut dyn Renderer,
    data: &[Ohlcv],
    indicators: &[IndicatorOutput],
    markers: &[&Marker],
    annotations: &Annotations,
    config: &ChartConfig,
) -> ChartLayoutInfo {
    if data.is_empty() {
        return ChartLayoutInfo::default();
    }
    let mut layout_info = ChartLayoutInfo::default();

    renderer.set_background(config.background);

    let total_rect = Rect::new(
        config.margin.left,
        config.margin.top,
        config.width - config.margin.left - config.margin.right,
        config.height - config.margin.top - config.margin.bottom,
    );

    // Partition indicators
    let overlays: Vec<&IndicatorOutput> = indicators
        .iter()
        .filter(|ind| ind.placement == IndicatorPlacement::Overlay)
        .collect();
    let sub_panels: Vec<&IndicatorOutput> = indicators
        .iter()
        .filter(|ind| ind.placement != IndicatorPlacement::Overlay)
        .collect();

    // Panel weights: use custom if provided, otherwise default
    let expected_panels = 2 + sub_panels.len();
    let weights = if let Some(ref w) = config.panel_weights {
        if w.len() == expected_panels {
            w.clone()
        } else {
            default_panel_weights(sub_panels.len())
        }
    } else {
        default_panel_weights(sub_panels.len())
    };
    let layout = PanelLayout::new(&weights, total_rect, 4.0);
    let price_panel = layout.get(0).unwrap();
    let volume_panel = layout.get(1).unwrap();

    // Use visible_bar_slots for X spacing (enables future space)
    let bar_slots = config.visible_bar_slots.unwrap_or(data.len());
    let time_range = TimeRange::new(0, bar_slots);
    let price_data_rect = inset_rect_horizontal(&price_panel.rect, bar_slots);

    // --- Price panel ---
    // Extend price range to include overlay indicator values (BB bands, etc.)
    let mut price_range = PriceRange::from_ohlcv(data).unwrap_or(PriceRange::new(0.0, 100.0));
    for overlay in &overlays {
        for series in &overlay.series {
            if series.style_hint == SeriesStyle::Line {
                for &v in &series.values {
                    if !v.is_nan() {
                        if v < price_range.min {
                            price_range.min = v;
                        }
                        if v > price_range.max {
                            price_range.max = v;
                        }
                    }
                }
            }
        }
    }
    let price_range = price_range.with_padding(0.03);
    // Apply manual Y-axis scaling
    let price_range = if (config.price_scale - 1.0).abs() > f64::EPSILON {
        let mid = f64::midpoint(price_range.min, price_range.max);
        let half_span = price_range.span() / 2.0 * config.price_scale;
        PriceRange::new(mid - half_span, mid + half_span)
    } else {
        price_range
    };
    let price_vp = Viewport {
        rect: price_data_rect,
        time_range,
        price_range,
    };
    let price_transform = Transform::from_viewport(&price_vp);

    // Clip to price panel + right margin for Y-axis labels
    let clip_rect = Rect::new(
        price_panel.rect.x,
        price_panel.rect.y,
        price_panel.rect.width + config.margin.right,
        price_panel.rect.height,
    );
    renderer.clip(clip_rect);

    draw_y_axis(
        renderer,
        &price_panel.rect,
        &price_range,
        &price_transform,
        config,
    );
    let candles = CandleGeometry::compute_all(data, 0, &price_transform, config.body_ratio);
    draw_candles(renderer, &candles, config);

    // Draw overlay indicators on price panel
    let mut color_idx = 0;
    for overlay in &overlays {
        draw_indicator_overlay(renderer, overlay, &price_transform, config, &mut color_idx);
    }

    // Draw markers on price panel
    draw_markers(renderer, markers, data, &price_transform, config);

    // Draw annotations (trendlines, fibonacci) on price panel
    draw_annotations(
        renderer,
        annotations,
        &price_transform,
        &price_panel.rect,
        bar_slots,
        config,
    );

    // Price panel legend
    draw_panel_legend(renderer, price_panel.rect, &overlays, config);

    renderer.restore_clip();
    renderer.draw_rect_outline(
        price_panel.rect,
        &LineStyle {
            color: config.axis_color,
            width: 1.0,
        },
    );
    layout_info.panels.push(PanelInfo {
        rect: price_panel.rect,
        kind: PanelKind::Price,
    });

    // --- Volume panel ---
    let vol_clip = Rect::new(
        volume_panel.rect.x,
        volume_panel.rect.y,
        volume_panel.rect.width + config.margin.right,
        volume_panel.rect.height,
    );
    renderer.clip(vol_clip);
    let vol_data_rect = inset_rect_horizontal(&volume_panel.rect, bar_slots);
    let max_vol = data.iter().map(|b| b.volume).fold(0.0_f64, f64::max);
    let vol_range = PriceRange::new(0.0, max_vol * 1.1);
    let vol_vp = Viewport {
        rect: vol_data_rect,
        time_range,
        price_range: vol_range,
    };
    let vol_transform = Transform::from_viewport(&vol_vp);

    draw_volume_axis(
        renderer,
        &volume_panel.rect,
        &vol_range,
        &vol_transform,
        config,
    );
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
    // Volume panel legend
    draw_label_in_panel(renderer, volume_panel.rect, "Volume", config);

    renderer.restore_clip();
    renderer.draw_rect_outline(
        volume_panel.rect,
        &LineStyle {
            color: config.axis_color,
            width: 1.0,
        },
    );
    layout_info.panels.push(PanelInfo {
        rect: volume_panel.rect,
        kind: PanelKind::Volume,
    });

    // --- Sub-panel indicators (RSI, MACD, etc.) ---
    for (idx, sub_ind) in sub_panels.iter().enumerate() {
        let panel = layout.get(2 + idx).unwrap();
        draw_indicator_sub_panel(
            renderer,
            panel.rect,
            sub_ind,
            bar_slots,
            config,
            &mut color_idx,
        );
        layout_info.panels.push(PanelInfo {
            rect: panel.rect,
            kind: PanelKind::Indicator(sub_ind.name.clone()),
        });
    }

    // X-axis labels below the bottommost panel
    let bottom_panel = layout.get(layout.len() - 1).unwrap();
    draw_x_axis(renderer, data, &bottom_panel.rect, &price_transform, config);

    layout_info.price_transform = Some(price_transform);
    layout_info.bar_slots = bar_slots;
    layout_info
}

/// Draw overlay indicator series (lines on the price panel).
fn draw_indicator_overlay(
    renderer: &mut dyn Renderer,
    output: &IndicatorOutput,
    transform: &Transform,
    config: &ChartConfig,
    color_idx: &mut usize,
) {
    for series in &output.series {
        if series.style_hint != SeriesStyle::Line {
            continue;
        }
        let color = config.indicator_colors[*color_idx % config.indicator_colors.len()];
        *color_idx += 1;

        let style = LineStyle { color, width: 1.5 };
        draw_series_line(renderer, &series.values, transform, &style);
    }
}

/// Draw a sub-panel indicator (RSI, MACD).
#[allow(clippy::cast_precision_loss)]
fn draw_indicator_sub_panel(
    renderer: &mut dyn Renderer,
    panel_rect: Rect,
    output: &IndicatorOutput,
    num_bars: usize,
    config: &ChartConfig,
    color_idx: &mut usize,
) {
    // Clip to panel + right margin for Y-axis labels
    let clip_rect = Rect::new(
        panel_rect.x,
        panel_rect.y,
        panel_rect.width + config.margin.right,
        panel_rect.height,
    );
    renderer.clip(clip_rect);

    let data_rect = inset_rect_horizontal(&panel_rect, num_bars);
    let time_range = TimeRange::new(0, num_bars);

    // Determine Y range
    let y_range = match output.placement {
        IndicatorPlacement::SubPanel { y_min, y_max } => PriceRange::new(y_min, y_max),
        IndicatorPlacement::SubPanelAuto => {
            let mut min = f64::MAX;
            let mut max = f64::MIN;
            for s in &output.series {
                if s.style_hint == SeriesStyle::HorizontalLine {
                    continue;
                }
                for &v in &s.values {
                    if !v.is_nan() {
                        if v < min {
                            min = v;
                        }
                        if v > max {
                            max = v;
                        }
                    }
                }
            }
            if min > max {
                PriceRange::new(-1.0, 1.0)
            } else {
                PriceRange::new(min, max).with_padding(0.1)
            }
        }
        IndicatorPlacement::Overlay => return, // shouldn't happen
    };

    let vp = Viewport {
        rect: data_rect,
        time_range,
        price_range: y_range,
    };
    let transform = Transform::from_viewport(&vp);

    // Y-axis labels
    draw_sub_panel_y_axis(renderer, &panel_rect, &y_range, &transform, config);

    // Panel name label
    let text_style = TextStyle {
        color: config.text_color,
        size: config.font_size,
        font_family: "monospace".to_string(),
    };
    renderer.draw_text(
        &output.name,
        Point {
            x: panel_rect.x + 5.0,
            y: panel_rect.y + config.font_size + 2.0,
        },
        &text_style,
        TextAnchor::Start,
    );

    // Draw each series
    for series in &output.series {
        let color = if series.style_hint == SeriesStyle::HorizontalLine {
            config.grid_color
        } else {
            let c = config.indicator_colors[*color_idx % config.indicator_colors.len()];
            *color_idx += 1;
            c
        };

        match series.style_hint {
            SeriesStyle::Line => {
                draw_series_line(
                    renderer,
                    &series.values,
                    &transform,
                    &LineStyle { color, width: 1.5 },
                );
            }
            SeriesStyle::Histogram => {
                draw_series_histogram(
                    renderer,
                    &series.values,
                    &transform,
                    color,
                    config.body_ratio,
                );
            }
            SeriesStyle::HorizontalLine => {
                if let Some(&val) = series.values.first() {
                    let y = transform.price_y(val);
                    renderer.draw_line(
                        Point { x: panel_rect.x, y },
                        Point {
                            x: panel_rect.right(),
                            y,
                        },
                        &LineStyle { color, width: 0.5 },
                    );
                }
            }
        }
    }

    renderer.restore_clip();
    renderer.draw_rect_outline(
        panel_rect,
        &LineStyle {
            color: config.axis_color,
            width: 1.0,
        },
    );
}

/// Draw Y-axis labels and grid lines for a sub-panel.
#[allow(clippy::cast_precision_loss)]
fn draw_sub_panel_y_axis(
    renderer: &mut dyn Renderer,
    panel_rect: &Rect,
    y_range: &PriceRange,
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

    let num_labels: i32 = 4;
    let step = y_range.span() / f64::from(num_labels);

    for i in 1..num_labels {
        let val = y_range.min + step * f64::from(i);
        let y = transform.price_y(val);

        renderer.draw_line(
            Point { x: panel_rect.x, y },
            Point {
                x: panel_rect.right(),
                y,
            },
            &grid_style,
        );

        renderer.draw_text(
            &format!("{val:.1}"),
            Point {
                x: panel_rect.right() + 5.0,
                y: y + 4.0,
            },
            &text_style,
            TextAnchor::Start,
        );
    }
}

/// Draw a line series, splitting at NaN gaps.
#[allow(clippy::cast_precision_loss)]
fn draw_series_line(
    renderer: &mut dyn Renderer,
    values: &[f64],
    transform: &Transform,
    style: &LineStyle,
) {
    let mut segment: Vec<Point> = Vec::new();

    for (i, &v) in values.iter().enumerate() {
        if v.is_nan() {
            if segment.len() >= 2 {
                renderer.draw_path(&segment, style);
            }
            segment.clear();
        } else {
            segment.push(transform.to_pixel(i as f64, v));
        }
    }

    if segment.len() >= 2 {
        renderer.draw_path(&segment, style);
    }
}

/// Draw a legend for overlay indicators in the top-left of a panel.
#[allow(clippy::cast_precision_loss)]
fn draw_panel_legend(
    renderer: &mut dyn Renderer,
    panel_rect: Rect,
    overlays: &[&IndicatorOutput],
    config: &ChartConfig,
) {
    let font_size = config.font_size - 1.0;
    let text_style = TextStyle {
        color: config.text_color,
        size: font_size,
        font_family: "monospace".to_string(),
    };

    let mut x = panel_rect.x + 6.0;
    let y = panel_rect.y + font_size + 4.0;
    let line_len = 14.0;
    let gap = 8.0;
    let mut color_idx = 0;

    for overlay in overlays {
        for series in &overlay.series {
            if series.style_hint != SeriesStyle::Line {
                continue;
            }
            let color = config.indicator_colors[color_idx % config.indicator_colors.len()];
            color_idx += 1;

            // Color swatch (short line)
            renderer.draw_line(
                Point {
                    x,
                    y: y - font_size * 0.3,
                },
                Point {
                    x: x + line_len,
                    y: y - font_size * 0.3,
                },
                &LineStyle { color, width: 2.0 },
            );
            x += line_len + 3.0;

            // Label
            let label = if overlay.series.len() == 1 {
                overlay.name.clone()
            } else {
                format!("{} ({})", overlay.name, series.name)
            };
            renderer.draw_text(&label, Point { x, y }, &text_style, TextAnchor::Start);
            x += label.len() as f64 * font_size * 0.6 + gap;
        }
    }
}

/// Draw a simple text label in the top-left of a panel.
fn draw_label_in_panel(
    renderer: &mut dyn Renderer,
    panel_rect: Rect,
    label: &str,
    config: &ChartConfig,
) {
    let text_style = TextStyle {
        color: config.text_color,
        size: config.font_size - 1.0,
        font_family: "monospace".to_string(),
    };
    renderer.draw_text(
        label,
        Point {
            x: panel_rect.x + 6.0,
            y: panel_rect.y + config.font_size + 3.0,
        },
        &text_style,
        TextAnchor::Start,
    );
}

/// Draw trendlines and Fibonacci retracements on the price panel.
#[allow(clippy::cast_precision_loss)]
fn draw_annotations(
    renderer: &mut dyn Renderer,
    annotations: &Annotations,
    transform: &Transform,
    panel_rect: &Rect,
    bar_slots: usize,
    config: &ChartConfig,
) {
    let offset = config.visible_offset as f64;

    // Trend lines
    for line in &annotations.trend_lines {
        let color = Color::rgb(line.color.0, line.color.1, line.color.2);
        let style = LineStyle {
            color,
            width: line.width,
        };

        // Convert absolute bar indices to visible-relative
        let rel_start = line.start_bar - offset;
        let rel_end = line.end_bar - offset;

        let start = transform.to_pixel(rel_start, line.start_price);
        let mut end = transform.to_pixel(rel_end, line.end_price);

        if line.extend_right && (line.end_bar - line.start_bar).abs() > f64::EPSILON {
            let slope = (line.end_price - line.start_price) / (line.end_bar - line.start_bar);
            let extended_bar = bar_slots as f64;
            let extended_price = line.end_price + slope * (extended_bar + offset - line.end_bar);
            end = transform.to_pixel(extended_bar, extended_price);
        }

        renderer.draw_line(start, end, &style);
    }

    // Corridors (parallel trendlines)
    for corridor in &annotations.corridors {
        let line = &corridor.line;
        let color = Color::rgba(line.color.0, line.color.1, line.color.2, 150);
        let fill_color = Color::rgba(line.color.0, line.color.1, line.color.2, 25);
        let style = LineStyle {
            color,
            width: line.width,
        };

        let rel_start = line.start_bar - offset;
        let rel_end = if line.extend_right {
            bar_slots as f64
        } else {
            line.end_bar - offset
        };

        let slope = if (line.end_bar - line.start_bar).abs() > f64::EPSILON {
            (line.end_price - line.start_price) / (line.end_bar - line.start_bar)
        } else {
            0.0
        };

        let p1_upper = line.start_price + corridor.offset;
        let p2_upper =
            line.start_price + slope * (rel_end + offset - line.start_bar) + corridor.offset;
        let p1_lower = line.start_price;
        let p2_lower = line.start_price + slope * (rel_end + offset - line.start_bar);

        // Upper line
        renderer.draw_line(
            transform.to_pixel(rel_start, p1_upper),
            transform.to_pixel(rel_end, p2_upper),
            &style,
        );
        // Lower line
        renderer.draw_line(
            transform.to_pixel(rel_start, p1_lower),
            transform.to_pixel(rel_end, p2_lower),
            &style,
        );
        // Fill between
        renderer.draw_rect(
            Rect::new(
                transform.to_pixel(rel_start, 0.0).x,
                transform.price_y(p1_upper.max(p1_lower)),
                transform.to_pixel(rel_end, 0.0).x - transform.to_pixel(rel_start, 0.0).x,
                (transform.price_y(p1_upper.min(p1_lower))
                    - transform.price_y(p1_upper.max(p1_lower)))
                .abs(),
            ),
            &FillStyle { color: fill_color },
        );
    }

    // Fibonacci retracements
    for fib in &annotations.fibonaccis {
        let color = Color::rgb(fib.color.0, fib.color.1, fib.color.2);
        let text_style = TextStyle {
            color,
            size: config.font_size - 1.0,
            font_family: "monospace".to_string(),
        };

        let left_x = panel_rect.x;
        let right_x = panel_rect.right();

        for (level, price) in fib.level_prices() {
            let y = transform.price_y(price);
            let alpha = if level < f64::EPSILON || (level - 1.0).abs() < f64::EPSILON {
                180
            } else {
                80
            };
            let line_color = Color::rgba(fib.color.0, fib.color.1, fib.color.2, alpha);

            renderer.draw_line(
                Point { x: left_x, y },
                Point { x: right_x, y },
                &LineStyle {
                    color: line_color,
                    width: 0.5,
                },
            );

            renderer.draw_text(
                &format!("{:.1}% ({:.2})", level * 100.0, price),
                Point {
                    x: left_x + 5.0,
                    y: y - 3.0,
                },
                &text_style,
                TextAnchor::Start,
            );
        }
    }
}

/// Draw markers on the price panel.
#[allow(clippy::cast_precision_loss)]
fn draw_markers(
    renderer: &mut dyn Renderer,
    markers: &[&Marker],
    data: &[Ohlcv],
    transform: &Transform,
    config: &ChartConfig,
) {
    let bar_width = transform.bar_width();
    // Marker radius = 25% of bar width (so diameter = 50% of candle width).
    // Clamp to a reasonable minimum so markers stay visible when zoomed out.
    let marker_radius = (bar_width * 0.25).max(3.0);
    let offset = marker_radius * 0.75; // distance from high/low

    let text_style = TextStyle {
        color: Color::LIGHT_GRAY,
        size: config.font_size - 1.0,
        font_family: "monospace".to_string(),
    };

    for marker in markers {
        if marker.bar_index >= data.len() {
            continue;
        }
        let bar = &data[marker.bar_index];
        let x = transform.bar_x(marker.bar_index);

        let (cy, label_y, label_anchor) = match marker.position {
            MarkerPosition::BelowBar => {
                let y = transform.price_y(bar.low) + offset + marker_radius;
                (y, y + marker_radius + 2.0, TextAnchor::Middle)
            }
            MarkerPosition::AboveBar => {
                let y = transform.price_y(bar.high) - offset - marker_radius;
                (y, y - marker_radius - 2.0, TextAnchor::Middle)
            }
        };

        let color = Color::rgba(
            marker.color.0,
            marker.color.1,
            marker.color.2,
            marker.color.3,
        );

        match marker.shape {
            MarkerShape::ArrowUp => {
                let top = Point {
                    x,
                    y: cy - marker_radius,
                };
                let bl = Point {
                    x: x - marker_radius * 0.6,
                    y: cy,
                };
                let br = Point {
                    x: x + marker_radius * 0.6,
                    y: cy,
                };
                renderer.draw_path(&[top, br, bl, top], &LineStyle { color, width: 2.0 });
            }
            MarkerShape::ArrowDown => {
                let bottom = Point {
                    x,
                    y: cy + marker_radius,
                };
                let tl = Point {
                    x: x - marker_radius * 0.6,
                    y: cy,
                };
                let tr = Point {
                    x: x + marker_radius * 0.6,
                    y: cy,
                };
                renderer.draw_path(&[bottom, tl, tr, bottom], &LineStyle { color, width: 2.0 });
            }
            MarkerShape::Circle => {
                // Filled circle (ball) marker
                renderer.draw_circle(
                    Point { x, y: cy },
                    marker_radius,
                    &FillStyle { color },
                );
            }
            MarkerShape::Diamond => {
                let s = marker_radius * 0.6;
                let pts = [
                    Point { x, y: cy - s },
                    Point { x: x + s, y: cy },
                    Point { x, y: cy + s },
                    Point { x: x - s, y: cy },
                    Point { x, y: cy - s },
                ];
                renderer.draw_path(&pts, &LineStyle { color, width: 2.0 });
            }
        }

        // Label
        if !marker.label.is_empty() {
            renderer.draw_text(
                &marker.label,
                Point { x, y: label_y },
                &text_style,
                label_anchor,
            );
        }
    }
}

/// Draw a histogram series (bars above/below zero line).
fn draw_series_histogram(
    renderer: &mut dyn Renderer,
    values: &[f64],
    transform: &Transform,
    color: Color,
    body_ratio: f64,
) {
    let zero_y = transform.price_y(0.0);
    let bar_w = transform.bar_width() * body_ratio;

    for (i, &v) in values.iter().enumerate() {
        if v.is_nan() {
            continue;
        }
        let x = transform.bar_x(i);
        let val_y = transform.price_y(v);
        let (top, height) = if val_y < zero_y {
            (val_y, zero_y - val_y)
        } else {
            (zero_y, val_y - zero_y)
        };
        if height > 0.5 {
            renderer.draw_rect(
                Rect::new(x - bar_w / 2.0, top, bar_w, height),
                &FillStyle { color },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_data() -> Vec<Ohlcv> {
        vec![
            Ohlcv {
                timestamp: 1_700_000_000,
                open: 100.0,
                high: 110.0,
                low: 95.0,
                close: 108.0,
                volume: 5000.0,
                institutional_ratio: 0.0,
            },
            Ohlcv {
                timestamp: 1_700_086_400,
                open: 108.0,
                high: 115.0,
                low: 105.0,
                close: 112.0,
                volume: 6000.0,
                institutional_ratio: 0.0,
            },
            Ohlcv {
                timestamp: 1_700_172_800,
                open: 112.0,
                high: 118.0,
                low: 100.0,
                close: 102.0,
                volume: 8000.0,
                institutional_ratio: 0.0,
            },
            Ohlcv {
                timestamp: 1_700_259_200,
                open: 102.0,
                high: 108.0,
                low: 98.0,
                close: 106.0,
                volume: 4000.0,
                institutional_ratio: 0.0,
            },
            Ohlcv {
                timestamp: 1_700_345_600,
                open: 106.0,
                high: 120.0,
                low: 104.0,
                close: 118.0,
                volume: 7000.0,
                institutional_ratio: 0.0,
            },
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
    fn format_timestamp_daily() {
        let label = format_timestamp(1_700_000_000, 86_400);
        // 2023-11-14 → just "14"
        assert_eq!(label, "14");
    }

    #[test]
    fn format_timestamp_hourly() {
        // 1700000000 + 3600*10 = 10:00 UTC
        let label = format_timestamp(1_700_000_000 + 3600 * 10, 3600);
        assert!(label.contains(":00"));
    }

    #[test]
    fn format_timestamp_minute() {
        let label = format_timestamp(1_700_000_000 + 3600 * 14 + 60 * 30, 60);
        assert!(label.contains(':'));
    }

    #[test]
    fn detect_interval_daily() {
        let data = sample_data();
        let interval = detect_interval(&data);
        assert_eq!(interval, 86_400);
    }

    #[test]
    fn format_volume_with_suffix() {
        assert_eq!(format_volume(500.0), "500");
        assert_eq!(format_volume(5_000.0), "5.0K");
        assert_eq!(format_volume(1_500_000.0), "1.5M");
    }

    #[test]
    fn render_full_chart_with_indicators() {
        use ferrochart_core::Indicator;
        use ferrochart_core::indicator::{BollingerBands, Ema, Macd, Rsi, Sma};

        let data = sample_data();
        let indicators: Vec<IndicatorOutput> = vec![
            Sma { period: 3 }.compute(&data),
            Ema { period: 3 }.compute(&data),
            BollingerBands {
                period: 3,
                std_dev: 2.0,
            }
            .compute(&data),
            Rsi { period: 3 }.compute(&data),
            Macd {
                fast_period: 2,
                slow_period: 3,
                signal_period: 2,
            }
            .compute(&data),
        ];

        let mut r = crate::SvgRenderer::new(900.0, 600.0);
        let config = ChartConfig {
            height: 600.0,
            ..ChartConfig::default()
        };
        render_full_chart(&mut r, &data, &indicators, &config);
        let out = String::from_utf8(r.finish()).unwrap();

        // Should have paths (indicator lines)
        assert!(out.contains("<path"), "expected indicator line paths");
        // Should have sub-panel labels
        assert!(out.contains("RSI"), "expected RSI panel label");
        assert!(out.contains("MACD"), "expected MACD panel label");
    }

    #[test]
    fn render_full_chart_no_indicators() {
        let mut r = crate::SvgRenderer::new(900.0, 500.0);
        render_full_chart(&mut r, &sample_data(), &[], &ChartConfig::default());
        let out = String::from_utf8(r.finish()).unwrap();
        assert!(out.contains("<svg"));
        assert!(out.contains("<rect"));
    }

    #[test]
    fn render_full_chart_empty_data() {
        let mut r = crate::SvgRenderer::new(900.0, 500.0);
        render_full_chart(&mut r, &[], &[], &ChartConfig::default());
        let out = String::from_utf8(r.finish()).unwrap();
        assert!(out.contains("<svg"));
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

    #[test]
    fn split_candle_produces_two_rects_per_body() {
        // One bar with institutional_ratio = 0.5 should produce 2 body rects
        let data = vec![Ohlcv {
            timestamp: 1_700_000_000,
            open: 100.0,
            high: 110.0,
            low: 95.0,
            close: 108.0,
            volume: 5000.0,
            institutional_ratio: 0.5,
        }];
        let mut r = crate::SvgRenderer::new(900.0, 500.0);
        render_candlestick_chart(&mut r, &data, &ChartConfig::default());
        let out = String::from_utf8(r.finish()).unwrap();
        // 2 body rects (split) + 1 background + 1 chart border = 4
        let rect_count = out.matches("<rect").count();
        assert_eq!(
            rect_count, 4,
            "expected 4 rects (2 split body + bg + border), got {rect_count}"
        );
    }

    #[test]
    fn no_split_when_ratio_is_zero() {
        let data = vec![Ohlcv {
            timestamp: 1_700_000_000,
            open: 100.0,
            high: 110.0,
            low: 95.0,
            close: 108.0,
            volume: 5000.0,
            institutional_ratio: 0.0,
        }];
        let mut r = crate::SvgRenderer::new(900.0, 500.0);
        render_candlestick_chart(&mut r, &data, &ChartConfig::default());
        let out = String::from_utf8(r.finish()).unwrap();
        // 1 body rect + 1 background + 1 chart border = 3
        let rect_count = out.matches("<rect").count();
        assert_eq!(
            rect_count, 3,
            "expected 3 rects (1 body + bg + border), got {rect_count}"
        );
    }

    #[test]
    fn split_candle_uses_institutional_color() {
        let config = ChartConfig {
            institutional_color: Color::rgb(0, 120, 255),
            ..ChartConfig::default()
        };
        let data = vec![Ohlcv {
            timestamp: 1_700_000_000,
            open: 100.0,
            high: 110.0,
            low: 95.0,
            close: 108.0,
            volume: 5000.0,
            institutional_ratio: 0.4,
        }];
        let mut r = crate::SvgRenderer::new(900.0, 500.0);
        render_candlestick_chart(&mut r, &data, &config);
        let out = String::from_utf8(r.finish()).unwrap();
        // The institutional color rgb(0,120,255) should appear in the SVG
        assert!(
            out.contains("rgb(0,120,255)"),
            "expected institutional color in SVG output"
        );
    }

    #[test]
    fn mixed_split_and_normal_candles() {
        let data = vec![
            Ohlcv {
                timestamp: 1_700_000_000,
                open: 100.0,
                high: 110.0,
                low: 95.0,
                close: 108.0,
                volume: 5000.0,
                institutional_ratio: 0.0, // normal: 1 rect
            },
            Ohlcv {
                timestamp: 1_700_086_400,
                open: 108.0,
                high: 115.0,
                low: 105.0,
                close: 112.0,
                volume: 6000.0,
                institutional_ratio: 0.7, // split: 2 rects
            },
        ];
        let mut r = crate::SvgRenderer::new(900.0, 500.0);
        render_candlestick_chart(&mut r, &data, &ChartConfig::default());
        let out = String::from_utf8(r.finish()).unwrap();
        // 1 normal body + 2 split body + 1 border + 1 background = ?
        // Let's just verify the institutional color appears exactly once
        let inst_color = ChartConfig::default().institutional_color.to_css();
        let inst_count = out.matches(&inst_color).count();
        assert_eq!(
            inst_count, 1,
            "expected 1 institutional rect, got {inst_count}"
        );
    }
}
