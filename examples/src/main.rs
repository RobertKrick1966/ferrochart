// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

//! Example binary that renders sample candlestick charts to SVG files.

use std::fs;

use ferrochart_core::{
    AndrewsPitchfork, Annotations, ChartType, Corridor, Ellipse, FibonacciRetracement, GannFan,
    HorizontalRay, Marker, MarkerPosition, MarkerShape, MeasurementTool, Ohlcv, PriceChannel, Ray,
    RectangleZone, TrendLine, VerticalLine,
    indicator::{Atr, Indicator, Rsi, Sma, Stochastic, VolumeSma},
};
use ferrochart_render::chart::{
    ChartConfig, render_candlestick_chart, render_full_chart, render_full_chart_with_markers,
    render_with_volume,
};
use ferrochart_render::{Renderer, SvgRenderer};

fn main() {
    let data = sample_ohlcv();

    fs::create_dir_all("output").expect("failed to create output dir");

    // 1. Basic candlestick chart
    generate_svg(
        "output/01_candlestick.svg",
        &data,
        |renderer, data, config| {
            render_candlestick_chart(renderer, data, config);
        },
    );

    // 2. Candlestick with volume panel
    generate_svg("output/02_volume.svg", &data, |renderer, data, config| {
        render_with_volume(renderer, data, config);
    });

    // 3. Split candle (institutional vs. retail)
    let split_data = sample_ohlcv_with_institutional();
    generate_svg(
        "output/03_split_candle.svg",
        &split_data,
        |renderer, data, config| {
            render_candlestick_chart(renderer, data, config);
        },
    );

    // 4. SMA overlay indicator
    generate_svg(
        "output/04_sma_overlay.svg",
        &data,
        |renderer, data, config| {
            let sma = Sma { period: 7 };
            let output = sma.compute(data);
            render_full_chart(renderer, data, &[output], config);
        },
    );

    // 5. Volume SMA sub-panel indicator
    generate_svg(
        "output/05_volume_sma.svg",
        &data,
        |renderer, data, config| {
            let vol_sma = VolumeSma { period: 5 };
            let output = vol_sma.compute(data);
            render_full_chart(renderer, data, &[output], config);
        },
    );

    // 6. Circle markers (balls above/below candles)
    generate_svg("output/06_markers.svg", &data, |renderer, data, config| {
        let markers = sample_markers();
        let marker_refs: Vec<&Marker> = markers.iter().collect();
        render_full_chart_with_markers(
            renderer,
            data,
            &[],
            &marker_refs,
            &Annotations::default(),
            None,
            config,
        );
    });

    // 7. Annotations: trendlines, corridor, Fibonacci retracement
    generate_svg(
        "output/07_annotations.svg",
        &data,
        |renderer, data, config| {
            let mut annotations = Annotations::new();

            // Rising trendline (yellow)
            annotations.add_trend_line(TrendLine {
                start_bar: 2.0,
                start_price: data[2].low,
                end_bar: 20.0,
                end_price: data[20].low,
                color: (255, 235, 59),
                width: 2.0,
                extend_right: true,
            });

            // Falling trendline (red, not extended)
            annotations.add_trend_line(TrendLine {
                start_bar: 6.0,
                start_price: data[6].high,
                end_bar: 19.0,
                end_price: data[19].high,
                color: (255, 60, 60),
                width: 1.5,
                extend_right: false,
            });

            // Corridor / channel (cyan)
            annotations.add_corridor(Corridor {
                line: TrendLine {
                    start_bar: 10.0,
                    start_price: data[10].low,
                    end_bar: 25.0,
                    end_price: data[25].low,
                    color: (0, 200, 255),
                    width: 1.0,
                    extend_right: false,
                },
                offset: 8.0,
            });

            // Fibonacci retracement (orange)
            let high_bar = 19;
            let low_bar = 9;
            annotations.add_fibonacci(FibonacciRetracement {
                high_bar,
                high_price: data[high_bar].high,
                low_bar,
                low_price: data[low_bar].low,
                color: (255, 165, 0),
            });

            let marker_refs: Vec<&Marker> = vec![];
            render_full_chart_with_markers(
                renderer,
                data,
                &[],
                &marker_refs,
                &annotations,
                None,
                config,
            );
        },
    );

    // 8. Heikin-Ashi chart
    generate_svg(
        "output/08_heikin_ashi.svg",
        &data,
        |renderer, data, config| {
            let mut cfg = config.clone();
            cfg.chart_type = ChartType::HeikinAshi;
            render_full_chart_with_markers(
                renderer,
                data,
                &[],
                &[],
                &Annotations::default(),
                None,
                &cfg,
            );
        },
    );

    // 9. Line chart
    generate_svg(
        "output/09_line_chart.svg",
        &data,
        |renderer, data, config| {
            let mut cfg = config.clone();
            cfg.chart_type = ChartType::Line;
            render_full_chart_with_markers(
                renderer,
                data,
                &[],
                &[],
                &Annotations::default(),
                None,
                &cfg,
            );
        },
    );

    // 10. Area chart
    generate_svg(
        "output/10_area_chart.svg",
        &data,
        |renderer, data, config| {
            let mut cfg = config.clone();
            cfg.chart_type = ChartType::Area;
            render_full_chart_with_markers(
                renderer,
                data,
                &[],
                &[],
                &Annotations::default(),
                None,
                &cfg,
            );
        },
    );

    // 11. OHLC Bars
    generate_svg(
        "output/11_ohlc_bars.svg",
        &data,
        |renderer, data, config| {
            let mut cfg = config.clone();
            cfg.chart_type = ChartType::OhlcBars;
            render_full_chart_with_markers(
                renderer,
                data,
                &[],
                &[],
                &Annotations::default(),
                None,
                &cfg,
            );
        },
    );

    // 12. ATR indicator (sub-panel)
    generate_svg("output/12_atr.svg", &data, |renderer, data, config| {
        let atr = Atr { period: 14 };
        let output = atr.compute(data);
        render_full_chart(renderer, data, &[output], config);
    });

    // 13. RSI indicator (sub-panel)
    generate_svg("output/13_rsi.svg", &data, |renderer, data, config| {
        let rsi = Rsi { period: 14 };
        let output = rsi.compute(data);
        render_full_chart(renderer, data, &[output], config);
    });

    // 14. Stochastic indicator (sub-panel)
    generate_svg(
        "output/14_stochastic.svg",
        &data,
        |renderer, data, config| {
            let stoch = Stochastic {
                k_period: 14,
                d_period: 3,
            };
            let output = stoch.compute(data);
            render_full_chart(renderer, data, &[output], config);
        },
    );

    // 15. Drawing tools: HorizontalRay, VerticalLine, RectangleZone
    generate_svg(
        "output/15_drawing_tools.svg",
        &data,
        |renderer, data, config| {
            let mut annotations = Annotations::new();
            annotations.add_horizontal_ray(HorizontalRay {
                price: data[15].high,
                color: (255, 200, 0),
                width: 1.5,
            });
            annotations.add_vertical_line(VerticalLine {
                bar_index: 10.0,
                color: (100, 200, 255),
                width: 1.0,
            });
            annotations.add_rectangle_zone(RectangleZone {
                start_bar: 5.0,
                end_bar: 12.0,
                top_price: data[8].high,
                bottom_price: data[8].low,
                border_color: (255, 100, 100),
                fill_color: (255, 100, 100, 30),
                width: 1.0,
            });
            render_full_chart_with_markers(renderer, data, &[], &[], &annotations, None, config);
        },
    );

    // 16. Renko chart
    generate_svg("output/16_renko.svg", &data, |renderer, data, config| {
        let mut cfg = config.clone();
        cfg.chart_type = ChartType::Renko { brick_size: 2.0 };
        render_full_chart_with_markers(
            renderer,
            data,
            &[],
            &[],
            &Annotations::default(),
            None,
            &cfg,
        );
    });

    // 17. Point & Figure chart
    generate_svg(
        "output/17_point_figure.svg",
        &data,
        |renderer, data, config| {
            let mut cfg = config.clone();
            cfg.chart_type = ChartType::PointFigure {
                box_size: 1.5,
                reversal: 3,
            };
            render_full_chart_with_markers(
                renderer,
                data,
                &[],
                &[],
                &Annotations::default(),
                None,
                &cfg,
            );
        },
    );

    // 18. Ray
    generate_svg("output/18_ray.svg", &data, |renderer, data, config| {
        let mut annotations = Annotations::new();
        annotations.add_ray(Ray {
            start_bar: 5.0,
            start_price: data[5].low,
            end_bar: 15.0,
            end_price: data[15].low,
            color: (0, 200, 255),
            width: 1.5,
        });
        render_full_chart_with_markers(renderer, data, &[], &[], &annotations, None, config);
    });

    // 19. Measurement Tool
    generate_svg(
        "output/19_measurement.svg",
        &data,
        |renderer, data, config| {
            let mut annotations = Annotations::new();
            annotations.add_measurement(MeasurementTool {
                start_bar: 5.0,
                start_price: data[5].close,
                end_bar: 20.0,
                end_price: data[20].close,
                color: (255, 200, 0),
            });
            render_full_chart_with_markers(renderer, data, &[], &[], &annotations, None, config);
        },
    );

    // 20. Ellipse
    generate_svg("output/20_ellipse.svg", &data, |renderer, data, config| {
        let mut annotations = Annotations::new();
        annotations.add_ellipse(Ellipse {
            start_bar: 8.0,
            start_price: data[8].low,
            end_bar: 16.0,
            end_price: data[16].high,
            color: (100, 200, 100),
            fill_color: (100, 200, 100, 25),
            width: 1.5,
        });
        render_full_chart_with_markers(renderer, data, &[], &[], &annotations, None, config);
    });

    // 21. Andrews Pitchfork
    generate_svg(
        "output/21_pitchfork.svg",
        &data,
        |renderer, data, config| {
            let mut annotations = Annotations::new();
            annotations.add_pitchfork(AndrewsPitchfork {
                bar1: 2.0,
                price1: data[2].low,
                bar2: 10.0,
                price2: data[10].high,
                bar3: 18.0,
                price3: data[18].low,
                color: (255, 165, 0),
                width: 1.5,
            });
            render_full_chart_with_markers(renderer, data, &[], &[], &annotations, None, config);
        },
    );

    // 22. Gann Fan
    generate_svg("output/22_gann_fan.svg", &data, |renderer, data, config| {
        let mut annotations = Annotations::new();
        annotations.add_gann_fan(GannFan {
            anchor_bar: 3.0,
            anchor_price: data[3].low,
            scale: 1.5,
            color: (200, 100, 255),
        });
        render_full_chart_with_markers(renderer, data, &[], &[], &annotations, None, config);
    });

    // 23. Price Channel
    generate_svg(
        "output/23_price_channel.svg",
        &data,
        |renderer, data, config| {
            let mut annotations = Annotations::new();
            annotations.add_price_channel(PriceChannel {
                start_bar: 3.0,
                end_bar: 25.0,
                upper_start_price: data[3].high,
                upper_end_price: data[25].high,
                lower_start_price: data[3].low,
                lower_end_price: data[25].low,
                color: (0, 200, 255),
                fill_color: (0, 200, 255, 20),
                width: 1.5,
            });
            render_full_chart_with_markers(renderer, data, &[], &[], &annotations, None, config);
        },
    );

    println!("All SVGs written to output/");
}

