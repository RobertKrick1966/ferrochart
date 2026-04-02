// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

//! C-compatible FFI bindings for `FerroChart`.
//!
//! This crate exposes `FerroChart` functionality through an opaque handle and
//! `extern "C"` functions, making it usable from C, C++, Python (ctypes),
//! C# (P/Invoke), and any language with C FFI support.
//!
//! # Safety
//!
//! All functions that accept a `*mut FcChart` or `*const FcChart` require a
//! valid, non-null pointer obtained from [`fc_chart_create`]. Passing null or
//! dangling pointers is undefined behavior.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::slice;

use ferrochart_core::annotation::Annotations;
use ferrochart_core::indicator::{self, Indicator, IndicatorOutput};
use ferrochart_core::marker::{Marker, MarkerPosition, MarkerShape};
use ferrochart_core::{ChartType, Ohlcv};
use ferrochart_render::chart::{ChartConfig, render_full_chart_with_markers};
use ferrochart_render::style::Color;
use ferrochart_render::{Renderer as _, SvgRenderer};

// ---------------------------------------------------------------------------
// Opaque handle
// ---------------------------------------------------------------------------

/// Opaque chart handle exposed to C.
///
/// Callers must treat this as a pointer-sized opaque type.
/// Create with [`fc_chart_create`], destroy with [`fc_chart_destroy`].
pub struct FcChart {
    data: Vec<Ohlcv>,
    indicators: Vec<IndicatorOutput>,
    indicator_specs: Vec<IndicatorSpec>,
    markers: Vec<Marker>,
    annotations: Annotations,
    config: ChartConfig,
}

/// Internal record so we can recompute indicators when data changes.
struct IndicatorSpec {
    name: String,
    period: usize,
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/// Create a new chart handle with default configuration.
///
/// Returns a heap-allocated handle. The caller owns this handle and **must**
/// call [`fc_chart_destroy`] to free it.
#[unsafe(no_mangle)]
pub extern "C" fn fc_chart_create() -> *mut FcChart {
    let chart = Box::new(FcChart {
        data: Vec::new(),
        indicators: Vec::new(),
        indicator_specs: Vec::new(),
        markers: Vec::new(),
        annotations: Annotations::default(),
        config: ChartConfig::default(),
    });
    Box::into_raw(chart)
}

/// Destroy a chart handle and free its memory.
///
/// # Safety
///
/// `handle` must be a valid pointer returned by [`fc_chart_create`], or null.
/// After this call the pointer is dangling and must not be used again.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_destroy(handle: *mut FcChart) {
    if !handle.is_null() {
        drop(unsafe { Box::from_raw(handle) });
    }
}

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

/// Set OHLCV data from parallel C arrays.
///
/// All arrays must have exactly `len` elements.
///
/// # Safety
///
/// All pointers must be valid and point to arrays of at least `len` elements.
/// `handle` must be a valid `FcChart` pointer.
#[unsafe(no_mangle)]
#[allow(clippy::many_single_char_names)]
pub unsafe extern "C" fn fc_chart_set_data(
    handle: *mut FcChart,
    timestamps: *const i64,
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    volume: *const f64,
    len: usize,
) {
    let chart = unsafe { &mut *handle };
    let ts = unsafe { slice::from_raw_parts(timestamps, len) };
    let (o, h, l, c, v) = unsafe {
        (
            slice::from_raw_parts(open, len),
            slice::from_raw_parts(high, len),
            slice::from_raw_parts(low, len),
            slice::from_raw_parts(close, len),
            slice::from_raw_parts(volume, len),
        )
    };

    chart.data = (0..len)
        .map(|i| Ohlcv {
            timestamp: ts[i],
            open: o[i],
            high: h[i],
            low: l[i],
            close: c[i],
            volume: v[i],
            institutional_ratio: 0.0,
        })
        .collect();

    recompute_indicators(chart);
}

