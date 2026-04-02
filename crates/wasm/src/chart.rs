// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use ferrochart_core::indicator::{
    Adx, AnchoredVwap, Atr, BollingerBands, Cci, Cusum, Donchian, Ema, Ichimoku, Keltner, Macd,
    Obv, ParabolicSar, Rsi, SessionVwap, Sma, Stochastic, Supertrend, VolumeSma, WilliamsR,
};
use ferrochart_core::interaction::{compute_pan, compute_zoom, is_in_chart_area};
use ferrochart_core::{
    Annotations, BarrierOutcome, ChartType, ConfidenceBand, Corridor, FibonacciRetracement,
    HorizontalHistogram, HorizontalLevel, HorizontalRay, Indicator, IndicatorOutput,
    IndicatorPlacement, Marker, MarkerPosition, MarkerSet, MarkerShape, NewsEvent, Ohlcv, Point,
    PriceRange, Rect, RectangleZone, SeriesStyle, TextLabel, TimeRange, Transform, TrendLine,
    TripleBarrier, VerticalLine, Viewport, WalkForwardZone, ZoomPanState,
};
use ferrochart_render::Renderer;
use ferrochart_render::chart::{
    ChartConfig, ChartLayoutInfo, PanelKind, render_full_chart_with_markers,
};
use ferrochart_render::style::{Color, FillStyle, LineStyle, TextAnchor, TextStyle};

use crate::CanvasRenderer;

type RafClosure = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

/// Layer-granular dirty flags for selective redraw.
///
/// Most viewport changes (pan, zoom) mark all layers dirty.
/// The real win is for frequent lightweight updates:
/// - Crosshair movement: `OVERLAY` only
/// - Realtime tick: `CANDLES | INDICATORS`
/// - Annotation edits: `ANNOTATIONS`
#[derive(Clone, Copy, Default)]
struct DirtyFlags(u8);

impl DirtyFlags {
    const CANDLES: u8 = 0b0001;
    const INDICATORS: u8 = 0b0010;
    const ANNOTATIONS: u8 = 0b0100;
    const OVERLAY: u8 = 0b1000;
    const ALL: u8 = 0b1111;

    fn mark(&mut self, layers: u8) {
        self.0 |= layers;
    }
    fn mark_all(&mut self) {
        self.0 = Self::ALL;
    }
    fn any(self) -> bool {
        self.0 != 0
    }
    fn clear(&mut self) {
        self.0 = 0;
    }
}

/// Active drawing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DrawMode {
    None,
    TrendLine,
    Fibonacci,
    /// Corridor: first two clicks = trendline, third click = parallel offset.
    Corridor,
}

/// In-progress drawing.
#[derive(Debug, Clone, Copy)]
struct DrawingInProgress {
    /// Start in data coordinates (bar index, price).
    start_bar: f64,
    start_price: f64,
    /// For corridor: second point (set after first two clicks).
    end_bar: Option<f64>,
    end_price: Option<f64>,
}

/// State for an active panel splitter drag.
#[derive(Debug, Clone)]
struct SplitterDrag {
    /// Index of the panel above the splitter.
    panel_above: usize,
    start_y: f64,
    start_weights: Vec<f64>,
}

/// Internal mutable state shared between the chart and event closures.
struct ChartState {
    canvas: HtmlCanvasElement,
    data: Vec<Ohlcv>,
    config: ChartConfig,
    zoom_pan: ZoomPanState,
    indicators: Vec<Box<dyn Indicator>>,
    /// Cached indicator outputs computed on the full dataset.
    cached_outputs: Vec<IndicatorOutput>,
    markers: MarkerSet,
    annotations: Annotations,
    draw_mode: DrawMode,
    drawing: Option<DrawingInProgress>,
    /// Cached layout info from last render (for coordinate mapping).
    last_layout: ChartLayoutInfo,
    mouse_pos: Option<Point>,
    is_dragging: bool,
    drag_start_x: f64,
    drag_start_offset: usize,
    /// Y-axis drag scaling: multiplier for price range (1.0 = auto).
    price_scale: f64,
    y_drag_active: bool,
    y_drag_start_y: f64,
    y_drag_start_scale: f64,
    /// Panel splitter: custom weights (None = default).
    panel_weights: Option<Vec<f64>>,
    splitter_drag: Option<SplitterDrag>,
    /// For pinch-zoom: distance between two touches at start.
    pinch_start_dist: f64,
    pinch_start_visible: usize,
    /// Number of price buckets for volume profile (0 = disabled).
    volume_profile_buckets: usize,
    /// Active chart type (candlestick, line, area, etc.).
    chart_type: ChartType,
    dirty: DirtyFlags,
}

impl ChartState {
    /// Recompute all indicator outputs from the full dataset.
    fn recompute_indicators(&mut self) {
        self.cached_outputs = self
            .indicators
            .iter()
            .map(|ind| ind.compute(&self.data))
            .collect();
    }
}

/// Interactive candlestick chart rendered on an HTML canvas.
#[wasm_bindgen]
pub struct FerroChart {
    state: Rc<RefCell<ChartState>>,
    _closures: Vec<Closure<dyn FnMut(web_sys::MouseEvent)>>,
    _touch_closures: Vec<Closure<dyn FnMut(web_sys::TouchEvent)>>,
    _wheel_closure: Option<Closure<dyn FnMut(web_sys::WheelEvent)>>,
    _key_closure: Option<Closure<dyn FnMut(web_sys::KeyboardEvent)>>,
    _raf_closure: RafClosure,
}

