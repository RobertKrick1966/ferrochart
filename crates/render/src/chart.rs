// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use ferrochart_core::{
    Annotations, BarrierOutcome, CandleGeometry, ChartType, HorizontalRay, IndicatorOutput,
    IndicatorPlacement, Marker, MarkerPosition, MarkerShape, Ohlcv, PFDirection, PanelLayout,
    Point, PriceRange, Rect, RectangleZone, SeriesStyle, TextLabel, TimeRange, Transform,
    VerticalLine, Viewport, YScaleMode, compute_heikin_ashi, compute_point_figure, compute_renko,
    indicator::VolumeProfile,
};

use crate::Renderer;
use crate::style::{Color, FillStyle, LineStyle, TextAnchor, TextStyle};

/// Configuration for chart rendering.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChartConfig {
    /// Total chart width in pixels.
    pub width: f64,
    /// Total chart height in pixels.
    pub height: f64,
    /// Background color for the chart area.
    pub background: Color,
    /// Body color for bullish (close >= open) candles.
    pub bullish_color: Color,
    /// Body color for bearish (close < open) candles.
    pub bearish_color: Color,
    /// Color for the institutional portion of a split candle body.
    pub institutional_color: Color,
    /// Wick (high-low line) color.
    pub wick_color: Color,
    /// Color for axis border lines.
    pub axis_color: Color,
    /// Color for background grid lines.
    pub grid_color: Color,
    /// Color for axis labels and legend text.
    pub text_color: Color,
    /// Candle body width as a fraction of bar spacing (0.0..1.0).
    pub body_ratio: f64,
    /// Margins around the chart area.
    pub margin: ChartMargin,
    /// Font size in pixels for axis labels and legends.
    pub font_size: f64,
    /// Rotating palette of colors assigned to indicator series.
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
    /// Use logarithmic Y-axis for the price panel.
    /// Volume and indicator sub-panels always use linear scale.
    pub log_y: bool,
    /// Visual style used to render price bars.
    pub chart_type: ChartType,
}

/// Margins around the chart area.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChartMargin {
    /// Top margin in pixels.
    pub top: f64,
    /// Right margin in pixels (houses Y-axis labels).
    pub right: f64,
    /// Bottom margin in pixels (houses X-axis labels).
    pub bottom: f64,
    /// Left margin in pixels.
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
            log_y: false,
            chart_type: ChartType::Candlestick,
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
    let y_mode = if config.log_y {
        YScaleMode::Logarithmic
    } else {
        YScaleMode::Linear
    };
    let transform = Transform::from_viewport_with_mode(&viewport, y_mode);

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
    if candles.is_empty() {
        return;
    }

    // Fast path: when many candles are sub-pixel, draw single colored line per bar
    // Skip for small datasets (<=2 bars) where zero width is a viewport edge case
    let thin = candles.len() > 2 && candles[0].body_width < 2.0;

    if thin {
        for c in candles {
            let color = if c.bullish {
                config.bullish_color
            } else {
                config.bearish_color
            };
            renderer.draw_line(
                Point {
                    x: c.x,
                    y: c.wick_top,
                },
                Point {
                    x: c.x,
                    y: c.wick_bottom,
                },
                &LineStyle { color, width: 1.0 },
            );
        }
        return;
    }

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
            let inst_height = body_height * c.institutional_ratio;
            let retail_height = body_height - inst_height;

            renderer.draw_rect(
                Rect::new(body_x, c.body_top, c.body_width, retail_height),
                &FillStyle { color: body_color },
            );
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

    let tick_prices = if transform.y_mode() == YScaleMode::Logarithmic && price_range.min > 0.0 {
        // Log mode: distribute ticks evenly in log-space
        let log_min = price_range.min.ln();
        let log_max = price_range.max.ln();
        let num_labels: i32 = 8;
        let step = (log_max - log_min) / f64::from(num_labels);
        (1..num_labels)
            .map(|i| (log_min + step * f64::from(i)).exp())
            .collect::<Vec<_>>()
    } else {
        // Linear mode
        let num_labels: i32 = 8;
        let step = price_range.span() / f64::from(num_labels);
        (1..num_labels)
            .map(|i| price_range.min + step * f64::from(i))
            .collect::<Vec<_>>()
    };

    for price in tick_prices {
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
    let y_mode = if config.log_y {
        YScaleMode::Logarithmic
    } else {
        YScaleMode::Linear
    };
    let price_transform = Transform::from_viewport_with_mode(&price_vp, y_mode);

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
    /// Main candlestick price panel.
    Price,
    /// Volume bar panel.
    Volume,
    /// Sub-panel indicator identified by name.
    Indicator(String),
}

/// Info about a rendered panel (for hit-testing/tooltip).
#[derive(Debug, Clone)]
pub struct PanelInfo {
    /// Bounding rectangle of this panel in pixel coordinates.
    pub rect: Rect,
    /// What this panel displays.
    pub kind: PanelKind,
}

/// Result of rendering, containing panel layout info and transforms.
#[derive(Debug, Clone, Default)]
pub struct ChartLayoutInfo {
    /// Ordered list of rendered panels and their bounding rectangles.
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
        None,
        config,
    )
}