/// Set OHLCV data from a JSON string.
///
/// Expected format: `[{"timestamp":…,"open":…,"high":…,"low":…,"close":…,"volume":…}, …]`
///
/// Returns 0 on success, -1 on parse error.
///
/// # Safety
///
/// `handle` must be valid. `json` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_set_data_json(handle: *mut FcChart, json: *const c_char) -> i32 {
    let chart = unsafe { &mut *handle };
    let c_str = unsafe { CStr::from_ptr(json) };
    let Ok(s) = c_str.to_str() else { return -1 };
    match serde_json::from_str::<Vec<Ohlcv>>(s) {
        Ok(bars) => {
            chart.data = bars;
            recompute_indicators(chart);
            0
        }
        Err(_) => -1,
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Set chart dimensions in pixels.
///
/// # Safety
///
/// `handle` must be a valid `FcChart` pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_set_size(handle: *mut FcChart, width: f64, height: f64) {
    let chart = unsafe { &mut *handle };
    chart.config.width = width;
    chart.config.height = height;
}

/// Set the chart type.
///
/// Supported values: `"candlestick"`, `"heikin_ashi"`, `"ohlc"`, `"line"`,
/// `"area"`, `"renko"`, `"point_figure"`.
///
/// Returns 0 on success, -1 on unknown type.
///
/// # Safety
///
/// `handle` must be valid. `name` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_set_type(handle: *mut FcChart, name: *const c_char) -> i32 {
    let chart = unsafe { &mut *handle };
    let c_str = unsafe { CStr::from_ptr(name) };
    let Ok(s) = c_str.to_str() else { return -1 };
    match s {
        "candlestick" => chart.config.chart_type = ChartType::Candlestick,
        "heikin_ashi" => chart.config.chart_type = ChartType::HeikinAshi,
        "ohlc" => chart.config.chart_type = ChartType::OhlcBars,
        "line" => chart.config.chart_type = ChartType::Line,
        "area" => chart.config.chart_type = ChartType::Area,
        "renko" => chart.config.chart_type = ChartType::Renko { brick_size: 1.0 },
        "point_figure" => {
            chart.config.chart_type = ChartType::PointFigure {
                box_size: 1.0,
                reversal: 3,
            };
        }
        _ => return -1,
    }
    0
}

/// Set Renko brick size. Only effective when chart type is `"renko"`.
///
/// # Safety
///
/// `handle` must be a valid `FcChart` pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_set_renko_brick_size(handle: *mut FcChart, brick_size: f64) {
    let chart = unsafe { &mut *handle };
    chart.config.chart_type = ChartType::Renko { brick_size };
}

/// Set Point & Figure parameters.
///
/// # Safety
///
/// `handle` must be a valid `FcChart` pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_set_pf_config(
    handle: *mut FcChart,
    box_size: f64,
    reversal: u32,
) {
    let chart = unsafe { &mut *handle };
    chart.config.chart_type = ChartType::PointFigure {
        box_size,
        reversal: reversal as usize,
    };
}

/// Apply dark theme colors.
///
/// # Safety
///
/// `handle` must be a valid `FcChart` pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_set_theme_dark(handle: *mut FcChart) {
    let chart = unsafe { &mut *handle };
    chart.config.background = Color::rgba(17, 17, 17, 255);
    chart.config.bullish_color = Color::rgba(38, 166, 91, 255);
    chart.config.bearish_color = Color::rgba(239, 67, 82, 255);
    chart.config.wick_color = Color::rgba(180, 180, 180, 255);
    chart.config.axis_color = Color::rgba(60, 60, 60, 255);
    chart.config.grid_color = Color::rgba(40, 40, 40, 255);
    chart.config.text_color = Color::rgba(180, 180, 180, 255);
}

/// Apply light theme colors.
///
/// # Safety
///
/// `handle` must be a valid `FcChart` pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_set_theme_light(handle: *mut FcChart) {
    let chart = unsafe { &mut *handle };
    chart.config.background = Color::rgba(255, 255, 255, 255);
    chart.config.bullish_color = Color::rgba(38, 166, 91, 255);
    chart.config.bearish_color = Color::rgba(239, 67, 82, 255);
    chart.config.wick_color = Color::rgba(100, 100, 100, 255);
    chart.config.axis_color = Color::rgba(200, 200, 200, 255);
    chart.config.grid_color = Color::rgba(235, 235, 235, 255);
    chart.config.text_color = Color::rgba(60, 60, 60, 255);
}

/// Enable or disable logarithmic Y-axis.
///
/// # Safety
///
/// `handle` must be a valid `FcChart` pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_set_log_scale(handle: *mut FcChart, enabled: bool) {
    let chart = unsafe { &mut *handle };
    chart.config.log_y = enabled;
}