#[wasm_bindgen]
impl FerroChart {
    /// Create a new interactive chart on the given canvas element.
    ///
    /// # Errors
    ///
    /// Returns a `JsValue` error if event listeners cannot be attached.
    ///
    /// # Panics
    ///
    /// Panics if `window()` is not available (non-browser environment).
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: &HtmlCanvasElement) -> Result<FerroChart, JsValue> {
        console_error_panic_hook::set_once();
        let config = ChartConfig {
            width: f64::from(canvas.width()),
            height: f64::from(canvas.height()),
            ..ChartConfig::default()
        };

        let state = Rc::new(RefCell::new(ChartState {
            canvas: canvas.clone(),
            data: Vec::new(),
            config,
            zoom_pan: ZoomPanState::new(0, 100),
            indicators: Vec::new(),
            cached_outputs: Vec::new(),
            markers: MarkerSet::new(),
            annotations: Annotations::new(),
            draw_mode: DrawMode::None,
            drawing: None,
            last_layout: ChartLayoutInfo::default(),
            mouse_pos: None,
            is_dragging: false,
            drag_start_x: 0.0,
            drag_start_offset: 0,
            price_scale: 1.0,
            y_drag_active: false,
            y_drag_start_y: 0.0,
            y_drag_start_scale: 1.0,
            panel_weights: None,
            splitter_drag: None,
            pinch_start_dist: 0.0,
            pinch_start_visible: 100,
            volume_profile_buckets: 0,
            chart_type: ChartType::Candlestick,
            dirty: DirtyFlags(DirtyFlags::ALL),
        }));

        let mut closures: Vec<Closure<dyn FnMut(web_sys::MouseEvent)>> = Vec::new();
        attach_mouse_events(canvas, &state, &mut closures)?;
        let on_wheel = attach_wheel_event(canvas, &state)?;
        let mut touch_closures = Vec::new();
        attach_touch_events(canvas, &state, &mut touch_closures)?;
        let on_key = attach_keyboard_events(canvas, &state)?;
        let raf_handle = start_render_loop(&state);

        Ok(FerroChart {
            state,
            _closures: closures,
            _touch_closures: touch_closures,
            _wheel_closure: Some(on_wheel),
            _key_closure: Some(on_key),
            _raf_closure: raf_handle,
        })
    }

    /// Set the OHLCV data from parallel arrays.
    #[wasm_bindgen(js_name = setData)]
    pub fn set_data(
        &self,
        timestamps: &[f64],
        opens: &[f64],
        highs: &[f64],
        lows: &[f64],
        closes: &[f64],
        volumes: &[f64],
    ) {
        let len = timestamps.len();
        let data: Vec<Ohlcv> = (0..len)
            .map(|i| Ohlcv {
                timestamp: timestamps[i] as i64,
                open: opens[i],
                high: highs[i],
                low: lows[i],
                close: closes[i],
                volume: volumes[i],
                institutional_ratio: 0.0,
            })
            .collect();

        let mut st = self.state.borrow_mut();
        let total = data.len();
        st.data = data;
        let future = total / 3; // allow scrolling 33% past data
        st.zoom_pan = ZoomPanState::new(total, 100.min(total)).with_future_bars(future);
        st.recompute_indicators();
        st.dirty.mark_all();
    }

    /// Set OHLCV data with institutional activity ratios.
    ///
    /// Same as `setData` but accepts an additional `institutional_ratios` array
    /// (values 0.0–1.0) that controls split-body candle rendering.
    #[wasm_bindgen(js_name = setDataWithRatios)]
    #[allow(clippy::too_many_arguments)]
    pub fn set_data_with_ratios(
        &self,
        timestamps: &[f64],
        opens: &[f64],
        highs: &[f64],
        lows: &[f64],
        closes: &[f64],
        volumes: &[f64],
        institutional_ratios: &[f64],
    ) {
        let len = timestamps.len();
        let data: Vec<Ohlcv> = (0..len)
            .map(|i| Ohlcv {
                timestamp: timestamps[i] as i64,
                open: opens[i],
                high: highs[i],
                low: lows[i],
                close: closes[i],
                volume: volumes[i],
                institutional_ratio: institutional_ratios.get(i).copied().unwrap_or(0.0),
            })
            .collect();

        let mut st = self.state.borrow_mut();
        let total = data.len();
        st.data = data;
        let future = total / 3;
        st.zoom_pan = ZoomPanState::new(total, 100.min(total)).with_future_bars(future);
        st.recompute_indicators();
        st.dirty.mark_all();
    }

    /// Update the last candle in-place (realtime tick).
    ///
    /// Does not change zoom/pan state. Recomputes indicators since the
    /// last bar's values affect moving averages etc.
    #[wasm_bindgen(js_name = updateLastCandle)]
    #[allow(clippy::too_many_arguments)]
    pub fn update_last_candle(
        &self,
        timestamp: f64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) {
        let mut st = self.state.borrow_mut();
        if let Some(last) = st.data.last_mut() {
            last.timestamp = timestamp as i64;
            last.open = open;
            last.high = high;
            last.low = low;
            last.close = close;
            last.volume = volume;
        }
        st.recompute_indicators();
        st.dirty.mark(DirtyFlags::CANDLES | DirtyFlags::INDICATORS);
    }

    /// Append a new candle (new trading period).
    ///
    /// If the user was viewing the latest bar, the view auto-scrolls to follow.
    /// If scrolled back into history, the view stays put.
    #[wasm_bindgen(js_name = pushCandle)]
    #[allow(clippy::too_many_arguments)]
    pub fn push_candle(
        &self,
        timestamp: f64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) {
        let mut st = self.state.borrow_mut();
        st.data.push(Ohlcv {
            timestamp: timestamp as i64,
            open,
            high,
            low,
            close,
            volume,
            institutional_ratio: 0.0,
        });
        let total = st.data.len();
        let future = total / 3;
        let was_at_end = st.zoom_pan.offset + st.zoom_pan.visible_bars >= st.zoom_pan.total_bars;
        st.zoom_pan.total_bars = total;
        st.zoom_pan.future_bars = future;
        if was_at_end {
            st.zoom_pan = st.zoom_pan.scroll_to_end();
        }
        st.recompute_indicators();
        st.dirty.mark_all();
    }

    /// Add an indicator by name.
    ///
    /// Supported: `"sma"`, `"ema"`, `"bollinger"`, `"rsi"`, `"macd"`.
    /// `period` is the main period parameter (default depends on indicator).
    ///
    /// # Errors
    ///
    /// Returns an error if the indicator name is unknown.
    #[wasm_bindgen(js_name = addIndicator)]
    pub fn add_indicator(&self, name: &str, period: Option<u32>) -> Result<(), JsValue> {
        let indicator: Box<dyn Indicator> = match name {
            "sma" => Box::new(Sma {
                period: period.unwrap_or(20) as usize,
            }),
            "ema" => Box::new(Ema {
                period: period.unwrap_or(20) as usize,
            }),
            "bollinger" => Box::new(BollingerBands {
                period: period.unwrap_or(20) as usize,
                std_dev: 2.0,
            }),
            "rsi" => Box::new(Rsi {
                period: period.unwrap_or(14) as usize,
            }),
            "macd" => Box::new(Macd {
                fast_period: 12,
                slow_period: period.unwrap_or(26) as usize,
                signal_period: 9,
            }),
            "volsma" => Box::new(VolumeSma {
                period: period.unwrap_or(20) as usize,
            }),
            "cusum" => Box::new(Cusum {
                threshold: period.map_or(0.03, |p| f64::from(p) / 1000.0),
            }),
            "atr" => Box::new(Atr {
                period: period.unwrap_or(14) as usize,
            }),
            "obv" => Box::new(Obv),
            "session_vwap" => Box::new(SessionVwap),
            "stochastic" => Box::new(Stochastic {
                k_period: period.unwrap_or(14) as usize,
                d_period: 3,
            }),
            "donchian" => Box::new(Donchian {
                period: period.unwrap_or(20) as usize,
            }),
            "keltner" => Box::new(Keltner {
                ema_period: period.unwrap_or(20) as usize,
                atr_period: 10,
                multiplier: 2.0,
            }),
            "williams_r" => Box::new(WilliamsR {
                period: period.unwrap_or(14) as usize,
            }),
            "cci" => Box::new(Cci {
                period: period.unwrap_or(20) as usize,
            }),
            "adx" => Box::new(Adx {
                period: period.unwrap_or(14) as usize,
            }),
            "parabolic_sar" => Box::new(ParabolicSar::default()),
            "supertrend" => Box::new(Supertrend {
                period: period.unwrap_or(10) as usize,
                multiplier: 3.0,
            }),
            "ichimoku" => Box::new(Ichimoku {
                tenkan_period: period.unwrap_or(9) as usize,
                kijun_period: 26,
                senkou_b_period: 52,
            }),
            _ => return Err(JsValue::from_str(&format!("unknown indicator: {name}"))),
        };

        let mut st = self.state.borrow_mut();
        st.indicators.push(indicator);
        st.recompute_indicators();
        st.dirty.mark(DirtyFlags::INDICATORS | DirtyFlags::CANDLES);
        Ok(())
    }

    /// Add an anchored VWAP overlay starting from the given bar index.
    #[wasm_bindgen(js_name = addAnchoredVwap)]
    pub fn add_anchored_vwap(&self, anchor_bar: u32) {
        let mut st = self.state.borrow_mut();
        st.indicators.push(Box::new(AnchoredVwap {
            anchor_bar: anchor_bar as usize,
        }));
        st.recompute_indicators();
        st.dirty.mark(DirtyFlags::INDICATORS | DirtyFlags::CANDLES);
    }

    /// Remove an indicator by name (e.g. `"sma"`, `"rsi"`).
    /// Removes all indicators matching that name.
    #[wasm_bindgen(js_name = removeIndicator)]
    pub fn remove_indicator(&self, name: &str) {
        let mut st = self.state.borrow_mut();
        let target = name.to_ascii_lowercase();
        st.indicators.retain(|ind| {
            let ind_name = ind.name().to_ascii_lowercase();
            ind_name != target
        });
        st.recompute_indicators();
        st.dirty.mark(DirtyFlags::INDICATORS | DirtyFlags::CANDLES);
    }

    /// Remove all indicators.
    #[wasm_bindgen(js_name = clearIndicators)]
    pub fn clear_indicators(&self) {
        let mut st = self.state.borrow_mut();
        st.indicators.clear();
        st.cached_outputs.clear();
        st.dirty.mark(DirtyFlags::INDICATORS | DirtyFlags::CANDLES);
    }

    /// Set the chart type.
    ///
    /// Supported names: `"candlestick"`, `"heikin_ashi"`, `"line"`, `"area"`, `"ohlc"`,
    /// `"renko"`, `"point_figure"`.
    ///
    /// # Errors
    ///
    /// Returns an error if the chart type name is unknown.
    #[wasm_bindgen(js_name = setChartType)]
    pub fn set_chart_type(&self, name: &str) -> Result<(), JsValue> {
        let chart_type = match name {
            "candlestick" => ChartType::Candlestick,
            "heikin_ashi" => ChartType::HeikinAshi,
            "line" => ChartType::Line,
            "area" => ChartType::Area,
            "ohlc" => ChartType::OhlcBars,
            "renko" => ChartType::Renko { brick_size: 1.0 },
            "point_figure" => ChartType::PointFigure {
                box_size: 1.0,
                reversal: 3,
            },
            _ => return Err(JsValue::from_str(&format!("unknown chart type: {name}"))),
        };
        let mut st = self.state.borrow_mut();
        if st.chart_type != chart_type {
            st.chart_type = chart_type;
            st.dirty.mark_all();
        }
        Ok(())
    }

    /// Set chart type to Renko with a custom brick size.
    #[wasm_bindgen(js_name = setRenkoConfig)]
    pub fn set_renko_config(&self, brick_size: f64) {
        let mut st = self.state.borrow_mut();
        st.chart_type = ChartType::Renko { brick_size };
        st.dirty.mark_all();
    }

    /// Set chart type to Point & Figure with a custom box size and reversal count.
    #[wasm_bindgen(js_name = setPfConfig)]
    pub fn set_pf_config(&self, box_size: f64, reversal: usize) {
        let mut st = self.state.borrow_mut();
        st.chart_type = ChartType::PointFigure { box_size, reversal };
        st.dirty.mark_all();
    }

    /// Add a marker at a specific bar index.
    ///
    /// `shape`: `"arrow_up"`, `"arrow_down"`, `"circle"`, `"diamond"`.
    /// `position`: `"above"` or `"below"`.
    ///
    /// # Errors
    ///
    /// Returns an error if the shape or position is unknown.
    #[wasm_bindgen(js_name = addMarker)]
    #[allow(clippy::too_many_arguments)]
    pub fn add_marker(
        &self,
        bar_index: u32,
        shape: &str,
        position: &str,
        r: u8,
        g: u8,
        b: u8,
        label: &str,
    ) -> Result<(), JsValue> {
        let shape = match shape {
            "arrow_up" => MarkerShape::ArrowUp,
            "arrow_down" => MarkerShape::ArrowDown,
            "circle" => MarkerShape::Circle,
            "diamond" => MarkerShape::Diamond,
            _ => return Err(JsValue::from_str(&format!("unknown shape: {shape}"))),
        };
        let position = match position {
            "above" => MarkerPosition::AboveBar,
            "below" => MarkerPosition::BelowBar,
            _ => return Err(JsValue::from_str(&format!("unknown position: {position}"))),
        };

        let mut st = self.state.borrow_mut();
        st.markers.add(Marker {
            bar_index: bar_index as usize,
            shape,
            position,
            color: (r, g, b, 255),
            label: label.to_string(),
        });
        st.dirty.mark(DirtyFlags::CANDLES);
        Ok(())
    }

    /// Remove all markers.
    #[wasm_bindgen(js_name = clearMarkers)]
    pub fn clear_markers(&self) {
        let mut st = self.state.borrow_mut();
        st.markers.clear();
        st.dirty.mark(DirtyFlags::CANDLES);
    }

    /// Add a trendline between two bar/price points.
    #[wasm_bindgen(js_name = addTrendLine)]
    #[allow(clippy::too_many_arguments)]
    pub fn add_trend_line(
        &self,
        start_bar: f64,
        start_price: f64,
        end_bar: f64,
        end_price: f64,
        r: u8,
        g: u8,
        b: u8,
        extend_right: bool,
    ) {
        let mut st = self.state.borrow_mut();
        st.annotations.add_trend_line(TrendLine {
            start_bar,
            start_price,
            end_bar,
            end_price,
            color: (r, g, b),
            width: 1.5,
            extend_right,
        });
        st.dirty.mark(DirtyFlags::ANNOTATIONS);
    }

    /// Add a Fibonacci retracement between a high and low point.
    #[wasm_bindgen(js_name = addFibonacci)]
    #[allow(clippy::too_many_arguments)]
    pub fn add_fibonacci(
        &self,
        high_bar: u32,
        high_price: f64,
        low_bar: u32,
        low_price: f64,
        r: u8,
        g: u8,
        b: u8,
    ) {
        let mut st = self.state.borrow_mut();
        st.annotations.add_fibonacci(FibonacciRetracement {
            high_bar: high_bar as usize,
            high_price,
            low_bar: low_bar as usize,
            low_price,
            color: (r, g, b),
        });
        st.dirty.mark(DirtyFlags::ANNOTATIONS);
    }

    /// Add a triple barrier overlay (take-profit, stop-loss, time limit).
    ///
    /// `outcome`: `"tp"`, `"sl"`, `"time"`, or empty string if unknown.
    #[wasm_bindgen(js_name = addTripleBarrier)]
    #[allow(clippy::too_many_arguments)]
    pub fn add_triple_barrier(
        &self,
        entry_bar: u32,
        entry_price: f64,
        tp_price: f64,
        sl_price: f64,
        horizon: u32,
        exit_bar: Option<u32>,
        outcome: &str,
        r: u8,
        g: u8,
        b: u8,
    ) {
        let outcome_enum = match outcome {
            "tp" => Some(BarrierOutcome::TakeProfit),
            "sl" => Some(BarrierOutcome::StopLoss),
            "time" => Some(BarrierOutcome::TimeExpired),
            _ => None,
        };
        let mut st = self.state.borrow_mut();
        st.annotations.add_triple_barrier(TripleBarrier {
            entry_bar: entry_bar as usize,
            entry_price,
            tp_price,
            sl_price,
            horizon: horizon as usize,
            exit_bar: exit_bar.map(|b| b as usize),
            outcome: outcome_enum,
            color: (r, g, b),
        });
        st.dirty.mark(DirtyFlags::ANNOTATIONS);
    }

    /// Add an ML confidence band overlay on the price panel.
    ///
    /// `upper` and `lower` are parallel arrays of prices (one per bar).
    #[wasm_bindgen(js_name = addConfidenceBand)]
    pub fn add_confidence_band(
        &self,
        upper: &[f64],
        lower: &[f64],
        r: u8,
        g: u8,
        b: u8,
        alpha: u8,
    ) {
        let mut st = self.state.borrow_mut();
        st.annotations.add_confidence_band(ConfidenceBand {
            upper: upper.to_vec(),
            lower: lower.to_vec(),
            color: (r, g, b),
            alpha,
        });
        st.dirty.mark(DirtyFlags::ANNOTATIONS);
    }

    /// Add a walk-forward train/validation zone.
    #[wasm_bindgen(js_name = addWalkForwardZone)]
    #[allow(clippy::too_many_arguments)]
    pub fn add_walk_forward_zone(&self, start_bar: u32, end_bar: u32, is_train: bool, label: &str) {
        let mut st = self.state.borrow_mut();
        st.annotations.add_walk_forward_zone(WalkForwardZone {
            start_bar: start_bar as usize,
            end_bar: end_bar as usize,
            is_train,
            label: label.to_string(),
            color: None,
        });
        st.dirty.mark(DirtyFlags::ANNOTATIONS);
    }

    /// Add a news/event marker at a specific bar.
    ///
    /// `impact`: -1.0 (bearish) to +1.0 (bullish).
    /// `urgency`: 0=low, 1=medium, 2=high, 3=critical.
    #[wasm_bindgen(js_name = addNewsEvent)]
    #[allow(clippy::too_many_arguments)]
    pub fn add_news_event(&self, bar_index: u32, label: &str, impact: f64, urgency: u8) {
        let mut st = self.state.borrow_mut();
        st.annotations.add_news_event(NewsEvent {
            bar_index: bar_index as usize,
            label: label.to_string(),
            impact,
            urgency,
            color: None,
        });
        st.dirty.mark(DirtyFlags::ANNOTATIONS);
    }

    /// Add a horizontal histogram (e.g. GEX profile) to the price panel.
    ///
    /// `prices` and `values` are parallel arrays of price levels and their values.
    #[wasm_bindgen(js_name = addHorizontalHistogram)]
    #[allow(clippy::too_many_arguments)]
    pub fn add_horizontal_histogram(
        &self,
        prices: &[f64],
        values: &[f64],
        label: &str,
        r: u8,
        g: u8,
        b: u8,
        alpha: u8,
    ) {
        let levels = prices
            .iter()
            .zip(values.iter())
            .map(|(&p, &v)| (p, v))
            .collect();
        let mut st = self.state.borrow_mut();
        st.annotations
            .add_horizontal_histogram(HorizontalHistogram {
                levels,
                label: label.to_string(),
                color: (r, g, b),
                alpha,
            });
        st.dirty.mark(DirtyFlags::ANNOTATIONS);
    }

    /// Add a horizontal price level line (e.g. Max Pain).
    #[wasm_bindgen(js_name = addHorizontalLevel)]
    #[allow(clippy::too_many_arguments)]
    pub fn add_horizontal_level(&self, price: f64, label: &str, r: u8, g: u8, b: u8, width: f64) {
        let mut st = self.state.borrow_mut();
        st.annotations.add_horizontal_level(HorizontalLevel {
            price,
            label: label.to_string(),
            color: (r, g, b),
            width,
        });
        st.dirty.mark(DirtyFlags::ANNOTATIONS);
    }

    /// Add a horizontal ray (full-width price line) at the given price level.
    ///
    /// `color_hex` should be a `"#RRGGBB"` hex string.
    #[wasm_bindgen(js_name = addHorizontalRay)]
    pub fn add_horizontal_ray(&self, price: f64, color_hex: &str, width: f64) {
        let color = parse_color(color_hex);
        let mut st = self.state.borrow_mut();
        st.annotations.add_horizontal_ray(HorizontalRay {
            price,
            color,
            width,
        });
        st.dirty.mark(DirtyFlags::ANNOTATIONS);
    }

    /// Add a vertical line at the given bar index.
    ///
    /// `color_hex` should be a `"#RRGGBB"` hex string.
    #[wasm_bindgen(js_name = addVerticalLine)]
    pub fn add_vertical_line(&self, bar_index: f64, color_hex: &str, width: f64) {
        let color = parse_color(color_hex);
        let mut st = self.state.borrow_mut();
        st.annotations.add_vertical_line(VerticalLine {
            bar_index,
            color,
            width,
        });
        st.dirty.mark(DirtyFlags::ANNOTATIONS);
    }

    /// Add a price × time rectangle zone.
    ///
    /// `color_hex` is the border color; `fill_hex` is the fill color, both as `"#RRGGBB"`.
    /// Fill alpha defaults to 30 (semi-transparent).
    #[wasm_bindgen(js_name = addRectangle)]
    #[allow(clippy::too_many_arguments)]
    pub fn add_rectangle(
        &self,
        start_bar: f64,
        end_bar: f64,
        top_price: f64,
        bottom_price: f64,
        color_hex: &str,
        fill_hex: &str,
    ) {
        let border_color = parse_color(color_hex);
        let fill_rgb = parse_color(fill_hex);
        let fill_color = (fill_rgb.0, fill_rgb.1, fill_rgb.2, 30u8);
        let mut st = self.state.borrow_mut();
        st.annotations.add_rectangle_zone(RectangleZone {
            start_bar,
            end_bar,
            top_price,
            bottom_price,
            border_color,
            fill_color,
            width: 1.0,
        });
        st.dirty.mark(DirtyFlags::ANNOTATIONS);
    }

    /// Add a text label at a specific bar and price position.
    ///
    /// `color_hex` should be a `"#RRGGBB"` hex string.
    #[wasm_bindgen(js_name = addTextLabel)]
    pub fn add_text_label(&self, bar_index: f64, price: f64, text: &str, color_hex: &str) {
        let color = parse_color(color_hex);
        let mut st = self.state.borrow_mut();
        st.annotations.add_text_label(TextLabel {
            bar_index,
            price,
            text: text.to_string(),
            color,
        });
        st.dirty.mark(DirtyFlags::ANNOTATIONS);
    }

    /// Add an equity curve sub-panel from pre-computed per-bar returns.
    #[wasm_bindgen(js_name = addEquityCurve)]
    pub fn add_equity_curve(&self, returns: &[f64]) {
        use ferrochart_core::indicator::EquityCurve;
        let mut st = self.state.borrow_mut();
        st.indicators.push(Box::new(EquityCurve {
            returns: returns.to_vec(),
        }));
        st.recompute_indicators();
        st.dirty.mark(DirtyFlags::INDICATORS | DirtyFlags::CANDLES);
    }

    /// Set the interactive drawing mode.
    ///
    /// `"trendline"` — click two points to draw a trendline.
    /// `"fibonacci"` — click two points (high/low) for Fibonacci retracement.
    /// `"none"` — normal mode (pan/zoom).
    ///
    /// # Errors
    ///
    /// Returns an error if the mode is unknown.
    #[wasm_bindgen(js_name = setDrawMode)]
    pub fn set_draw_mode(&self, mode: &str) -> Result<(), JsValue> {
        let mut st = self.state.borrow_mut();
        st.draw_mode = match mode {
            "none" => DrawMode::None,
            "trendline" => DrawMode::TrendLine,
            "fibonacci" => DrawMode::Fibonacci,
            "corridor" => DrawMode::Corridor,
            _ => return Err(JsValue::from_str(&format!("unknown draw mode: {mode}"))),
        };
        st.drawing = None;
        st.dirty.mark(DirtyFlags::ANNOTATIONS | DirtyFlags::OVERLAY);
        Ok(())
    }

    /// Remove all annotations (trendlines, Fibonacci).
    #[wasm_bindgen(js_name = clearAnnotations)]
    pub fn clear_annotations(&self) {
        let mut st = self.state.borrow_mut();
        st.annotations.clear();
        st.dirty.mark(DirtyFlags::ANNOTATIONS);
    }

    /// Export all annotations as a JSON string for persistence.
    ///
    /// # Errors
    ///
    /// Returns a `JsValue` error if serialization fails.
    #[wasm_bindgen(js_name = exportAnnotations)]
    pub fn export_annotations(&self) -> Result<String, JsValue> {
        let st = self.state.borrow();
        serde_json::to_string(&st.annotations)
            .map_err(|e| JsValue::from_str(&format!("serialize error: {e}")))
    }

    /// Import annotations from a JSON string (replaces current annotations).
    ///
    /// # Errors
    ///
    /// Returns a `JsValue` error if the JSON is invalid.
    #[wasm_bindgen(js_name = importAnnotations)]
    pub fn import_annotations(&self, json: &str) -> Result<(), JsValue> {
        let annotations: Annotations = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("deserialize error: {e}")))?;
        let mut st = self.state.borrow_mut();
        st.annotations = annotations;
        st.dirty.mark(DirtyFlags::ANNOTATIONS);
        Ok(())
    }

    /// Set the color theme: `"dark"` (default) or `"light"`.
    ///
    /// # Errors
    ///
    /// Returns an error if the theme name is unknown.
    #[wasm_bindgen(js_name = setTheme)]
    pub fn set_theme(&self, theme: &str) -> Result<(), JsValue> {
        let mut st = self.state.borrow_mut();
        let w = st.config.width;
        let h = st.config.height;
        let scale = st.config.price_scale;
        let weights = st.config.panel_weights.clone();
        let slots = st.config.visible_bar_slots;

        st.config = match theme {
            "dark" => ChartConfig::dark(),
            "light" => ChartConfig::light(),
            _ => return Err(JsValue::from_str(&format!("unknown theme: {theme}"))),
        };
        // Preserve runtime state
        st.config.width = w;
        st.config.height = h;
        st.config.price_scale = scale;
        st.config.panel_weights = weights;
        st.config.visible_bar_slots = slots;
        st.dirty.mark_all();
        Ok(())
    }

    /// Update the chart dimensions (call after canvas resize).
    pub fn resize(&self, width: u32, height: u32) {
        let mut st = self.state.borrow_mut();
        st.config.width = f64::from(width);
        st.config.height = f64::from(height);
        st.dirty.mark_all();
    }

    /// Enable or disable logarithmic Y-axis for the price panel.
    #[wasm_bindgen(js_name = setLogScale)]
    pub fn set_log_scale(&self, enabled: bool) {
        let mut st = self.state.borrow_mut();
        st.config.log_y = enabled;
        st.dirty.mark_all();
    }

    /// Show volume profile histogram on the price panel.
    ///
    /// `num_buckets`: number of price-level buckets (e.g. 50).
    /// Pass 0 to hide.
    #[wasm_bindgen(js_name = showVolumeProfile)]
    pub fn show_volume_profile(&self, num_buckets: u32) {
        let mut st = self.state.borrow_mut();
        st.volume_profile_buckets = num_buckets as usize;
        st.dirty.mark_all();
    }

    /// Get the current zoom/pan state as `[visible_bars, offset, total_bars]`.
    /// Use with `setZoomPanState` for multi-chart synchronization.
    #[must_use]
    #[wasm_bindgen(js_name = getZoomPanState)]
    pub fn get_zoom_pan_state(&self) -> Vec<u32> {
        let st = self.state.borrow();
        vec![
            st.zoom_pan.visible_bars as u32,
            st.zoom_pan.offset as u32,
            st.zoom_pan.total_bars as u32,
        ]
    }

    /// Set the zoom/pan state from `[visible_bars, offset, total_bars]`.
    /// Use for multi-chart synchronization.
    #[wasm_bindgen(js_name = setZoomPanState)]
    pub fn set_zoom_pan_state(&self, visible_bars: u32, offset: u32) {
        let mut st = self.state.borrow_mut();
        st.zoom_pan.visible_bars = visible_bars as usize;
        st.zoom_pan.offset = offset as usize;
        st.dirty.mark_all();
    }

    /// Set chart configuration from a JSON string.
    ///
    /// Accepts a partial config — only provided fields are updated.
    /// Width and height are preserved from current state.
    ///
    /// # Errors
    ///
    /// Returns a `JsValue` error if the JSON is invalid.
    #[wasm_bindgen(js_name = setConfig)]
    pub fn set_config(&self, json: &str) -> Result<(), JsValue> {
        let new_config: ChartConfig = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("config parse error: {e}")))?;
        let mut st = self.state.borrow_mut();
        // Preserve runtime dimensions
        let w = st.config.width;
        let h = st.config.height;
        st.config = new_config;
        st.config.width = w;
        st.config.height = h;
        st.dirty.mark_all();
        Ok(())
    }

    /// Set OHLCV data from a JSON array of objects.
    ///
    /// Each object must have: `timestamp`, `open`, `high`, `low`, `close`, `volume`.
    /// Optional: `institutional_ratio` (defaults to 0.0).
    ///
    /// # Errors
    ///
    /// Returns a `JsValue` error if the JSON is invalid.
    #[wasm_bindgen(js_name = setDataJson)]
    pub fn set_data_json(&self, json: &str) -> Result<(), JsValue> {
        let data: Vec<Ohlcv> = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("data parse error: {e}")))?;
        let mut st = self.state.borrow_mut();
        let total = data.len();
        st.data = data;
        let future = total / 3;
        st.zoom_pan = ZoomPanState::new(total, 100.min(total)).with_future_bars(future);
        st.recompute_indicators();
        st.dirty.mark_all();
        Ok(())
    }

    /// Handle a wheel event externally (for framework integration).
    ///
    /// `delta_y`: scroll amount (positive = zoom out, negative = zoom in).
    /// `mouse_x`: cursor X position in canvas-pixel coordinates.
    #[wasm_bindgen(js_name = onWheel)]
    pub fn on_wheel(&self, delta_y: f64, mouse_x: f64) {
        let mut st = self.state.borrow_mut();
        if st.data.is_empty() {
            return;
        }
        let chart_left = st.config.margin.left;
        let chart_width = st.config.width - chart_left - st.config.margin.right;
        st.zoom_pan = compute_zoom(st.zoom_pan, mouse_x, chart_left, chart_width, delta_y);
        st.dirty.mark_all();
    }

    /// Handle a pan event externally (for framework integration).
    ///
    /// `dx`: horizontal pixel distance dragged since pan start.
    #[wasm_bindgen(js_name = onPan)]
    pub fn on_pan(&self, dx: f64) {
        let mut st = self.state.borrow_mut();
        if st.data.is_empty() {
            return;
        }
        let chart_width = st.config.width - st.config.margin.left - st.config.margin.right;
        let drag_start = st.zoom_pan.offset;
        st.zoom_pan = compute_pan(st.zoom_pan, dx, chart_width, drag_start);
        st.dirty.mark_all();
    }
}

