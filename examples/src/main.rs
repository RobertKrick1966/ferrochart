// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use std::fs;

use ferrochart_core::Ohlcv;
use ferrochart_render::chart::{ChartConfig, render_candlestick_chart, render_with_volume};
use ferrochart_render::{Renderer, SvgRenderer};

fn main() {
    let data = sample_ohlcv();

    // Simple candlestick chart
    let config = ChartConfig::default();
    let mut renderer = SvgRenderer::new(config.width, config.height);
    render_candlestick_chart(&mut renderer, &data, &config);

    let svg = renderer.finish();
    fs::write("candlestick.svg", &svg).expect("failed to write candlestick.svg");
    println!("Wrote candlestick.svg ({} bytes)", svg.len());

    // Chart with volume panel
    let mut renderer2 = SvgRenderer::new(config.width, config.height);
    render_with_volume(&mut renderer2, &data, &config);

    let svg2 = renderer2.finish();
    fs::write("candlestick_volume.svg", &svg2).expect("failed to write candlestick_volume.svg");
    println!("Wrote candlestick_volume.svg ({} bytes)", svg2.len());
}

/// Generate realistic-looking OHLCV sample data (30 daily bars).
#[allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]
fn sample_ohlcv() -> Vec<Ohlcv> {
    let base_timestamp: i64 = 1_700_000_000; // ~2023-11-14
    let day: i64 = 86_400;
    let mut price = 100.0_f64;
    let mut data = Vec::with_capacity(30);

    // Deterministic pseudo-random walk
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
        });

        price = close;
    }

    data
}