/// Apply a full JSON configuration.
///
/// Returns 0 on success, -1 on parse error.
///
/// # Safety
///
/// `handle` must be valid. `json` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_set_config_json(
    handle: *mut FcChart,
    json: *const c_char,
) -> i32 {
    let chart = unsafe { &mut *handle };
    let c_str = unsafe { CStr::from_ptr(json) };
    let Ok(s) = c_str.to_str() else { return -1 };
    match serde_json::from_str::<ChartConfig>(s) {
        Ok(cfg) => {
            chart.config = cfg;
            0
        }
        Err(_) => -1,
    }
}

// ---------------------------------------------------------------------------
// Indicators
// ---------------------------------------------------------------------------

/// Add a technical indicator by name.
///
/// Supported names: `"sma"`, `"ema"`, `"bollinger"`, `"rsi"`, `"macd"`,
/// `"atr"`, `"obv"`, `"stochastic"`, `"williams_r"`, `"cci"`, `"adx"`,
/// `"donchian"`, `"keltner"`, `"parabolic_sar"`, `"supertrend"`,
/// `"ichimoku"`, `"session_vwap"`, `"volume_sma"`.
///
/// `period` is the look-back window (ignored for indicators with fixed params).
///
/// Returns 0 on success, -1 on unknown indicator name.
///
/// # Safety
///
/// `handle` must be valid. `name` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_add_indicator(
    handle: *mut FcChart,
    name: *const c_char,
    period: usize,
) -> i32 {
    let chart = unsafe { &mut *handle };
    let c_str = unsafe { CStr::from_ptr(name) };
    let Ok(s) = c_str.to_str() else { return -1 };

    if let Some(out) = compute_indicator(s, period, &chart.data) {
        chart.indicators.push(out);
        chart.indicator_specs.push(IndicatorSpec {
            name: s.to_string(),
            period,
        });
        0
    } else {
        -1
    }
}

/// Remove all indicators.
///
/// # Safety
///
/// `handle` must be a valid `FcChart` pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_clear_indicators(handle: *mut FcChart) {
    let chart = unsafe { &mut *handle };
    chart.indicators.clear();
    chart.indicator_specs.clear();
}

// ---------------------------------------------------------------------------
// Markers
// ---------------------------------------------------------------------------

/// Add a marker (buy/sell signal, label) at a specific bar.
///
/// `shape`: 0 = arrow up, 1 = arrow down, 2 = circle, 3 = diamond.
/// `position`: 0 = above bar, 1 = below bar.
///
/// # Safety
///
/// `handle` must be valid. `label` must be a valid null-terminated UTF-8 string
/// (or null for no label).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_add_marker(
    handle: *mut FcChart,
    bar_index: usize,
    shape: u32,
    position: u32,
    r: u8,
    g: u8,
    b: u8,
    label: *const c_char,
) {
    let chart = unsafe { &mut *handle };
    let marker_shape = match shape {
        0 => MarkerShape::ArrowUp,
        1 => MarkerShape::ArrowDown,
        2 => MarkerShape::Circle,
        _ => MarkerShape::Diamond,
    };
    let marker_pos = match position {
        0 => MarkerPosition::AboveBar,
        _ => MarkerPosition::BelowBar,
    };
    let label_str = if label.is_null() {
        String::new()
    } else {
        let c_str = unsafe { CStr::from_ptr(label) };
        c_str.to_str().unwrap_or("").to_string()
    };
    chart.markers.push(Marker {
        bar_index,
        shape: marker_shape,
        position: marker_pos,
        color: (r, g, b, 255),
        label: label_str,
    });
}

/// Remove all markers.
///
/// # Safety
///
/// `handle` must be a valid `FcChart` pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_clear_markers(handle: *mut FcChart) {
    let chart = unsafe { &mut *handle };
    chart.markers.clear();
}

// ---------------------------------------------------------------------------
// Annotations (JSON-based)
// ---------------------------------------------------------------------------