/// Parse a `"#RRGGBB"` hex color string into `(r, g, b)`.
///
/// Falls back to `(128, 128, 128)` if the string is not in the expected format.
fn parse_color(hex: &str) -> (u8, u8, u8) {
    let s = hex.trim().trim_start_matches('#');
    let parse = || -> Option<(u8, u8, u8)> {
        if s.len() != 6 {
            return None;
        }
        Some((
            u8::from_str_radix(&s[0..2], 16).ok()?,
            u8::from_str_radix(&s[2..4], 16).ok()?,
            u8::from_str_radix(&s[4..6], 16).ok()?,
        ))
    };
    parse().unwrap_or((128, 128, 128))
}

/// Helper: get mouse position relative to canvas in canvas-pixel coordinates.
/// Accounts for `devicePixelRatio` when CSS size differs from canvas resolution.
fn mouse_pos(e: &web_sys::MouseEvent, canvas: &HtmlCanvasElement) -> Point {
    let rect = canvas.get_bounding_client_rect();
    let css_x = f64::from(e.client_x()) - rect.left();
    let css_y = f64::from(e.client_y()) - rect.top();
    // Scale from CSS pixels to canvas pixels
    let scale_x = f64::from(canvas.width()) / rect.width();
    let scale_y = f64::from(canvas.height()) / rect.height();
    Point {
        x: css_x * scale_x,
        y: css_y * scale_y,
    }
}

