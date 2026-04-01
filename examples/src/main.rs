// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use std::fs;

use ferrochart_core::{
    Annotations, Marker, MarkerPosition, MarkerShape, Ohlcv,
    indicator::{Indicator, Sma, VolumeSma},
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
    generate_svg("output/01_candlestick.svg", &data, |renderer, data, config| {
        render_candlestick_chart(renderer, data, config);
    });

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
    generate_svg("output/04_sma_overlay.svg", &data, |renderer, data, config| {
        let sma = Sma { period: 7 };
        let output = sma.compute(data);
        render_full_chart(renderer, data, &[output], config);
    });

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
    generate_svg(
        "output/06_markers.svg",
        &data,
        |renderer, data, config| {
            let markers = vec![
                // Green balls above bar (e.g., CUSUM event detected)
                Marker { bar_index: 6, shape: MarkerShape::Circle, position: MarkerPosition::AboveBar, color: (0, 200, 0, 255), label: String::new() },
                Marker { bar_index: 12, shape: MarkerShape::Circle, position: MarkerPosition::AboveBar, color: (0, 200, 0, 255), label: String::new() },
                Marker { bar_index: 19, shape: MarkerShape::Circle, position: MarkerPosition::AboveBar, color: (0, 200, 0, 255), label: "CUSUM".to_string() },
                // Red balls below bar
                Marker { bar_index: 3, shape: MarkerShape::Circle, position: MarkerPosition::BelowBar, color: (220, 0, 0, 255), label: String::new() },
                Marker { bar_index: 9, shape: MarkerShape::Circle, position: MarkerPosition::BelowBar, color: (220, 0, 0, 255), label: String::new() },
                Marker { bar_index: 15, shape: MarkerShape::Circle, position: MarkerPosition::BelowBar, color: (220, 0, 0, 255), label: "Alert".to_string() },
                // Other marker shapes for comparison
                Marker { bar_index: 22, shape: MarkerShape::ArrowUp, position: MarkerPosition::BelowBar, color: (0, 200, 0, 255), label: String::new() },
                Marker { bar_index: 25, shape: MarkerShape::ArrowDown, position: MarkerPosition::AboveBar, color: (220, 0, 0, 255), label: String::new() },
                Marker { bar_index: 28, shape: MarkerShape::Diamond, position: MarkerPosition::AboveBar, color: (255, 200, 0, 255), label: String::new() },
            ];
            let marker_refs: Vec<&Marker> = markers.iter().collect();
            render_full_chart_with_markers(
                renderer,
                data,
                &[],
                &marker_refs,
                &Annotations::default(),
                config,
            );
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
#[allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]
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

/// OHLCV data with institutional activity for split-candle rendering.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]
fn sample_ohlcv_with_institutional() -> Vec<Ohlcv> {
    let mut data = sample_ohlcv();
    // Add institutional ratios to some bars
    let ratios = [
        0.0, 0.0, 0.3, 0.0, 0.0, 0.6, 0.0, 0.45, 0.0, 0.0, 0.7, 0.0, 0.0, 0.5, 0.0, 0.0, 0.35,
        0.0, 0.0, 0.8, 0.0, 0.0, 0.55, 0.0, 0.0, 0.4, 0.0, 0.0, 0.65, 0.0,
    ];
    for (bar, &ratio) in data.iter_mut().zip(ratios.iter()) {
        bar.institutional_ratio = ratio;
    }
    data
}