fn generate_svg(
    path: &str,
    data: &[Ohlcv],
    render_fn: impl FnOnce(&mut dyn Renderer, &[Ohlcv], &ChartConfig),
) {
    let config = ChartConfig::default();
    let mut renderer = SvgRenderer::new(config.width, config.height);
    render_fn(&mut renderer, data, &config);
    let svg = renderer.finish();
    fs::write(path, &svg).unwrap_or_else(|e| panic!("failed to write {path}: {e}"));
    println!("Wrote {path} ({} bytes)", svg.len());
}

/// Generate realistic-looking OHLCV sample data (30 daily bars).
fn sample_ohlcv() -> Vec<Ohlcv> {
    let base_timestamp: i64 = 1_700_000_000; // ~2023-11-14
    let day: i64 = 86_400;
    let mut price = 100.0_f64;
    let mut data = Vec::with_capacity(30);

    let moves = [
        2.5, -1.2, 3.8, -0.5, 1.7, -2.3, 4.1, -1.8, 0.9, -3.2, 2.1, 1.5, -0.8, 3.3, -2.7, 1.9,
        -1.1, 2.6, -0.3, 4.5, -3.1, 1.4, 2.2, -1.6, 0.7, -2.0, 3.0, -0.9, 1.8, -1.4,
    ];

    for (i, &mv) in moves.iter().enumerate() {
        let open = price;
        let close = open + mv;
        let high = open.max(close) + (mv.abs() * 0.5);
        let low = open.min(close) - (mv.abs() * 0.3);
        let volume = 5000.0 + (mv.abs() * 1000.0) + (i as f64 * 100.0);

        data.push(Ohlcv {
            timestamp: base_timestamp + i as i64 * day,
            open,
            high,
            low,
            close,
            volume,
            institutional_ratio: 0.0,
        });

        price = close;
    }

    data
}