fn attach_mouse_events(
    canvas: &HtmlCanvasElement,
    state: &Rc<RefCell<ChartState>>,
    closures: &mut Vec<Closure<dyn FnMut(web_sys::MouseEvent)>>,
) -> Result<(), JsValue> {
    // Mouse move (crosshair + drag + Y-axis scale + splitter)
    let s = Rc::clone(state);
    let on_mousemove = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
        let mut st = s.borrow_mut();
        let pos = mouse_pos(&e, &st.canvas);
        st.mouse_pos = Some(pos);

        if let Some(ref drag) = st.splitter_drag.clone() {
            // Splitter drag: redistribute weights between adjacent panels
            let dy = pos.y - drag.start_y;
            let total_h = st.config.height - st.config.margin.top - st.config.margin.bottom;
            let weight_delta = dy / total_h * drag.start_weights.iter().sum::<f64>();
            let mut w = drag.start_weights.clone();
            let above = drag.panel_above;
            let below = above + 1;
            if below < w.len() {
                w[above] = (w[above] + weight_delta).max(5.0);
                w[below] = (w[below] - weight_delta).max(5.0);
                st.panel_weights = Some(w);
            }
        } else if st.y_drag_active {
            let dy = st.y_drag_start_y - pos.y;
            let sensitivity = 0.005;
            st.price_scale = (st.y_drag_start_scale + dy * sensitivity).clamp(0.1, 10.0);
        } else if st.is_dragging {
            let dx = pos.x - st.drag_start_x;
            let chart_width = st.config.width - st.config.margin.left - st.config.margin.right;
            st.zoom_pan = compute_pan(st.zoom_pan, dx, chart_width, st.drag_start_offset);
            st.dirty.mark_all();
            return;
        }
        // Crosshair/tooltip only (most frequent case)
        st.dirty.mark(DirtyFlags::OVERLAY);
    }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    canvas.add_event_listener_with_callback("mousemove", on_mousemove.as_ref().unchecked_ref())?;
    closures.push(on_mousemove);

    // Mouse down (drawing, drag, Y-axis scale, or splitter)
    let s = Rc::clone(state);
    let on_mousedown = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
        let mut st = s.borrow_mut();
        let pos = mouse_pos(&e, &st.canvas);
        let y_axis_left = st.config.width - st.config.margin.right;

        // Drawing mode takes priority
        if st.draw_mode != DrawMode::None
            && pos.x < y_axis_left
            && let Some(data_pos) = pixel_to_data(&st, pos)
        {
            if let Some(start) = st.drawing {
                match st.draw_mode {
                    DrawMode::TrendLine => {
                        st.annotations.add_trend_line(TrendLine {
                            start_bar: start.start_bar,
                            start_price: start.start_price,
                            end_bar: data_pos.0,
                            end_price: data_pos.1,
                            color: (255, 255, 0),
                            width: 1.5,
                            extend_right: true,
                        });
                        st.drawing = None;
                        st.draw_mode = DrawMode::None;
                    }
                    DrawMode::Fibonacci => {
                        let (high_bar, high_price, low_bar, low_price) =
                            if start.start_price >= data_pos.1 {
                                (
                                    start.start_bar as usize,
                                    start.start_price,
                                    data_pos.0 as usize,
                                    data_pos.1,
                                )
                            } else {
                                (
                                    data_pos.0 as usize,
                                    data_pos.1,
                                    start.start_bar as usize,
                                    start.start_price,
                                )
                            };
                        st.annotations.add_fibonacci(FibonacciRetracement {
                            high_bar,
                            high_price,
                            low_bar,
                            low_price,
                            color: (255, 165, 0),
                        });
                        st.drawing = None;
                        st.draw_mode = DrawMode::None;
                    }
                    DrawMode::Corridor => {
                        if let (Some(end_bar), Some(end_price)) = (start.end_bar, start.end_price) {
                            // Third click → set corridor offset
                            let price_offset = data_pos.1 - start.start_price;
                            st.annotations.add_corridor(Corridor {
                                line: TrendLine {
                                    start_bar: start.start_bar,
                                    start_price: start.start_price,
                                    end_bar,
                                    end_price,
                                    color: (0, 150, 255),
                                    width: 1.0,
                                    extend_right: true,
                                },
                                offset: price_offset,
                            });
                            st.drawing = None;
                            st.draw_mode = DrawMode::None;
                        } else {
                            // Second click → set end of trendline, wait for third
                            st.drawing = Some(DrawingInProgress {
                                start_bar: start.start_bar,
                                start_price: start.start_price,
                                end_bar: Some(data_pos.0),
                                end_price: Some(data_pos.1),
                            });
                        }
                    }
                    DrawMode::None => {}
                }
            } else {
                // First click → start drawing
                st.drawing = Some(DrawingInProgress {
                    start_bar: data_pos.0,
                    start_price: data_pos.1,
                    end_bar: None,
                    end_price: None,
                });
            }
            st.dirty.mark(DirtyFlags::ANNOTATIONS | DirtyFlags::OVERLAY);
            return;
        }

        if pos.x >= y_axis_left {
            // Click in Y-axis area → start Y-scale drag
            st.y_drag_active = true;
            st.y_drag_start_y = pos.y;
            st.y_drag_start_scale = st.price_scale;
        } else if let Some(panel_idx) = find_splitter_at_y(&st, pos.y) {
            // Click on a splitter gap → start splitter drag
            let num_sub = st
                .cached_outputs
                .iter()
                .filter(|o| o.placement != IndicatorPlacement::Overlay)
                .count();
            let weights = st.panel_weights.clone().unwrap_or_else(|| {
                let mut w = vec![55.0, 20.0];
                w.extend(std::iter::repeat_n(15.0, num_sub));
                w
            });
            st.splitter_drag = Some(SplitterDrag {
                panel_above: panel_idx,
                start_y: pos.y,
                start_weights: weights,
            });
        } else {
            // Normal chart drag → pan
            st.is_dragging = true;
            st.drag_start_x = pos.x;
            st.drag_start_offset = st.zoom_pan.offset;
        }
    }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    canvas.add_event_listener_with_callback("mousedown", on_mousedown.as_ref().unchecked_ref())?;
    closures.push(on_mousedown);

    // Mouse up (stop drag)
    let s = Rc::clone(state);
    let on_mouseup = Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
        let mut st = s.borrow_mut();
        st.is_dragging = false;
        st.y_drag_active = false;
        st.splitter_drag = None;
    }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    canvas.add_event_listener_with_callback("mouseup", on_mouseup.as_ref().unchecked_ref())?;
    closures.push(on_mouseup);

    // Double-click on Y-axis resets scale
    let s = Rc::clone(state);
    let on_dblclick = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
        let mut st = s.borrow_mut();
        let pos = mouse_pos(&e, &st.canvas);
        let y_axis_left = st.config.width - st.config.margin.right;
        if pos.x >= y_axis_left {
            st.price_scale = 1.0;
            st.dirty.mark_all();
        }
    }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    canvas.add_event_listener_with_callback("dblclick", on_dblclick.as_ref().unchecked_ref())?;
    closures.push(on_dblclick);

    // Mouse leave (hide crosshair)
    let s = Rc::clone(state);
    let on_mouseleave = Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
        let mut st = s.borrow_mut();
        st.mouse_pos = None;
        st.is_dragging = false;
        st.y_drag_active = false;
        st.dirty.mark(DirtyFlags::OVERLAY);
    }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    canvas
        .add_event_listener_with_callback("mouseleave", on_mouseleave.as_ref().unchecked_ref())?;
    closures.push(on_mouseleave);

    Ok(())
}