/// Import annotations from a JSON string.
///
/// Returns 0 on success, -1 on parse error.
///
/// # Safety
///
/// `handle` must be valid. `json` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_import_annotations(
    handle: *mut FcChart,
    json: *const c_char,
) -> i32 {
    let chart = unsafe { &mut *handle };
    let c_str = unsafe { CStr::from_ptr(json) };
    let Ok(s) = c_str.to_str() else { return -1 };
    match serde_json::from_str::<Annotations>(s) {
        Ok(ann) => {
            chart.annotations = ann;
            0
        }
        Err(_) => -1,
    }
}

/// Export annotations as a JSON string.
///
/// Returns a heap-allocated null-terminated UTF-8 string. The caller must free
/// it with [`fc_string_free`].
///
/// # Safety
///
/// `handle` must be a valid `FcChart` pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_export_annotations(handle: *const FcChart) -> *mut c_char {
    let chart = unsafe { &*handle };
    let json = serde_json::to_string(&chart.annotations).unwrap_or_default();
    string_to_c(json)
}

/// Remove all annotations.
///
/// # Safety
///
/// `handle` must be a valid `FcChart` pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_clear_annotations(handle: *mut FcChart) {
    let chart = unsafe { &mut *handle };
    chart.annotations = Annotations::default();
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// Render the chart to an SVG string.
///
/// Returns a heap-allocated null-terminated UTF-8 string containing the SVG.
/// The caller must free it with [`fc_string_free`].
///
/// Returns null if there is no data set.
///
/// # Safety
///
/// `handle` must be a valid `FcChart` pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_render_svg(handle: *const FcChart) -> *mut c_char {
    let chart = unsafe { &*handle };
    if chart.data.is_empty() {
        return ptr::null_mut();
    }

    let mut renderer = SvgRenderer::new(chart.config.width, chart.config.height);
    let marker_refs: Vec<&Marker> = chart.markers.iter().collect();

    render_full_chart_with_markers(
        &mut renderer,
        &chart.data,
        &chart.indicators,
        &marker_refs,
        &chart.annotations,
        None,
        &chart.config,
    );

    let bytes = renderer.finish();
    let svg = String::from_utf8_lossy(&bytes).into_owned();
    string_to_c(svg)
}

/// Return the number of OHLCV bars currently loaded.
///
/// # Safety
///
/// `handle` must be a valid `FcChart` pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_chart_bar_count(handle: *const FcChart) -> usize {
    let chart = unsafe { &*handle };
    chart.data.len()
}

// ---------------------------------------------------------------------------
// String management
// ---------------------------------------------------------------------------

/// Free a string previously returned by this library.
///
/// # Safety
///
/// `s` must be a pointer returned by an `fc_*` function that returns
/// `*mut c_char`, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fc_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(unsafe { CString::from_raw(s) });
    }
}

// ---------------------------------------------------------------------------
// Version
// ---------------------------------------------------------------------------

/// Return the library version as a static string.
///
/// The returned pointer is valid for the lifetime of the library and must NOT
/// be freed by the caller.
#[unsafe(no_mangle)]
pub extern "C" fn fc_version() -> *const c_char {
    c"0.1.0".as_ptr()
}

// ===========================================================================
// Internal helpers
// ===========================================================================

fn string_to_c(s: String) -> *mut c_char {
    CString::new(s).map_or(ptr::null_mut(), CString::into_raw)
}

/// Recompute all indicator outputs after data changes.
fn recompute_indicators(chart: &mut FcChart) {
    chart.indicators.clear();
    for spec in &chart.indicator_specs {
        if let Some(out) = compute_indicator(&spec.name, spec.period, &chart.data) {
            chart.indicators.push(out);
        }
    }
}