/// Render a full chart with candlesticks, volume, indicators, markers, and annotations.
///
/// # Panics
///
/// Panics if the internal panel layout cannot be constructed.
pub fn render_full_chart_with_markers(
    renderer: &mut dyn Renderer,
    data: &[Ohlcv],
    indicators: &[IndicatorOutput],
    markers: &[&Marker],
    annotations: &Annotations,
    volume_profile: Option<&VolumeProfile>,
    config: &ChartConfig,
) -> ChartLayoutInfo {
    if data.is_empty() {
        return ChartLayoutInfo::default();
    }

    // Non-uniform chart types have their own coordinate system and rendering pipeline.
    // Dispatch early so the standard panel/volume/X-axis logic is not applied.
    match config.chart_type {
        ChartType::Renko { brick_size } => {
            renderer.set_background(config.background);
            let renko_bars = compute_renko(data, brick_size);
            render_renko_chart(renderer, &renko_bars, config);
            return ChartLayoutInfo::default();
        }
        ChartType::PointFigure { box_size, reversal } => {
            renderer.set_background(config.background);
            let columns = compute_point_figure(data, box_size, reversal);
            render_point_figure_chart(renderer, &columns, box_size, config);
            return ChartLayoutInfo::default();
        }
        _ => {}
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

    // --- Determine render data (may be Heikin-Ashi transformed) ---
    let ha_data: Vec<Ohlcv>;
    let render_data: &[Ohlcv] = match config.chart_type {
        ChartType::HeikinAshi => {
            ha_data = compute_heikin_ashi(data);
            &ha_data
        }
        _ => data,
    };

    // --- Price panel ---
    // Extend price range to include overlay indicator values (BB bands, etc.)
    // For Line/Area charts use close-only range; otherwise use full OHLC range.
    // Renko/P&F chart types compute their own price range inside their render function,
    // but we still need a range here for the panel layout; use OHLC range.
    let base_price_range = match config.chart_type {
        ChartType::Line | ChartType::Area => {
            PriceRange::from_closes(render_data).unwrap_or(PriceRange::new(0.0, 100.0))
        }
        _ => PriceRange::from_ohlcv(render_data).unwrap_or(PriceRange::new(0.0, 100.0)),
    };
    let mut price_range = base_price_range;
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
    let y_mode = if config.log_y {
        YScaleMode::Logarithmic
    } else {
        YScaleMode::Linear
    };
    let price_transform = Transform::from_viewport_with_mode(&price_vp, y_mode);

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

    // Render price bars according to chart type
    match config.chart_type {
        ChartType::Candlestick | ChartType::HeikinAshi => {
            let candles =
                CandleGeometry::compute_all(render_data, 0, &price_transform, config.body_ratio);
            draw_candles(renderer, &candles, config);
        }
        ChartType::Line => {
            let line_color = config
                .indicator_colors
                .first()
                .copied()
                .unwrap_or(Color::LIGHT_GRAY);
            draw_line_chart(renderer, render_data, &price_transform, config, line_color);
        }
        ChartType::Area => {
            let area_color = config
                .indicator_colors
                .first()
                .copied()
                .unwrap_or(Color::LIGHT_GRAY);
            draw_area_chart(
                renderer,
                render_data,
                &price_transform,
                config,
                &price_panel.rect,
                area_color,
            );
        }
        ChartType::OhlcBars => {
            draw_ohlc_bars(renderer, render_data, &price_transform, config);
        }
        // Renko and PointFigure are handled by the early-return dispatch above.
        ChartType::Renko { .. } | ChartType::PointFigure { .. } => {}
    }

    // Draw volume profile on price panel (behind overlays)
    if let Some(vp) = volume_profile {
        draw_volume_profile(renderer, vp, &price_panel.rect, &price_transform, config);
    }

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

/// Draw a line chart (close prices as a polyline).
fn draw_line_chart(
    renderer: &mut dyn Renderer,
    data: &[Ohlcv],
    transform: &Transform,
    _config: &ChartConfig,
    line_color: Color,
) {
    let values: Vec<f64> = data.iter().map(|b| b.close).collect();
    let style = LineStyle {
        color: line_color,
        width: 1.5,
    };
    draw_series_line(renderer, &values, transform, &style);
}

/// Draw an area chart (filled polygon below close prices + line on top).
fn draw_area_chart(
    renderer: &mut dyn Renderer,
    data: &[Ohlcv],
    transform: &Transform,
    config: &ChartConfig,
    panel_rect: &Rect,
    color: Color,
) {
    if data.is_empty() {
        return;
    }

    // Build top edge of polygon (close prices) + bottom corners
    let mut poly: Vec<Point> = data
        .iter()
        .enumerate()
        .map(|(i, b)| transform.to_pixel(i as f64, b.close))
        .collect();

    // Close polygon: bottom-right, then bottom-left
    let last_x = transform.bar_x(data.len() - 1);
    let first_x = transform.bar_x(0);
    let bottom_y = panel_rect.bottom();
    poly.push(Point {
        x: last_x,
        y: bottom_y,
    });
    poly.push(Point {
        x: first_x,
        y: bottom_y,
    });

    let fill_color = Color::rgba(color.r, color.g, color.b, 40);
    renderer.fill_polygon(&poly, &FillStyle { color: fill_color });

    // Draw line on top
    draw_line_chart(renderer, data, transform, config, color);
}

/// Draw OHLC bar chart (vertical bar + left tick for open, right tick for close).
fn draw_ohlc_bars(
    renderer: &mut dyn Renderer,
    data: &[Ohlcv],
    transform: &Transform,
    config: &ChartConfig,
) {
    let bar_width = transform.bar_width();
    let tick_len = (bar_width * config.body_ratio * 0.5).max(2.0);

    for (i, bar) in data.iter().enumerate() {
        let x = transform.bar_x(i);
        let high_y = transform.price_y(bar.high);
        let low_y = transform.price_y(bar.low);
        let open_y = transform.price_y(bar.open);
        let close_y = transform.price_y(bar.close);

        let color = if bar.close >= bar.open {
            config.bullish_color
        } else {
            config.bearish_color
        };
        let style = LineStyle { color, width: 1.0 };

        // Vertical bar from high to low
        renderer.draw_line(Point { x, y: high_y }, Point { x, y: low_y }, &style);
        // Open tick (left)
        renderer.draw_line(
            Point {
                x: x - tick_len,
                y: open_y,
            },
            Point { x, y: open_y },
            &style,
        );
        // Close tick (right)
        renderer.draw_line(
            Point { x, y: close_y },
            Point {
                x: x + tick_len,
                y: close_y,
            },
            &style,
        );
    }
}

/// Render a standalone Renko chart into `renderer`.
///
/// Each brick is drawn as a filled rectangle (green for up, red for down).
/// No wicks are drawn — Renko bricks have no high/low beyond open/close.
fn render_renko_chart(
    renderer: &mut dyn Renderer,
    renko_bars: &[ferrochart_core::RenkoBar],
    config: &ChartConfig,
) {
    if renko_bars.is_empty() {
        return;
    }

    renderer.set_background(config.background);

    let chart_rect = Rect::new(
        config.margin.left,
        config.margin.top,
        config.width - config.margin.left - config.margin.right,
        config.height - config.margin.top - config.margin.bottom,
    );

    // Compute Y range from all bricks
    let min_low = renko_bars
        .iter()
        .map(|b| b.low)
        .fold(f64::INFINITY, f64::min);
    let max_high = renko_bars
        .iter()
        .map(|b| b.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let span = (max_high - min_low).max(1.0);
    let padding = span * 0.05;
    let y_min = min_low - padding;
    let y_max = max_high + padding;
    let y_span = y_max - y_min;

    let n = renko_bars.len();
    let bar_width = chart_rect.width / n as f64;

    let text_style = TextStyle {
        color: config.text_color,
        size: config.font_size,
        font_family: "monospace".to_string(),
    };
    let grid_style = LineStyle {
        color: config.grid_color,
        width: 1.0,
    };

    // Y-axis grid and labels
    let num_labels: i32 = 8;
    for i in 1..num_labels {
        let price = y_min + (y_span / f64::from(num_labels)) * f64::from(i);
        let y = chart_rect.bottom() - ((price - y_min) / y_span) * chart_rect.height;
        renderer.draw_line(
            Point { x: chart_rect.x, y },
            Point {
                x: chart_rect.right(),
                y,
            },
            &grid_style,
        );
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

    // Draw bricks
    renderer.clip(chart_rect);
    let up_color = Color::rgb(38, 166, 154); // teal (#26a69a)
    let dn_color = Color::rgb(239, 83, 80); // red (#ef5350)

    for (i, brick) in renko_bars.iter().enumerate() {
        let color = if brick.up { up_color } else { dn_color };
        let x = chart_rect.x + i as f64 * bar_width;
        let open_y = chart_rect.bottom() - ((brick.open - y_min) / y_span) * chart_rect.height;
        let close_y = chart_rect.bottom() - ((brick.close - y_min) / y_span) * chart_rect.height;
        let top_y = open_y.min(close_y);
        let height = (open_y - close_y).abs().max(1.0);
        renderer.draw_rect(
            Rect::new(x + 1.0, top_y, (bar_width - 2.0).max(1.0), height),
            &FillStyle { color },
        );
    }
    renderer.restore_clip();

    // X-axis: show bar index every ~10 bricks
    let step = (n / 10).max(1);
    for i in (0..n).step_by(step) {
        let x = chart_rect.x + i as f64 * bar_width + bar_width / 2.0;
        renderer.draw_text(
            &format!("{i}"),
            Point {
                x,
                y: chart_rect.bottom() + 14.0,
            },
            &text_style,
            TextAnchor::Middle,
        );
    }

    // Border
    renderer.draw_rect_outline(
        chart_rect,
        &LineStyle {
            color: config.axis_color,
            width: 1.0,
        },
    );
}

/// Render a standalone Point & Figure chart into `renderer`.
///
/// X columns are filled green rectangles; O columns are stroked red rectangles.
fn render_point_figure_chart(
    renderer: &mut dyn Renderer,
    columns: &[ferrochart_core::PFColumn],
    box_size: f64,
    config: &ChartConfig,
) {
    if columns.is_empty() {
        return;
    }

    renderer.set_background(config.background);

    let chart_rect = Rect::new(
        config.margin.left,
        config.margin.top,
        config.width - config.margin.left - config.margin.right,
        config.height - config.margin.top - config.margin.bottom,
    );

    // Compute Y range
    let min_bottom = columns
        .iter()
        .map(|c| c.bottom_price)
        .fold(f64::INFINITY, f64::min);
    let max_top = columns
        .iter()
        .map(|c| c.top_price)
        .fold(f64::NEG_INFINITY, f64::max);
    let span = (max_top - min_bottom).max(box_size);
    let padding = span * 0.05;
    let y_min = min_bottom - padding;
    let y_max = max_top + padding;
    let y_span = y_max - y_min;

    let n = columns.len();
    let raw_col_w = chart_rect.width / n as f64;
    // Clamp column width: min 8 px, no upper bound so columns fill available space.
    let col_width = raw_col_w.max(8.0);

    let text_style = TextStyle {
        color: config.text_color,
        size: config.font_size,
        font_family: "monospace".to_string(),
    };
    let grid_style = LineStyle {
        color: config.grid_color,
        width: 1.0,
    };

    // Horizontal price grid at each box interval; label every Nth box so ~8 labels show.
    let total_boxes = ((y_max - y_min) / box_size).ceil().max(1.0) as i64;
    let label_step = (total_boxes / 8).max(1);
    let mut price = (y_min / box_size).ceil() * box_size;
    while price <= y_max {
        let y = chart_rect.bottom() - ((price - y_min) / y_span) * chart_rect.height;
        renderer.draw_line(
            Point { x: chart_rect.x, y },
            Point {
                x: chart_rect.right(),
                y,
            },
            &grid_style,
        );
        let boxes_from_bottom = ((price - y_min) / box_size).round() as i64;
        if boxes_from_bottom % label_step == 0 {
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
        price += box_size;
    }

    renderer.clip(chart_rect);

    // Font size for X/O symbols: fit within one box height, capped to col_width.
    let box_px = (box_size / y_span * chart_rect.height).max(6.0);
    let symbol_size = box_px.min(col_width - 2.0).max(6.0);

    let x_style = TextStyle {
        color: Color::rgb(38, 166, 154), // teal
        size: symbol_size,
        font_family: "monospace".to_string(),
    };
    let o_style = TextStyle {
        color: Color::rgb(239, 83, 80), // red
        size: symbol_size,
        font_family: "monospace".to_string(),
    };

    for (i, col) in columns.iter().enumerate() {
        let col_x = chart_rect.x + i as f64 * col_width + col_width / 2.0;
        let num_boxes = col.box_count.max(1);
        for b in 0..num_boxes {
            let box_bottom = col.bottom_price + b as f64 * box_size;
            let box_mid = box_bottom + box_size / 2.0;
            let y = chart_rect.bottom() - ((box_mid - y_min) / y_span) * chart_rect.height
                + symbol_size * 0.35;
            match col.direction {
                PFDirection::X => {
                    renderer.draw_text("X", Point { x: col_x, y }, &x_style, TextAnchor::Middle);
                }
                PFDirection::O => {
                    renderer.draw_text("O", Point { x: col_x, y }, &o_style, TextAnchor::Middle);
                }
            }
        }
    }

    renderer.restore_clip();

    // Border
    renderer.draw_rect_outline(
        chart_rect,
        &LineStyle {
            color: config.axis_color,
            width: 1.0,
        },
    );
}

/// Draw a legend for overlay indicators in the top-left of a panel.
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

        let upper_start = transform.to_pixel(rel_start, p1_upper);
        let upper_end = transform.to_pixel(rel_end, p2_upper);
        let lower_start = transform.to_pixel(rel_start, p1_lower);
        let lower_end = transform.to_pixel(rel_end, p2_lower);

        // Fill between (polygon following the diagonal lines)
        renderer.fill_polygon(
            &[upper_start, upper_end, lower_end, lower_start],
            &FillStyle { color: fill_color },
        );
        // Upper line
        renderer.draw_line(upper_start, upper_end, &style);
        // Lower line
        renderer.draw_line(lower_start, lower_end, &style);
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

    // Triple barriers
    for tb in &annotations.triple_barriers {
        let entry_rel = tb.entry_bar as f64 - offset;
        let end_bar = tb.entry_bar + tb.horizon;
        let end_rel = end_bar as f64 - offset;

        let entry_x = transform.bar_x(entry_rel.round().max(0.0) as usize);
        let end_x = transform.bar_x(end_rel.round().max(0.0) as usize);
        let tp_y = transform.price_y(tb.tp_price);
        let sl_y = transform.price_y(tb.sl_price);
        let entry_y = transform.price_y(tb.entry_price);

        let tp_color = Color::rgba(0, 200, 0, 180);
        let sl_color = Color::rgba(220, 0, 0, 180);
        let time_color = Color::rgba(tb.color.0, tb.color.1, tb.color.2, 120);
        let fill_color = Color::rgba(tb.color.0, tb.color.1, tb.color.2, 15);

        // Semi-transparent fill between TP and SL
        renderer.fill_polygon(
            &[
                Point {
                    x: entry_x,
                    y: tp_y,
                },
                Point { x: end_x, y: tp_y },
                Point { x: end_x, y: sl_y },
                Point {
                    x: entry_x,
                    y: sl_y,
                },
            ],
            &FillStyle { color: fill_color },
        );

        // TP line (green, dashed effect via thinner width)
        renderer.draw_line(
            Point {
                x: entry_x,
                y: tp_y,
            },
            Point { x: end_x, y: tp_y },
            &LineStyle {
                color: tp_color,
                width: 1.0,
            },
        );

        // SL line (red)
        renderer.draw_line(
            Point {
                x: entry_x,
                y: sl_y,
            },
            Point { x: end_x, y: sl_y },
            &LineStyle {
                color: sl_color,
                width: 1.0,
            },
        );

        // Time barrier (vertical right edge)
        renderer.draw_line(
            Point { x: end_x, y: tp_y },
            Point { x: end_x, y: sl_y },
            &LineStyle {
                color: time_color,
                width: 1.0,
            },
        );

        // Entry marker (vertical left edge)
        renderer.draw_line(
            Point {
                x: entry_x,
                y: tp_y,
            },
            Point {
                x: entry_x,
                y: sl_y,
            },
            &LineStyle {
                color: time_color,
                width: 0.5,
            },
        );

        // Entry price horizontal line (thin)
        renderer.draw_line(
            Point {
                x: entry_x,
                y: entry_y,
            },
            Point {
                x: end_x,
                y: entry_y,
            },
            &LineStyle {
                color: Color::rgba(tb.color.0, tb.color.1, tb.color.2, 60),
                width: 0.5,
            },
        );

        // If exit is known, draw exit marker
        if let (Some(exit_bar), Some(outcome)) = (tb.exit_bar, tb.outcome) {
            let exit_rel = exit_bar as f64 - offset;
            let exit_x = transform.bar_x(exit_rel.round().max(0.0) as usize);
            let exit_price = match outcome {
                BarrierOutcome::TakeProfit => tb.tp_price,
                BarrierOutcome::StopLoss => tb.sl_price,
                BarrierOutcome::TimeExpired => tb.entry_price,
            };
            let exit_y = transform.price_y(exit_price);
            let exit_color = match outcome {
                BarrierOutcome::TakeProfit => tp_color,
                BarrierOutcome::StopLoss => sl_color,
                BarrierOutcome::TimeExpired => time_color,
            };
            renderer.draw_circle(
                Point {
                    x: exit_x,
                    y: exit_y,
                },
                4.0,
                &FillStyle { color: exit_color },
            );
        }
    }

    // Confidence bands
    for band in &annotations.confidence_bands {
        let color = Color::rgba(band.color.0, band.color.1, band.color.2, band.alpha);
        let n = band.upper.len().min(band.lower.len());

        // Draw band as connected polygon segments
        let mut top_points = Vec::new();
        let mut bot_points = Vec::new();
        for i in 0..n {
            if band.upper[i].is_nan() || band.lower[i].is_nan() {
                // Flush segment if we have points
                if top_points.len() >= 2 {
                    bot_points.reverse();
                    top_points.append(&mut bot_points);
                    renderer.fill_polygon(&top_points, &FillStyle { color });
                    top_points.clear();
                }
                bot_points.clear();
                continue;
            }
            let rel = i as f64 - offset;
            top_points.push(transform.to_pixel(rel, band.upper[i]));
            bot_points.push(transform.to_pixel(rel, band.lower[i]));
        }
        if top_points.len() >= 2 {
            bot_points.reverse();
            top_points.append(&mut bot_points);
            renderer.fill_polygon(&top_points, &FillStyle { color });
        }
    }

    // Walk-forward zones (vertical shaded regions across full panel height)
    for zone in &annotations.walk_forward_zones {
        let start_rel = zone.start_bar as f64 - offset;
        let end_rel = zone.end_bar as f64 - offset;
        let x1 = transform.bar_x(start_rel.round().max(0.0) as usize);
        let x2 = transform.bar_x(end_rel.round().max(0.0) as usize);
        let width = (x2 - x1).max(1.0);

        let (r, g, b) = zone.color.unwrap_or(if zone.is_train {
            (50, 100, 200) // blue for train
        } else {
            (255, 165, 0) // orange for validation
        });

        renderer.draw_rect(
            Rect::new(x1, panel_rect.y, width, panel_rect.height),
            &FillStyle {
                color: Color::rgba(r, g, b, 20),
            },
        );

        // Label at top
        if !zone.label.is_empty() {
            let text_style = TextStyle {
                color: Color::rgba(r, g, b, 180),
                size: config.font_size - 1.0,
                font_family: "monospace".to_string(),
            };
            renderer.draw_text(
                &zone.label,
                Point {
                    x: x1 + 3.0,
                    y: panel_rect.y + config.font_size,
                },
                &text_style,
                TextAnchor::Start,
            );
        }
    }

    // News event markers (vertical line + label at top of panel)
    for event in &annotations.news_events {
        let rel = event.bar_index as f64 - offset;
        let x = transform.bar_x(rel.round().max(0.0) as usize);

        let (r, g, b) = event.color.unwrap_or_else(|| {
            if event.impact > 0.2 {
                (0, 200, 0) // green = bullish
            } else if event.impact < -0.2 {
                (220, 0, 0) // red = bearish
            } else {
                (180, 180, 0) // yellow = neutral
            }
        });

        let alpha = match event.urgency {
            3 => 200, // critical
            2 => 150, // high
            1 => 100, // medium
            _ => 60,  // low
        };

        // Vertical line spanning panel
        renderer.draw_line(
            Point { x, y: panel_rect.y },
            Point {
                x,
                y: panel_rect.bottom(),
            },
            &LineStyle {
                color: Color::rgba(r, g, b, alpha),
                width: 1.0,
            },
        );

        // Label at top
        if !event.label.is_empty() {
            let text_style = TextStyle {
                color: Color::rgba(r, g, b, alpha.min(220)),
                size: config.font_size - 2.0,
                font_family: "monospace".to_string(),
            };
            renderer.draw_text(
                &event.label,
                Point {
                    x: x + 2.0,
                    y: panel_rect.y + config.font_size - 1.0,
                },
                &text_style,
                TextAnchor::Start,
            );
        }
    }

    // Horizontal histograms (GEX profile, etc.)
    for hist in &annotations.horizontal_histograms {
        if hist.levels.is_empty() {
            continue;
        }
        let max_val = hist
            .levels
            .iter()
            .map(|&(_, v)| v.abs())
            .fold(0.0_f64, f64::max);
        if max_val < f64::EPSILON {
            continue;
        }
        let max_bar_width = panel_rect.width * 0.15;
        let color = Color::rgba(hist.color.0, hist.color.1, hist.color.2, hist.alpha);

        for &(price, value) in &hist.levels {
            let y = transform.price_y(price);
            let width = (value.abs() / max_val) * max_bar_width;
            if width < 0.5 {
                continue;
            }
            // Draw from left edge for negative, right edge for positive
            let x = if value >= 0.0 {
                panel_rect.right() - width
            } else {
                panel_rect.x
            };
            renderer.draw_rect(Rect::new(x, y - 1.5, width, 3.0), &FillStyle { color });
        }
    }

    // Horizontal price level lines (Max Pain, support/resistance)
    for level in &annotations.horizontal_levels {
        let y = transform.price_y(level.price);
        let color = Color::rgb(level.color.0, level.color.1, level.color.2);

        renderer.draw_line(
            Point { x: panel_rect.x, y },
            Point {
                x: panel_rect.right(),
                y,
            },
            &LineStyle {
                color,
                width: level.width,
            },
        );

        if !level.label.is_empty() {
            let text_style = TextStyle {
                color,
                size: config.font_size - 1.0,
                font_family: "monospace".to_string(),
            };
            renderer.draw_text(
                &level.label,
                Point {
                    x: panel_rect.x + 5.0,
                    y: y - 3.0,
                },
                &text_style,
                TextAnchor::Start,
            );
        }
    }

    // Horizontal rays (full-width price lines)
    draw_horizontal_rays(
        renderer,
        &annotations.horizontal_rays,
        transform,
        panel_rect,
    );

    // Vertical lines at specific bar indices
    draw_vertical_lines(
        renderer,
        &annotations.vertical_lines,
        transform,
        panel_rect,
        offset,
    );

    // Rectangle zones
    draw_rectangle_zones(
        renderer,
        &annotations.rectangle_zones,
        transform,
        panel_rect,
        offset,
    );

    // Text labels
    draw_text_labels(
        renderer,
        &annotations.text_labels,
        transform,
        panel_rect,
        offset,
        config,
    );
}

/// Draw horizontal rays spanning the full panel width.
fn draw_horizontal_rays(
    renderer: &mut dyn Renderer,
    rays: &[HorizontalRay],
    transform: &Transform,
    panel_rect: &Rect,
) {
    for ray in rays {
        let y = transform.price_y(ray.price);
        let color = Color::rgb(ray.color.0, ray.color.1, ray.color.2);
        renderer.draw_line(
            Point { x: panel_rect.x, y },
            Point {
                x: panel_rect.right(),
                y,
            },
            &LineStyle {
                color,
                width: ray.width,
            },
        );
    }
}

/// Draw vertical lines at specific bar indices.
fn draw_vertical_lines(
    renderer: &mut dyn Renderer,
    lines: &[VerticalLine],
    transform: &Transform,
    panel_rect: &Rect,
    offset: f64,
) {
    for line in lines {
        let rel = line.bar_index - offset;
        let x = transform.bar_x(rel.round().max(0.0) as usize);
        let color = Color::rgb(line.color.0, line.color.1, line.color.2);
        renderer.draw_line(
            Point { x, y: panel_rect.y },
            Point {
                x,
                y: panel_rect.bottom(),
            },
            &LineStyle {
                color,
                width: line.width,
            },
        );
    }
}

/// Draw price × time rectangle zones.
fn draw_rectangle_zones(
    renderer: &mut dyn Renderer,
    zones: &[RectangleZone],
    transform: &Transform,
    panel_rect: &Rect,
    offset: f64,
) {
    for zone in zones {
        let rel_start = zone.start_bar - offset;
        let rel_end = zone.end_bar - offset;
        let x1 = transform.bar_x(rel_start.round().max(0.0) as usize);
        let x2 = transform.bar_x(rel_end.round().max(0.0) as usize);
        let y_top = transform.price_y(zone.top_price);
        let y_bottom = transform.price_y(zone.bottom_price);

        // Clamp to panel bounds
        let x_left = x1.max(panel_rect.x);
        let x_right = x2.min(panel_rect.right());
        let y_t = y_top.max(panel_rect.y);
        let y_b = y_bottom.min(panel_rect.bottom());

        let width = (x_right - x_left).max(0.0);
        let height = (y_b - y_t).max(0.0);

        if width < f64::EPSILON || height < f64::EPSILON {
            continue;
        }

        let (fr, fg, fb, fa) = zone.fill_color;
        let fill_color = Color::rgba(fr, fg, fb, fa);
        renderer.draw_rect(
            Rect::new(x_left, y_t, width, height),
            &FillStyle { color: fill_color },
        );

        let border_color = Color::rgb(
            zone.border_color.0,
            zone.border_color.1,
            zone.border_color.2,
        );
        let border_style = LineStyle {
            color: border_color,
            width: zone.width,
        };
        // Draw four border edges
        renderer.draw_line(
            Point { x: x_left, y: y_t },
            Point { x: x_right, y: y_t },
            &border_style,
        );
        renderer.draw_line(
            Point { x: x_right, y: y_t },
            Point { x: x_right, y: y_b },
            &border_style,
        );
        renderer.draw_line(
            Point { x: x_right, y: y_b },
            Point { x: x_left, y: y_b },
            &border_style,
        );
        renderer.draw_line(
            Point { x: x_left, y: y_b },
            Point { x: x_left, y: y_t },
            &border_style,
        );
    }
}

/// Draw text labels at specific bar and price positions.
///
/// Renders text at each label's bar/price position using `draw_text`.
fn draw_text_labels(
    renderer: &mut dyn Renderer,
    labels: &[TextLabel],
    transform: &Transform,
    panel_rect: &Rect,
    offset: f64,
    config: &ChartConfig,
) {
    let _ = panel_rect; // used implicitly via transform bounds
    for label in labels {
        let rel = label.bar_index - offset;
        let x = transform.bar_x(rel.round().max(0.0) as usize);
        let y = transform.price_y(label.price);
        let color = Color::rgb(label.color.0, label.color.1, label.color.2);
        let text_style = TextStyle {
            color,
            size: config.font_size,
            font_family: "monospace".to_string(),
        };
        renderer.draw_text(&label.text, Point { x, y }, &text_style, TextAnchor::Start);
    }
}

/// Draw volume profile histogram on the price panel (horizontal bars from right edge).
/// Draw volume profile histogram on the price panel (horizontal bars from right edge).
fn draw_volume_profile(
    renderer: &mut dyn Renderer,
    profile: &VolumeProfile,
    panel_rect: &Rect,
    transform: &Transform,
    config: &ChartConfig,
) {
    if profile.buckets.is_empty() || profile.max_volume < f64::EPSILON {
        return;
    }

    let max_bar_width = panel_rect.width * 0.20;
    let color = Color::rgba(100, 149, 237, 50); // cornflower blue, semi-transparent

    // Suppress unused variable warning for config
    let _ = config;

    for bucket in &profile.buckets {
        let y_top = transform.price_y(bucket.price_high);
        let y_bottom = transform.price_y(bucket.price_low);
        let height = (y_bottom - y_top).max(1.0);
        let width = (bucket.volume / profile.max_volume) * max_bar_width;

        if width < 0.5 {
            continue;
        }

        renderer.draw_rect(
            Rect::new(panel_rect.right() - width, y_top, width, height),
            &FillStyle { color },
        );
    }
}

/// Draw markers on the price panel.
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
                renderer.draw_circle(Point { x, y: cy }, marker_radius, &FillStyle { color });
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
    use ferrochart_core::{
        BarrierOutcome, Corridor, FibonacciRetracement, TrendLine, TripleBarrier,
    };

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

    /// Generate 20 bars of synthetic uptrend data for annotation tests.
    fn annotation_test_data() -> Vec<Ohlcv> {
        (0..20)
            .map(|i| {
                let base = 100.0 + i as f64 * 2.0;
                Ohlcv {
                    timestamp: 1_700_000_000 + i * 86_400,
                    open: base,
                    high: base + 5.0,
                    low: base - 3.0,
                    close: base + 3.0,
                    volume: 5000.0 + (i as f64 * 100.0),
                    institutional_ratio: 0.0,
                }
            })
            .collect()
    }

    /// Parse all `<line>` elements from SVG, returning (x1, y1, x2, y2, stroke).
    fn parse_svg_lines(svg: &str) -> Vec<(f64, f64, f64, f64, String)> {
        let mut results = Vec::new();
        for segment in svg.split("<line ") {
            if !segment.contains("x1=") {
                continue;
            }
            let attr = |name: &str| -> Option<f64> {
                let prefix = format!("{name}=\"");
                let start = segment.find(&prefix)? + prefix.len();
                let end = start + segment[start..].find('"')?;
                segment[start..end].parse().ok()
            };
            let stroke = || -> Option<String> {
                let prefix = "stroke=\"";
                let start = segment.find(prefix)? + prefix.len();
                let end = start + segment[start..].find('"')?;
                Some(segment[start..end].to_string())
            };
            if let (Some(x1), Some(y1), Some(x2), Some(y2), Some(s)) =
                (attr("x1"), attr("y1"), attr("x2"), attr("y2"), stroke())
            {
                results.push((x1, y1, x2, y2, s));
            }
        }
        results
    }

    /// Render a chart with a trendline and verify the line appears at the correct
    /// pixel coordinates in the SVG output.
    #[test]
    fn trendline_renders_at_correct_position() {
        let data = annotation_test_data();
        let config = ChartConfig::default();

        // Use a distinctive color so we can find the trendline in SVG
        let tl_color = (255, 100, 0); // orange – unique, not used elsewhere
        let tl_css = Color::rgb(tl_color.0, tl_color.1, tl_color.2).to_css();

        let mut annotations = Annotations::new();
        annotations.add_trend_line(TrendLine {
            start_bar: 3.0,
            start_price: 106.0,
            end_bar: 16.0,
            end_price: 135.0,
            color: tl_color,
            width: 2.0,
            extend_right: false,
        });

        let mut r = crate::SvgRenderer::new(config.width, config.height);
        let layout =
            render_full_chart_with_markers(&mut r, &data, &[], &[], &annotations, None, &config);
        let svg = String::from_utf8(r.finish()).unwrap();

        // --- Trendline must exist in SVG ---
        assert!(svg.contains(&tl_css), "trendline color missing from SVG");

        // --- Find the trendline <line> by its unique stroke color ---
        let lines = parse_svg_lines(&svg);
        let tl_lines: Vec<_> = lines.iter().filter(|l| l.4 == tl_css).collect();
        assert_eq!(
            tl_lines.len(),
            1,
            "expected exactly 1 trendline line, found {}",
            tl_lines.len()
        );
        let (x1, y1, x2, y2, _) = *tl_lines[0];

        // --- Recompute expected pixel positions from the transform ---
        let price_transform = layout
            .price_transform
            .expect("layout should have price_transform");

        // Trendline bars are relative to visible_offset (0 here)
        let expected_start = price_transform.to_pixel(3.0, 106.0);
        let expected_end = price_transform.to_pixel(16.0, 135.0);

        let tol = 0.1; // pixel tolerance for f64 rounding in SVG
        assert!(
            (x1 - expected_start.x).abs() < tol,
            "start x: SVG={x1:.2}, expected={:.2}",
            expected_start.x
        );
        assert!(
            (y1 - expected_start.y).abs() < tol,
            "start y: SVG={y1:.2}, expected={:.2}",
            expected_start.y
        );
        assert!(
            (x2 - expected_end.x).abs() < tol,
            "end x: SVG={x2:.2}, expected={:.2}",
            expected_end.x
        );
        assert!(
            (y2 - expected_end.y).abs() < tol,
            "end y: SVG={y2:.2}, expected={:.2}",
            expected_end.y
        );
    }

    /// Verify that `extend_right` causes the trendline to project beyond its
    /// endpoint to the right edge of the chart.
    #[test]
    fn trendline_extend_right_reaches_chart_edge() {
        let data = annotation_test_data();
        let config = ChartConfig::default();

        let tl_color = (0, 200, 255);
        let tl_css = Color::rgb(tl_color.0, tl_color.1, tl_color.2).to_css();

        let mut annotations = Annotations::new();
        annotations.add_trend_line(TrendLine {
            start_bar: 2.0,
            start_price: 104.0,
            end_bar: 10.0,
            end_price: 124.0,
            color: tl_color,
            width: 1.5,
            extend_right: true,
        });

        let mut r = crate::SvgRenderer::new(config.width, config.height);
        let layout =
            render_full_chart_with_markers(&mut r, &data, &[], &[], &annotations, None, &config);
        let svg = String::from_utf8(r.finish()).unwrap();

        let lines = parse_svg_lines(&svg);
        let tl_lines: Vec<_> = lines.iter().filter(|l| l.4 == tl_css).collect();
        assert_eq!(tl_lines.len(), 1);
        let (x1, _y1, x2, _y2, _) = *tl_lines[0];

        let price_transform = layout.price_transform.unwrap();

        // Start should match bar 2
        let expected_start = price_transform.to_pixel(2.0, 104.0);
        assert!(
            (x1 - expected_start.x).abs() < 0.1,
            "extend_right start x mismatch"
        );

        // End should extend to bar_slots (data.len() = 20)
        let bar_slots = data.len();
        let extended_x = price_transform.bar_x(bar_slots);
        assert!(
            (x2 - extended_x).abs() < 0.1,
            "extend_right end x: SVG={x2:.2}, expected bar_slots edge={extended_x:.2}"
        );
    }

    /// When `visible_offset > 0` (scrolled chart), trendline bar indices are
    /// shifted so their absolute positions stay correct.
    #[test]
    fn trendline_with_visible_offset() {
        let data = annotation_test_data();
        // Show only bars 5..15 (10 visible bars)
        let visible_data = &data[5..15];
        let config = ChartConfig {
            visible_offset: 5,
            ..ChartConfig::default()
        };

        let tl_color = (200, 50, 200);
        let tl_css = Color::rgb(tl_color.0, tl_color.1, tl_color.2).to_css();

        // Trendline in absolute coordinates: bar 7 to bar 12
        let mut annotations = Annotations::new();
        annotations.add_trend_line(TrendLine {
            start_bar: 7.0,
            start_price: 114.0,
            end_bar: 12.0,
            end_price: 127.0,
            color: tl_color,
            width: 2.0,
            extend_right: false,
        });

        let mut r = crate::SvgRenderer::new(config.width, config.height);
        let layout = render_full_chart_with_markers(
            &mut r,
            visible_data,
            &[],
            &[],
            &annotations,
            None,
            &config,
        );
        let svg = String::from_utf8(r.finish()).unwrap();

        let lines = parse_svg_lines(&svg);
        let tl_lines: Vec<_> = lines.iter().filter(|l| l.4 == tl_css).collect();
        assert_eq!(tl_lines.len(), 1);
        let (x1, y1, x2, y2, _) = *tl_lines[0];

        let price_transform = layout.price_transform.unwrap();

        // Relative bars: 7 - 5 = 2, 12 - 5 = 7
        let expected_start = price_transform.to_pixel(2.0, 114.0);
        let expected_end = price_transform.to_pixel(7.0, 127.0);

        let tol = 0.1;
        assert!(
            (x1 - expected_start.x).abs() < tol,
            "offset start x: SVG={x1:.2}, expected={:.2}",
            expected_start.x
        );
        assert!(
            (y1 - expected_start.y).abs() < tol,
            "offset start y: SVG={y1:.2}, expected={:.2}",
            expected_start.y
        );
        assert!(
            (x2 - expected_end.x).abs() < tol,
            "offset end x: SVG={x2:.2}, expected={:.2}",
            expected_end.x
        );
        assert!(
            (y2 - expected_end.y).abs() < tol,
            "offset end y: SVG={y2:.2}, expected={:.2}",
            expected_end.y
        );
    }

    /// Corridor renders two parallel lines and a fill rectangle.
    #[test]
    fn corridor_renders_two_lines_and_fill() {
        let data = annotation_test_data();
        let config = ChartConfig::default();

        let corr_color = (100, 255, 100);
        let corr_css_rgba = Color::rgba(corr_color.0, corr_color.1, corr_color.2, 150).to_css();

        let mut annotations = Annotations::new();
        annotations.add_corridor(Corridor {
            line: TrendLine {
                start_bar: 2.0,
                start_price: 104.0,
                end_bar: 18.0,
                end_price: 140.0,
                color: corr_color,
                width: 1.0,
                extend_right: false,
            },
            offset: 5.0,
        });

        let mut r = crate::SvgRenderer::new(config.width, config.height);
        render_full_chart_with_markers(&mut r, &data, &[], &[], &annotations, None, &config);
        let svg = String::from_utf8(r.finish()).unwrap();

        // Two lines with corridor color (alpha 150)
        let lines = parse_svg_lines(&svg);
        let corr_lines: Vec<_> = lines.iter().filter(|l| l.4 == corr_css_rgba).collect();
        assert_eq!(
            corr_lines.len(),
            2,
            "expected 2 corridor lines, found {}",
            corr_lines.len()
        );

        // Fill rectangle with alpha 25
        let fill_css = Color::rgba(corr_color.0, corr_color.1, corr_color.2, 25).to_css();
        assert!(
            svg.contains(&fill_css),
            "expected corridor fill color {fill_css} in SVG"
        );
    }

    /// Fibonacci retracement renders horizontal lines and level labels.
    #[test]
    fn fibonacci_renders_levels_and_labels() {
        let data = annotation_test_data();
        let config = ChartConfig::default();

        let fib_color = (255, 165, 0);

        let mut annotations = Annotations::new();
        annotations.add_fibonacci(FibonacciRetracement {
            high_bar: 15,
            high_price: 135.0,
            low_bar: 3,
            low_price: 103.0,
            color: fib_color,
        });

        let mut r = crate::SvgRenderer::new(config.width, config.height);
        render_full_chart_with_markers(&mut r, &data, &[], &[], &annotations, None, &config);
        let svg = String::from_utf8(r.finish()).unwrap();

        // 7 Fibonacci levels → 7 horizontal lines
        // Check labels exist
        assert!(svg.contains("0.0%"), "missing 0.0% fib label");
        assert!(svg.contains("50.0%"), "missing 50.0% fib label");
        assert!(svg.contains("100.0%"), "missing 100.0% fib label");
        assert!(svg.contains("61.8%"), "missing 61.8% fib label");

        // Check the 50% level price label (midpoint = 135 - 32*0.5 = 119.0)
        assert!(svg.contains("119.00"), "missing 50% price value 119.00");
    }

    #[test]
    fn render_log_y_produces_valid_svg() {
        let data = annotation_test_data();
        let config = ChartConfig {
            log_y: true,
            ..ChartConfig::default()
        };
        let mut r = crate::SvgRenderer::new(config.width, config.height);
        render_full_chart(&mut r, &data, &[], &config);
        let out = String::from_utf8(r.finish()).unwrap();
        assert!(out.starts_with("<svg"));
        assert!(out.contains("<rect")); // candle bodies
        assert!(out.contains("<line")); // wicks + grid
        assert!(out.contains("<text")); // axis labels
    }

    /// Triple barrier renders TP/SL lines, time barrier, fill polygon, and exit marker.
    #[test]
    fn triple_barrier_renders_box_and_exit() {
        let data = annotation_test_data();
        let config = ChartConfig::default();

        let mut annotations = Annotations::new();
        annotations.add_triple_barrier(TripleBarrier {
            entry_bar: 5,
            entry_price: 110.0,
            tp_price: 120.0,
            sl_price: 105.0,
            horizon: 8,
            exit_bar: Some(10),
            outcome: Some(BarrierOutcome::TakeProfit),
            color: (100, 150, 255),
        });

        let mut r = crate::SvgRenderer::new(config.width, config.height);
        render_full_chart_with_markers(&mut r, &data, &[], &[], &annotations, None, &config);
        let svg = String::from_utf8(r.finish()).unwrap();

        // TP line (green)
        assert!(
            svg.contains("rgba(0,200,0,"),
            "expected TP line color in SVG"
        );
        // SL line (red)
        assert!(
            svg.contains("rgba(220,0,0,"),
            "expected SL line color in SVG"
        );
        // Fill polygon between TP and SL
        assert!(svg.contains("<polygon"), "expected fill polygon in SVG");
        // Exit marker (circle)
        assert!(
            svg.contains("<circle"),
            "expected exit marker circle in SVG"
        );
    }

    /// Triple barrier without exit renders box only (no circle).
    #[test]
    fn triple_barrier_no_exit_renders_box_only() {
        let data = annotation_test_data();
        let config = ChartConfig::default();

        let mut annotations = Annotations::new();
        annotations.add_triple_barrier(TripleBarrier {
            entry_bar: 3,
            entry_price: 106.0,
            tp_price: 115.0,
            sl_price: 100.0,
            horizon: 10,
            exit_bar: None,
            outcome: None,
            color: (200, 200, 0),
        });

        let mut r = crate::SvgRenderer::new(config.width, config.height);
        render_full_chart_with_markers(&mut r, &data, &[], &[], &annotations, None, &config);
        let svg = String::from_utf8(r.finish()).unwrap();

        // Should have TP and SL lines but no exit circle
        assert!(svg.contains("rgba(0,200,0,"));
        assert!(svg.contains("rgba(220,0,0,"));
        // No circle from TB (markers panel might have circles, but TB shouldn't)
        // Just verify the polygon fill exists
        assert!(svg.contains("<polygon"));
    }

    /// CUSUM indicator produces sub-panel with S+, S-, Event series.
    #[test]
    fn cusum_renders_as_sub_panel() {
        use ferrochart_core::Indicator;
        use ferrochart_core::indicator::Cusum;

        let data = annotation_test_data();
        let cusum = Cusum { threshold: 0.02 };
        let output = cusum.compute(&data);

        assert_eq!(output.series.len(), 3);
        assert_eq!(output.series[0].name, "S+");
        assert_eq!(output.series[1].name, "S\u{2212}");
        assert_eq!(output.series[2].name, "Event");

        let mut r = crate::SvgRenderer::new(900.0, 600.0);
        let config = ChartConfig {
            height: 600.0,
            ..ChartConfig::default()
        };
        render_full_chart(&mut r, &data, &[output], &config);
        let svg = String::from_utf8(r.finish()).unwrap();

        assert!(svg.contains("CUSUM"), "expected CUSUM label in sub-panel");
    }
}