fn attach_wheel_event(
    canvas: &HtmlCanvasElement,
    state: &Rc<RefCell<ChartState>>,
) -> Result<Closure<dyn FnMut(web_sys::WheelEvent)>, JsValue> {
    let s = Rc::clone(state);
    let on_wheel = Closure::wrap(Box::new(move |e: web_sys::WheelEvent| {
        e.prevent_default();
        let mut st = s.borrow_mut();
        if st.data.is_empty() {
            return;
        }
        let rect = st.canvas.get_bounding_client_rect();
        let scale_x = f64::from(st.canvas.width()) / rect.width();
        let mouse_x = (f64::from(e.client_x()) - rect.left()) * scale_x;
        let chart_left = st.config.margin.left;
        let chart_width = st.config.width - chart_left - st.config.margin.right;

        st.zoom_pan = compute_zoom(st.zoom_pan, mouse_x, chart_left, chart_width, e.delta_y());
        st.dirty.mark_all();
    }) as Box<dyn FnMut(web_sys::WheelEvent)>);

    let opts = web_sys::AddEventListenerOptions::new();
    opts.set_passive(false);
    canvas.add_event_listener_with_callback_and_add_event_listener_options(
        "wheel",
        on_wheel.as_ref().unchecked_ref(),
        &opts,
    )?;

    Ok(on_wheel)
}

