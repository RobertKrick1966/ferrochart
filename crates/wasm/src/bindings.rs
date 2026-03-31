// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use ferrochart_core::Ohlcv;
use ferrochart_render::chart::{ChartConfig, render_with_volume};

use crate::CanvasRenderer;

/// Render a candlestick chart with volume panel onto a canvas element.
///
/// `timestamps`, `opens`, `highs`, `lows`, `closes`, `volumes` are parallel arrays
/// of equal length representing the OHLCV data.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn render_chart(
    canvas: &HtmlCanvasElement,
    timestamps: &[f64],
    opens: &[f64],
    highs: &[f64],
    lows: &[f64],
    closes: &[f64],
    volumes: &[f64],
) -> Result<(), JsValue> {
    let len = timestamps.len();
    if len == 0 {
        return Ok(());
    }
    if opens.len() != len
        || highs.len() != len
        || lows.len() != len
        || closes.len() != len
        || volumes.len() != len
    {
        return Err(JsValue::from_str("all arrays must have equal length"));
    }

    let data: Vec<Ohlcv> = (0..len)
        .map(|i| Ohlcv {
            #[allow(clippy::cast_possible_truncation)]
            timestamp: timestamps[i] as i64,
            open: opens[i],
            high: highs[i],
            low: lows[i],
            close: closes[i],
            volume: volumes[i],
            institutional_ratio: 0.0,
        })
        .collect();

    let config = ChartConfig {
        width: f64::from(canvas.width()),
        height: f64::from(canvas.height()),
        ..ChartConfig::default()
    };

    let mut renderer = CanvasRenderer::new(canvas)?;
    render_with_volume(&mut renderer, &data, &config);

    Ok(())
}