fn compute_indicator(name: &str, period: usize, data: &[Ohlcv]) -> Option<IndicatorOutput> {
    match name.to_lowercase().as_str() {
        "sma" => Some(indicator::Sma { period }.compute(data)),
        "ema" => Some(indicator::Ema { period }.compute(data)),
        "bollinger" => Some(
            (indicator::BollingerBands {
                period,
                std_dev: 2.0,
            })
            .compute(data),
        ),
        "rsi" => Some(indicator::Rsi { period }.compute(data)),
        "macd" => Some(
            (indicator::Macd {
                fast_period: 12,
                slow_period: 26,
                signal_period: 9,
            })
            .compute(data),
        ),
        "atr" => Some(indicator::Atr { period }.compute(data)),
        "obv" => Some(indicator::Obv.compute(data)),
        "stochastic" => Some(
            (indicator::Stochastic {
                k_period: period,
                d_period: 3,
            })
            .compute(data),
        ),
        "williams_r" => Some(indicator::WilliamsR { period }.compute(data)),
        "cci" => Some(indicator::Cci { period }.compute(data)),
        "adx" => Some(indicator::Adx { period }.compute(data)),
        "donchian" => Some(indicator::Donchian { period }.compute(data)),
        "keltner" => Some(
            (indicator::Keltner {
                ema_period: period,
                atr_period: period,
                multiplier: 2.0,
            })
            .compute(data),
        ),
        "parabolic_sar" => Some(
            (indicator::ParabolicSar {
                af_step: 0.02,
                af_max: 0.2,
            })
            .compute(data),
        ),
        "supertrend" => Some(
            (indicator::Supertrend {
                period,
                multiplier: 3.0,
            })
            .compute(data),
        ),
        "ichimoku" => Some(
            (indicator::Ichimoku {
                tenkan_period: 9,
                kijun_period: 26,
                senkou_b_period: 52,
            })
            .compute(data),
        ),
        "session_vwap" => Some(indicator::SessionVwap.compute(data)),
        "volume_sma" => Some(indicator::VolumeSma { period }.compute(data)),
        _ => None,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[allow(clippy::many_single_char_names)]
mod tests {
    use super::*;

    #[test]
    fn create_and_destroy() {
        let handle = fc_chart_create();
        assert!(!handle.is_null());
        unsafe { fc_chart_destroy(handle) };
    }

    #[test]
    fn set_data_and_render() {
        let handle = fc_chart_create();
        let ts = [1i64, 2, 3, 4, 5];
        let o = [100.0, 101.0, 102.0, 103.0, 104.0];
        let h = [105.0, 106.0, 107.0, 108.0, 109.0];
        let l = [95.0, 96.0, 97.0, 98.0, 99.0];
        let c = [102.0, 103.0, 104.0, 105.0, 106.0];
        let v = [1000.0, 1100.0, 1200.0, 1300.0, 1400.0];

        unsafe {
            fc_chart_set_data(
                handle,
                ts.as_ptr(),
                o.as_ptr(),
                h.as_ptr(),
                l.as_ptr(),
                c.as_ptr(),
                v.as_ptr(),
                5,
            );

            assert_eq!(fc_chart_bar_count(handle), 5);

            let svg = fc_chart_render_svg(handle);
            assert!(!svg.is_null());

            let svg_str = CStr::from_ptr(svg).to_str().unwrap();
            assert!(svg_str.starts_with("<svg"));

            fc_string_free(svg);
            fc_chart_destroy(handle);
        }
    }

    #[test]
    fn version_string() {
        let ver = fc_version();
        let s = unsafe { CStr::from_ptr(ver) }.to_str().unwrap();
        assert_eq!(s, "0.1.0");
    }

    #[test]
    fn set_chart_type() {
        let handle = fc_chart_create();
        unsafe {
            let name = c"heikin_ashi".as_ptr();
            assert_eq!(fc_chart_set_type(handle, name), 0);

            let bad = c"nonexistent".as_ptr();
            assert_eq!(fc_chart_set_type(handle, bad), -1);

            fc_chart_destroy(handle);
        }
    }

    #[test]
    fn add_indicator() {
        let handle = fc_chart_create();
        let ts = [1i64, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let prices = [100.0; 15];
        let vol = [1000.0; 15];

        unsafe {
            fc_chart_set_data(
                handle,
                ts.as_ptr(),
                prices.as_ptr(),
                prices.as_ptr(),
                prices.as_ptr(),
                prices.as_ptr(),
                vol.as_ptr(),
                15,
            );

            let sma = c"sma".as_ptr();
            assert_eq!(fc_chart_add_indicator(handle, sma, 5), 0);

            let bad = c"nonexistent_indicator".as_ptr();
            assert_eq!(fc_chart_add_indicator(handle, bad, 5), -1);

            fc_chart_clear_indicators(handle);
            fc_chart_destroy(handle);
        }
    }

    #[test]
    fn render_empty_returns_null() {
        let handle = fc_chart_create();
        unsafe {
            let svg = fc_chart_render_svg(handle);
            assert!(svg.is_null());
            fc_chart_destroy(handle);
        }
    }
}