/// Get touch position in canvas-pixel coordinates.
fn touch_pos(touch: &web_sys::Touch, canvas: &HtmlCanvasElement) -> Point {
    let rect = canvas.get_bounding_client_rect();
    let scale_x = f64::from(canvas.width()) / rect.width();
    let scale_y = f64::from(canvas.height()) / rect.height();
    Point {
        x: (f64::from(touch.client_x()) - rect.left()) * scale_x,
        y: (f64::from(touch.client_y()) - rect.top()) * scale_y,
    }
}

/// Distance between two touch points.
fn touch_distance(a: &web_sys::Touch, b: &web_sys::Touch) -> f64 {
    let dx = f64::from(a.client_x() - b.client_x());
    let dy = f64::from(a.client_y() - b.client_y());
    dx.hypot(dy)
}

fn attach_touch_events(
    canvas: &HtmlCanvasElement,
    state: &Rc<RefCell<ChartState>>,
    closures: &mut Vec<Closure<dyn FnMut(web_sys::TouchEvent)>>,
) -> Result<(), JsValue> {
    let opts = web_sys::AddEventListenerOptions::new();
    opts.set_passive(false);

    // touchstart
    let s = Rc::clone(state);
    let on_touchstart = Closure::wrap(Box::new(move |e: web_sys::TouchEvent| {
        e.prevent_default();
        let mut st = s.borrow_mut();
        let touches = e.touches();
        if touches.length() == 1 {
            // Single touch: start drag
            if let Some(t) = touches.get(0) {
                let pos = touch_pos(&t, &st.canvas);
                st.is_dragging = true;
                st.drag_start_x = pos.x;
                st.drag_start_offset = st.zoom_pan.offset;
                st.mouse_pos = Some(pos);
                st.dirty.mark_all();
            }
        } else if touches.length() == 2 {
            // Two touches: start pinch-zoom
            if let (Some(a), Some(b)) = (touches.get(0), touches.get(1)) {
                st.is_dragging = false;
                st.pinch_start_dist = touch_distance(&a, &b);
                st.pinch_start_visible = st.zoom_pan.visible_bars;
            }
        }
    }) as Box<dyn FnMut(web_sys::TouchEvent)>);
    canvas.add_event_listener_with_callback_and_add_event_listener_options(
        "touchstart",
        on_touchstart.as_ref().unchecked_ref(),
        &opts,
    )?;
    closures.push(on_touchstart);

    // touchmove
    let s = Rc::clone(state);
    let on_touchmove = Closure::wrap(Box::new(move |e: web_sys::TouchEvent| {
        e.prevent_default();
        let mut st = s.borrow_mut();
        let touches = e.touches();
        if touches.length() == 1 && st.is_dragging {
            // Single touch drag = pan
            if let Some(t) = touches.get(0) {
                let pos = touch_pos(&t, &st.canvas);
                let dx = pos.x - st.drag_start_x;
                let chart_width = st.config.width - st.config.margin.left - st.config.margin.right;
                st.zoom_pan = compute_pan(st.zoom_pan, dx, chart_width, st.drag_start_offset);
                st.mouse_pos = Some(pos);
                st.dirty.mark_all();
            }
        } else if touches.length() == 2 {
            // Pinch-zoom
            if let (Some(a), Some(b)) = (touches.get(0), touches.get(1)) {
                let dist = touch_distance(&a, &b);
                if st.pinch_start_dist > 1.0 {
                    let scale = dist / st.pinch_start_dist;
                    let new_visible = (st.pinch_start_visible as f64 / scale)
                        .round()
                        .clamp(5.0, st.zoom_pan.total_bars as f64)
                        as usize;
                    // Keep centered
                    let mid = st.zoom_pan.offset + st.zoom_pan.visible_bars / 2;
                    let new_offset = mid.saturating_sub(new_visible / 2);
                    st.zoom_pan = ZoomPanState {
                        visible_bars: new_visible,
                        offset: new_offset,
                        total_bars: st.zoom_pan.total_bars,
                        future_bars: 0,
                    };
                    // Clamp
                    st.zoom_pan = st.zoom_pan.pan(0);
                    st.dirty.mark_all();
                }
            }
        }
    }) as Box<dyn FnMut(web_sys::TouchEvent)>);
    canvas.add_event_listener_with_callback_and_add_event_listener_options(
        "touchmove",
        on_touchmove.as_ref().unchecked_ref(),
        &opts,
    )?;
    closures.push(on_touchmove);

    // touchend / touchcancel
    let s = Rc::clone(state);
    let on_touchend = Closure::wrap(Box::new(move |_e: web_sys::TouchEvent| {
        let mut st = s.borrow_mut();
        st.is_dragging = false;
        st.mouse_pos = None;
        st.dirty.mark(DirtyFlags::OVERLAY);
    }) as Box<dyn FnMut(web_sys::TouchEvent)>);
    canvas.add_event_listener_with_callback("touchend", on_touchend.as_ref().unchecked_ref())?;
    canvas.add_event_listener_with_callback("touchcancel", on_touchend.as_ref().unchecked_ref())?;
    closures.push(on_touchend);

    Ok(())
}