/// Sample markers demonstrating all marker shapes and positions.
fn sample_markers() -> Vec<Marker> {
    vec![
        // Green balls above bar (e.g., CUSUM event detected)
        Marker {
            bar_index: 6,
            shape: MarkerShape::Circle,
            position: MarkerPosition::AboveBar,
            color: (0, 200, 0, 255),
            label: String::new(),
        },
        Marker {
            bar_index: 12,
            shape: MarkerShape::Circle,
            position: MarkerPosition::AboveBar,
            color: (0, 200, 0, 255),
            label: String::new(),
        },
        Marker {
            bar_index: 19,
            shape: MarkerShape::Circle,
            position: MarkerPosition::AboveBar,
            color: (0, 200, 0, 255),
            label: "CUSUM".to_string(),
        },
        // Red balls below bar
        Marker {
            bar_index: 3,
            shape: MarkerShape::Circle,
            position: MarkerPosition::BelowBar,
            color: (220, 0, 0, 255),
            label: String::new(),
        },
        Marker {
            bar_index: 9,
            shape: MarkerShape::Circle,
            position: MarkerPosition::BelowBar,
            color: (220, 0, 0, 255),
            label: String::new(),
        },
        Marker {
            bar_index: 15,
            shape: MarkerShape::Circle,
            position: MarkerPosition::BelowBar,
            color: (220, 0, 0, 255),
            label: "Alert".to_string(),
        },
        // Other marker shapes for comparison
        Marker {
            bar_index: 22,
            shape: MarkerShape::ArrowUp,
            position: MarkerPosition::BelowBar,
            color: (0, 200, 0, 255),
            label: String::new(),
        },
        Marker {
            bar_index: 25,
            shape: MarkerShape::ArrowDown,
            position: MarkerPosition::AboveBar,
            color: (220, 0, 0, 255),
            label: String::new(),
        },
        Marker {
            bar_index: 28,
            shape: MarkerShape::Diamond,
            position: MarkerPosition::AboveBar,
            color: (255, 200, 0, 255),
            label: String::new(),
        },
    ]
}

/// OHLCV data with institutional activity for split-candle rendering.
fn sample_ohlcv_with_institutional() -> Vec<Ohlcv> {
    let mut data = sample_ohlcv();
    // Add institutional ratios to some bars
    let ratios = [
        0.0, 0.0, 0.3, 0.0, 0.0, 0.6, 0.0, 0.45, 0.0, 0.0, 0.7, 0.0, 0.0, 0.5, 0.0, 0.0, 0.35, 0.0,
        0.0, 0.8, 0.0, 0.0, 0.55, 0.0, 0.0, 0.4, 0.0, 0.0, 0.65, 0.0,
    ];
    for (bar, &ratio) in data.iter_mut().zip(ratios.iter()) {
        bar.institutional_ratio = ratio;
    }
    data
}