fn attach_keyboard_events(
    canvas: &HtmlCanvasElement,
    state: &Rc<RefCell<ChartState>>,
) -> Result<Closure<dyn FnMut(web_sys::KeyboardEvent)>, JsValue> {
    // Make canvas focusable
    canvas.set_tab_index(0);

    let s = Rc::clone(state);
    let on_keydown = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
        let mut st = s.borrow_mut();
        let key = e.key();
        match key.as_str() {
            "ArrowLeft" => {
                e.prevent_default();
                st.zoom_pan = st.zoom_pan.pan(-3);
                st.dirty.mark_all();
            }
            "ArrowRight" => {
                e.prevent_default();
                st.zoom_pan = st.zoom_pan.pan(3);
                st.dirty.mark_all();
            }
            "ArrowUp" => {
                e.prevent_default();
                st.price_scale = (st.price_scale - 0.1).clamp(0.1, 10.0);
                st.dirty.mark_all();
            }
            "ArrowDown" => {
                e.prevent_default();
                st.price_scale = (st.price_scale + 0.1).clamp(0.1, 10.0);
                st.dirty.mark_all();
            }
            "+" | "=" => {
                e.prevent_default();
                let mid = st.zoom_pan.offset + st.zoom_pan.visible_bars / 2;
                st.zoom_pan = st.zoom_pan.zoom(1.25, mid);
                st.dirty.mark_all();
            }
            "-" => {
                e.prevent_default();
                let mid = st.zoom_pan.offset + st.zoom_pan.visible_bars / 2;
                st.zoom_pan = st.zoom_pan.zoom(0.8, mid);
                st.dirty.mark_all();
            }
            "Escape" => {
                // Cancel drawing
                st.draw_mode = DrawMode::None;
                st.drawing = None;
                st.dirty.mark(DirtyFlags::ANNOTATIONS | DirtyFlags::OVERLAY);
            }
            "Home" => {
                e.prevent_default();
                st.zoom_pan = ZoomPanState {
                    offset: 0,
                    ..st.zoom_pan
                };
                st.dirty.mark_all();
            }
            "End" => {
                e.prevent_default();
                st.zoom_pan = st.zoom_pan.scroll_to_end();
                st.dirty.mark_all();
            }
            _ => {}
        }
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

    canvas.add_event_listener_with_callback("keydown", on_keydown.as_ref().unchecked_ref())?;
    Ok(on_keydown)
}

fn start_render_loop(state: &Rc<RefCell<ChartState>>) -> RafClosure {
    let s = Rc::clone(state);
    let raf_closure: RafClosure = Rc::new(RefCell::new(None));
    let raf_clone = Rc::clone(&raf_closure);

    *raf_closure.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        {
            let mut st = s.borrow_mut();
            if st.dirty.any() {
                st.dirty.clear();
                render_frame(&mut st);
            }
        }
        let window = web_sys::window().unwrap();
        let cb = raf_clone.borrow();
        let _ = window.request_animation_frame(cb.as_ref().unwrap().as_ref().unchecked_ref());
    }) as Box<dyn FnMut()>));

    // Start the loop
    {
        let window = web_sys::window().unwrap();
        let cb = raf_closure.borrow();
        let _ = window.request_animation_frame(cb.as_ref().unwrap().as_ref().unchecked_ref());
    }

    // Return the Rc — keeps the closure alive as long as FerroChart exists
    raf_closure
}

/// Convert a pixel position to data coordinates (bar index in full dataset, price).
/// Uses the cached transform from the last render for exact coordinate match.
fn pixel_to_data(st: &ChartState, pos: Point) -> Option<(f64, f64)> {
    let transform = st.last_layout.price_transform?;
    let range = st.zoom_pan.visible_range();
    let start = range.start.min(st.data.len());

    let (bar_f, price) = transform.to_data(pos);
    // Convert from visible-relative bar index to absolute dataset index
    let abs_bar = bar_f + start as f64;
    Some((abs_bar, price))
}

/// Check if a Y coordinate is in a gap between panels (splitter hit zone).
/// Returns the index of the panel above the gap, or None.
fn find_splitter_at_y(st: &ChartState, y: f64) -> Option<usize> {
    // We need to reconstruct the panel layout to find gaps.
    let num_sub = st
        .cached_outputs
        .iter()
        .filter(|o| o.placement != IndicatorPlacement::Overlay)
        .count();
    let expected = 2 + num_sub;
    let weights = st.panel_weights.clone().unwrap_or_else(|| {
        let mut w = vec![55.0, 20.0];
        w.extend(std::iter::repeat_n(15.0, num_sub));
        w
    });
    if weights.len() != expected {
        return None;
    }
    let total_rect = Rect::new(
        st.config.margin.left,
        st.config.margin.top,
        st.config.width - st.config.margin.left - st.config.margin.right,
        st.config.height - st.config.margin.top - st.config.margin.bottom,
    );
    let layout = ferrochart_core::PanelLayout::new(&weights, total_rect, 4.0);
    let hit_zone = 6.0; // pixels tolerance

    for i in 0..layout.len().saturating_sub(1) {
        let bottom = layout.get(i).unwrap().rect.bottom();
        let top_next = layout.get(i + 1).unwrap().rect.y;
        let gap_center = f64::midpoint(bottom, top_next);
        if (y - gap_center).abs() < hit_zone {
            return Some(i);
        }
    }
    None
}

/// Render one frame: chart + crosshair overlay.
fn render_frame(st: &mut ChartState) {
    if st.data.is_empty() {
        return;
    }

    // Ensure zoom_pan is consistent with current data length
    if st.zoom_pan.total_bars != st.data.len() {
        st.zoom_pan = ZoomPanState::new(st.data.len(), st.zoom_pan.visible_bars);
    }

    let range = st.zoom_pan.visible_range();
    let end = range.end.min(st.data.len());
    let start = range.start.min(end);
    let visible_data = &st.data[start..end];
    if visible_data.is_empty() {
        return;
    }

    let Ok(mut renderer) = CanvasRenderer::new(&st.canvas) else {
        return;
    };

    // LOD decimation: when bars are sub-pixel, aggregate for performance
    let chart_width = st.config.width - st.config.margin.left - st.config.margin.right;
    let decimation_target =
        ferrochart_core::decimation::decimate_target(visible_data.len(), chart_width);

    let (render_data, outputs) = if let Some(target) = decimation_target {
        let decimated = ferrochart_core::decimation::min_max_decimate(visible_data, target);
        let outputs: Vec<IndicatorOutput> = st
            .cached_outputs
            .iter()
            .map(|out| {
                let sliced = out.slice(start..end);
                IndicatorOutput {
                    name: sliced.name,
                    placement: sliced.placement,
                    series: sliced
                        .series
                        .iter()
                        .map(|s| ferrochart_core::IndicatorSeries {
                            name: s.name,
                            values: ferrochart_core::decimation::decimate_series(
                                &s.values,
                                target,
                                s.style_hint == ferrochart_core::SeriesStyle::Histogram,
                            ),
                            style_hint: s.style_hint,
                        })
                        .collect(),
                }
            })
            .collect();
        (decimated, outputs)
    } else {
        let outputs: Vec<IndicatorOutput> = st
            .cached_outputs
            .iter()
            .map(|out| out.slice(start..end))
            .collect();
        (visible_data.to_vec(), outputs)
    };

    // Get markers in visible range (adjust indices to be relative to visible slice)
    let visible_markers = st.markers.in_range(start, end);
    let adjusted_markers: Vec<Marker> = visible_markers
        .iter()
        .map(|m| Marker {
            bar_index: m.bar_index - start,
            shape: m.shape,
            position: m.position,
            color: m.color,
            label: m.label.clone(),
        })
        .collect();
    let marker_refs: Vec<&Marker> = adjusted_markers.iter().collect();

    // Apply Y-axis scale factor, panel weights, bar slot count, and chart type
    st.config.price_scale = st.price_scale;
    st.config.panel_weights = st.panel_weights.clone();
    st.config.visible_offset = start;
    st.config.chart_type = st.chart_type;
    // If scrolled into future space, there are fewer data bars than visible slots
    st.config.visible_bar_slots = if render_data.len() < st.zoom_pan.visible_bars {
        Some(st.zoom_pan.visible_bars)
    } else {
        None
    };

    // Compute volume profile on visible data (use original, not decimated)
    let vol_profile = if st.volume_profile_buckets > 0 {
        Some(ferrochart_core::indicator::VolumeProfile::compute(
            visible_data,
            st.volume_profile_buckets,
        ))
    } else {
        None
    };

    let layout_info = render_full_chart_with_markers(
        &mut renderer,
        &render_data,
        &outputs,
        &marker_refs,
        &st.annotations,
        vol_profile.as_ref(),
        &st.config,
    );
    st.last_layout = layout_info.clone();

    // Crosshair + Tooltip
    if let Some(mouse) = st.mouse_pos {
        let chart_left = st.config.margin.left;
        let chart_right = st.config.width - st.config.margin.right;
        let chart_top = st.config.margin.top;
        let chart_bottom = st.config.height - st.config.margin.bottom;

        if is_in_chart_area(mouse, chart_left, chart_right, chart_top, chart_bottom) {
            let crosshair_style = LineStyle {
                color: Color::rgba(200, 200, 200, 100),
                width: 0.5,
            };
            renderer.draw_line(
                Point {
                    x: mouse.x,
                    y: chart_top,
                },
                Point {
                    x: mouse.x,
                    y: chart_bottom,
                },
                &crosshair_style,
            );
            renderer.draw_line(
                Point {
                    x: chart_left,
                    y: mouse.y,
                },
                Point {
                    x: chart_right,
                    y: mouse.y,
                },
                &crosshair_style,
            );

            draw_tooltip(
                &mut renderer,
                mouse,
                visible_data,
                &outputs,
                &adjusted_markers,
                &layout_info,
                &st.config,
            );

            // Drawing preview
            if let Some(ref drawing) = st.drawing {
                draw_preview(&mut renderer, drawing, mouse, st, start);
            }
        }
    }
}

/// Draw a preview line/fibonacci while the user is placing the second point.
fn draw_preview(
    renderer: &mut CanvasRenderer,
    drawing: &DrawingInProgress,
    mouse: Point,
    st: &ChartState,
    visible_start: usize,
) {
    let Some(transform) = st.last_layout.price_transform else {
        return;
    };

    let chart_rect = Rect::new(
        st.config.margin.left,
        st.config.margin.top,
        st.config.width - st.config.margin.left - st.config.margin.right,
        st.config.height - st.config.margin.top - st.config.margin.bottom,
    );

    // Start point in visible-relative coordinates
    let rel_start_bar = drawing.start_bar - visible_start as f64;
    let start_pixel = transform.to_pixel(rel_start_bar, drawing.start_price);

    match st.draw_mode {
        DrawMode::TrendLine => {
            let style = LineStyle {
                color: Color::rgba(255, 255, 0, 180),
                width: 1.5,
            };
            renderer.draw_line(start_pixel, mouse, &style);
        }
        DrawMode::Fibonacci => {
            // Show horizontal lines at Fibonacci levels between start price and current mouse price
            let (_, mouse_price) = transform.to_data(mouse);
            let (high, low) = if drawing.start_price >= mouse_price {
                (drawing.start_price, mouse_price)
            } else {
                (mouse_price, drawing.start_price)
            };
            let range_val = high - low;
            let levels = [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0];

            for &level in &levels {
                let price = high - range_val * level;
                let y = transform.price_y(price);
                let alpha = if level < f64::EPSILON || (level - 1.0).abs() < f64::EPSILON {
                    180
                } else {
                    80
                };
                renderer.draw_line(
                    Point { x: chart_rect.x, y },
                    Point {
                        x: chart_rect.right(),
                        y,
                    },
                    &LineStyle {
                        color: Color::rgba(255, 165, 0, alpha),
                        width: 0.5,
                    },
                );
            }
            // Also draw vertical connection line
            renderer.draw_line(
                start_pixel,
                mouse,
                &LineStyle {
                    color: Color::rgba(255, 165, 0, 100),
                    width: 0.5,
                },
            );
        }
        DrawMode::Corridor => {
            let style = LineStyle {
                color: Color::rgba(0, 150, 255, 180),
                width: 1.0,
            };
            if let (Some(end_bar), Some(end_price)) = (drawing.end_bar, drawing.end_price) {
                // After second click: show main line + parallel preview at mouse position
                let rel_end = end_bar - visible_start as f64;
                let end_pixel = transform.to_pixel(rel_end, end_price);
                renderer.draw_line(start_pixel, end_pixel, &style);

                // Parallel line at mouse Y offset
                let (_, mouse_price) = transform.to_data(mouse);
                let offset = mouse_price - drawing.start_price;
                let p1 = transform.to_pixel(rel_start_bar, drawing.start_price + offset);
                let p2 = transform.to_pixel(rel_end, end_price + offset);
                renderer.draw_line(
                    p1,
                    p2,
                    &LineStyle {
                        color: Color::rgba(0, 150, 255, 100),
                        width: 1.0,
                    },
                );
            } else {
                // Before second click: show line from start to mouse
                renderer.draw_line(start_pixel, mouse, &style);
            }
        }
        DrawMode::None => {}
    }
}

/// Draw a panel-aware tooltip for the hovered bar.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn draw_tooltip(
    renderer: &mut CanvasRenderer,
    mouse: Point,
    data: &[Ohlcv],
    indicators: &[IndicatorOutput],
    markers: &[Marker],
    layout_info: &ChartLayoutInfo,
    config: &ChartConfig,
) {
    if data.is_empty() {
        return;
    }

    // Find which panel the mouse is in
    let active_panel = layout_info
        .panels
        .iter()
        .find(|p| mouse.y >= p.rect.y && mouse.y <= p.rect.bottom());

    // Reconstruct transform to map mouse X to bar index
    let chart_rect = Rect::new(
        config.margin.left,
        config.margin.top,
        config.width - config.margin.left - config.margin.right,
        config.height - config.margin.top - config.margin.bottom,
    );
    let inset = if data.len() > 1 {
        chart_rect.width / (data.len() - 1) as f64 * 0.5
    } else {
        0.0
    };
    let data_rect = Rect::new(
        chart_rect.x + inset,
        chart_rect.y,
        chart_rect.width - 2.0 * inset,
        chart_rect.height,
    );
    let price_range = PriceRange::from_ohlcv(data).unwrap_or(PriceRange::new(0.0, 100.0));
    let time_range = TimeRange::new(0, data.len());
    let vp = Viewport {
        rect: data_rect,
        time_range,
        price_range,
    };
    let transform = Transform::from_viewport(&vp);

    let (bar_f, _) = transform.to_data(mouse);
    let bar_idx = bar_f.round().clamp(0.0, (data.len() - 1) as f64) as usize;
    let bar = &data[bar_idx];

    // Build tooltip lines based on which panel the mouse is in
    let mut lines: Vec<String> = Vec::new();

    match active_panel.map(|p| &p.kind) {
        Some(PanelKind::Price) => {
            lines.push(format!(
                "O:{:.2}  H:{:.2}  L:{:.2}  C:{:.2}",
                bar.open, bar.high, bar.low, bar.close
            ));
            // Overlay indicators
            for output in indicators {
                if output.placement != IndicatorPlacement::Overlay {
                    continue;
                }
                let mut vals: Vec<String> = Vec::new();
                for series in &output.series {
                    if series.style_hint == SeriesStyle::HorizontalLine {
                        continue;
                    }
                    if bar_idx < series.values.len() && !series.values[bar_idx].is_nan() {
                        vals.push(format!("{:.2}", series.values[bar_idx]));
                    }
                }
                if !vals.is_empty() {
                    lines.push(format!("{}: {}", output.name, vals.join(" / ")));
                }
            }
            // Markers
            for m in markers {
                if m.bar_index == bar_idx && !m.label.is_empty() {
                    lines.push(format!(">>> {}", m.label));
                }
            }
        }
        Some(PanelKind::Volume) => {
            lines.push(format!("Vol: {}", format_vol(bar.volume)));
        }
        Some(PanelKind::Indicator(name)) => {
            if let Some(output) = indicators.iter().find(|o| &o.name == name) {
                let mut vals: Vec<String> = Vec::new();
                for series in &output.series {
                    if series.style_hint == SeriesStyle::HorizontalLine {
                        continue;
                    }
                    if bar_idx < series.values.len() && !series.values[bar_idx].is_nan() {
                        vals.push(format!("{}: {:.2}", series.name, series.values[bar_idx]));
                    }
                }
                if !vals.is_empty() {
                    lines.push(output.name.clone());
                    lines.extend(vals);
                }
            }
        }
        None => return,
    }

    if lines.is_empty() {
        return;
    }

    // Tooltip dimensions
    let font_size = config.font_size;
    let line_height = font_size + 4.0;
    let padding = 8.0;
    let tooltip_width = 260.0;
    let tooltip_height = lines.len() as f64 * line_height + padding * 2.0;

    // Position: avoid edges
    let tx = if mouse.x > config.width / 2.0 {
        mouse.x - tooltip_width - 15.0
    } else {
        mouse.x + 15.0
    };
    let ty = if mouse.y > config.height / 2.0 {
        mouse.y - tooltip_height - 10.0
    } else {
        mouse.y + 10.0
    };

    // Background
    renderer.draw_rect(
        Rect::new(tx, ty, tooltip_width, tooltip_height),
        &FillStyle {
            color: Color::rgba(22, 26, 37, 220),
        },
    );
    renderer.draw_rect_outline(
        Rect::new(tx, ty, tooltip_width, tooltip_height),
        &LineStyle {
            color: Color::GRAY,
            width: 0.5,
        },
    );

    // Text
    let text_style = TextStyle {
        color: Color::LIGHT_GRAY,
        size: font_size,
        font_family: "monospace".to_string(),
    };
    for (i, line) in lines.iter().enumerate() {
        renderer.draw_text(
            line,
            Point {
                x: tx + padding,
                y: ty + padding + (i as f64 + 1.0) * line_height - 2.0,
            },
            &text_style,
            TextAnchor::Start,
        );
    }
}

fn format_vol(vol: f64) -> String {
    if vol >= 1_000_000.0 {
        format!("{:.1}M", vol / 1_000_000.0)
    } else if vol >= 1_000.0 {
        format!("{:.1}K", vol / 1_000.0)
    } else {
        format!("{vol:.0}")
    }
}
