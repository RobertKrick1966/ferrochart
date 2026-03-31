// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use ferrochart_core::indicator::{BollingerBands, Ema, Macd, Rsi, Sma};
use ferrochart_core::interaction::{compute_pan, compute_zoom, is_in_chart_area};
use ferrochart_core::{
    Annotations, Corridor, FibonacciRetracement, Indicator, IndicatorOutput, IndicatorPlacement, Marker,
    MarkerPosition, MarkerSet, MarkerShape, Ohlcv, Point, PriceRange, Rect, SeriesStyle,
    TimeRange, Transform, TrendLine, Viewport, ZoomPanState,
};
use ferrochart_render::chart::{render_full_chart_with_markers, ChartConfig, ChartLayoutInfo, PanelKind};
use ferrochart_render::style::{Color, FillStyle, LineStyle, TextAnchor, TextStyle};
use ferrochart_render::Renderer;

use crate::CanvasRenderer;

type RafClosure = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

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
    dirty: bool,
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
    #[allow(clippy::too_many_lines)]
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
            dirty: true,
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
                #[allow(clippy::cast_possible_truncation)]
                timestamp: timestamps[i] as i64,
                open: opens[i],
                high: highs[i],
                low: lows[i],
                close: closes[i],
                volume: volumes[i],
            })
            .collect();

        let mut st = self.state.borrow_mut();
        let total = data.len();
        st.data = data;
        let future = total / 3; // allow scrolling 33% past data
        st.zoom_pan = ZoomPanState::new(total, 100.min(total)).with_future_bars(future);
        st.recompute_indicators();
        st.dirty = true;
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
            "sma" => Box::new(Sma { period: period.unwrap_or(20) as usize }),
            "ema" => Box::new(Ema { period: period.unwrap_or(20) as usize }),
            "bollinger" => Box::new(BollingerBands {
                period: period.unwrap_or(20) as usize,
                std_dev: 2.0,
            }),
            "rsi" => Box::new(Rsi { period: period.unwrap_or(14) as usize }),
            "macd" => Box::new(Macd {
                fast_period: 12,
                slow_period: period.unwrap_or(26) as usize,
                signal_period: 9,
            }),
            _ => return Err(JsValue::from_str(&format!("unknown indicator: {name}"))),
        };

        let mut st = self.state.borrow_mut();
        st.indicators.push(indicator);
        st.recompute_indicators();
        st.dirty = true;
        Ok(())
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
        st.dirty = true;
    }

    /// Remove all indicators.
    #[wasm_bindgen(js_name = clearIndicators)]
    pub fn clear_indicators(&self) {
        let mut st = self.state.borrow_mut();
        st.indicators.clear();
        st.cached_outputs.clear();
        st.dirty = true;
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
        st.dirty = true;
        Ok(())
    }

    /// Remove all markers.
    #[wasm_bindgen(js_name = clearMarkers)]
    pub fn clear_markers(&self) {
        let mut st = self.state.borrow_mut();
        st.markers.clear();
        st.dirty = true;
    }

    /// Add a trendline between two bar/price points.
    #[wasm_bindgen(js_name = addTrendLine)]
    #[allow(clippy::too_many_arguments)]
    pub fn add_trend_line(
        &self,
        start_bar: f64, start_price: f64,
        end_bar: f64, end_price: f64,
        r: u8, g: u8, b: u8,
        extend_right: bool,
    ) {
        let mut st = self.state.borrow_mut();
        st.annotations.add_trend_line(TrendLine {
            start_bar, start_price,
            end_bar, end_price,
            color: (r, g, b),
            width: 1.5,
            extend_right,
        });
        st.dirty = true;
    }

    /// Add a Fibonacci retracement between a high and low point.
    #[wasm_bindgen(js_name = addFibonacci)]
    #[allow(clippy::too_many_arguments)]
    pub fn add_fibonacci(
        &self,
        high_bar: u32, high_price: f64,
        low_bar: u32, low_price: f64,
        r: u8, g: u8, b: u8,
    ) {
        let mut st = self.state.borrow_mut();
        st.annotations.add_fibonacci(FibonacciRetracement {
            high_bar: high_bar as usize,
            high_price,
            low_bar: low_bar as usize,
            low_price,
            color: (r, g, b),
        });
        st.dirty = true;
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
        st.dirty = true;
        Ok(())
    }

    /// Remove all annotations (trendlines, Fibonacci).
    #[wasm_bindgen(js_name = clearAnnotations)]
    pub fn clear_annotations(&self) {
        let mut st = self.state.borrow_mut();
        st.annotations.clear();
        st.dirty = true;
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
        st.dirty = true;
        Ok(())
    }

    /// Update the chart dimensions (call after canvas resize).
    pub fn resize(&self, width: u32, height: u32) {
        let mut st = self.state.borrow_mut();
        st.config.width = f64::from(width);
        st.config.height = f64::from(height);
        st.dirty = true;
    }
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

#[allow(clippy::too_many_lines)]
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
        }
        st.dirty = true;
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
        if st.draw_mode != DrawMode::None && pos.x < y_axis_left
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
                            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                            let (high_bar, high_price, low_bar, low_price) =
                                if start.start_price >= data_pos.1 {
                                    (start.start_bar as usize, start.start_price, data_pos.0 as usize, data_pos.1)
                                } else {
                                    (data_pos.0 as usize, data_pos.1, start.start_bar as usize, start.start_price)
                                };
                            st.annotations.add_fibonacci(FibonacciRetracement {
                                high_bar, high_price, low_bar, low_price,
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
            st.dirty = true;
            return;
        }

        if pos.x >= y_axis_left {
            // Click in Y-axis area → start Y-scale drag
            st.y_drag_active = true;
            st.y_drag_start_y = pos.y;
            st.y_drag_start_scale = st.price_scale;
        } else if let Some(panel_idx) = find_splitter_at_y(&st, pos.y) {
            // Click on a splitter gap → start splitter drag
            let num_sub = st.cached_outputs.iter()
                .filter(|o| o.placement != IndicatorPlacement::Overlay)
                .count();
            let weights = st.panel_weights.clone()
                .unwrap_or_else(|| {
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
            st.dirty = true;
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
        st.dirty = true;
    }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    canvas.add_event_listener_with_callback("mouseleave", on_mouseleave.as_ref().unchecked_ref())?;
    closures.push(on_mouseleave);

    Ok(())
}

#[allow(clippy::cast_precision_loss)]
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
        st.dirty = true;
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

#[allow(clippy::cast_precision_loss)]
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
                st.dirty = true;
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
        "touchstart", on_touchstart.as_ref().unchecked_ref(), &opts,
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
                st.dirty = true;
            }
        } else if touches.length() == 2 {
            // Pinch-zoom
            if let (Some(a), Some(b)) = (touches.get(0), touches.get(1)) {
                let dist = touch_distance(&a, &b);
                if st.pinch_start_dist > 1.0 {
                    let scale = dist / st.pinch_start_dist;
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let new_visible = (st.pinch_start_visible as f64 / scale)
                        .round()
                        .clamp(5.0, st.zoom_pan.total_bars as f64) as usize;
                    // Keep centered
                    let mid = st.zoom_pan.offset + st.zoom_pan.visible_bars / 2;
                    let new_offset = mid.saturating_sub(new_visible / 2);
                    st.zoom_pan = ZoomPanState {
                        visible_bars: new_visible,
                        offset: new_offset,
                        total_bars: st.zoom_pan.total_bars, future_bars: 0,
                    };
                    // Clamp
                    st.zoom_pan = st.zoom_pan.pan(0);
                    st.dirty = true;
                }
            }
        }
    }) as Box<dyn FnMut(web_sys::TouchEvent)>);
    canvas.add_event_listener_with_callback_and_add_event_listener_options(
        "touchmove", on_touchmove.as_ref().unchecked_ref(), &opts,
    )?;
    closures.push(on_touchmove);

    // touchend / touchcancel
    let s = Rc::clone(state);
    let on_touchend = Closure::wrap(Box::new(move |_e: web_sys::TouchEvent| {
        let mut st = s.borrow_mut();
        st.is_dragging = false;
        st.mouse_pos = None;
        st.dirty = true;
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
                st.dirty = true;
            }
            "ArrowRight" => {
                e.prevent_default();
                st.zoom_pan = st.zoom_pan.pan(3);
                st.dirty = true;
            }
            "ArrowUp" => {
                e.prevent_default();
                st.price_scale = (st.price_scale - 0.1).clamp(0.1, 10.0);
                st.dirty = true;
            }
            "ArrowDown" => {
                e.prevent_default();
                st.price_scale = (st.price_scale + 0.1).clamp(0.1, 10.0);
                st.dirty = true;
            }
            "+" | "=" => {
                e.prevent_default();
                let mid = st.zoom_pan.offset + st.zoom_pan.visible_bars / 2;
                st.zoom_pan = st.zoom_pan.zoom(1.25, mid);
                st.dirty = true;
            }
            "-" => {
                e.prevent_default();
                let mid = st.zoom_pan.offset + st.zoom_pan.visible_bars / 2;
                st.zoom_pan = st.zoom_pan.zoom(0.8, mid);
                st.dirty = true;
            }
            "Escape" => {
                // Cancel drawing
                st.draw_mode = DrawMode::None;
                st.drawing = None;
                st.dirty = true;
            }
            "Home" => {
                e.prevent_default();
                st.zoom_pan = ZoomPanState {
                    offset: 0,
                    ..st.zoom_pan
                };
                st.dirty = true;
            }
            "End" => {
                e.prevent_default();
                st.zoom_pan = st.zoom_pan.scroll_to_end();
                st.dirty = true;
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
            if st.dirty {
                st.dirty = false;
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
#[allow(clippy::cast_precision_loss)]
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
    let num_sub = st.cached_outputs.iter()
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

    // Slice cached indicator outputs for visible range
    let outputs: Vec<IndicatorOutput> = st.cached_outputs
        .iter()
        .map(|out| out.slice(start..end))
        .collect();

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

    // Apply Y-axis scale factor, panel weights, and bar slot count
    st.config.price_scale = st.price_scale;
    st.config.panel_weights = st.panel_weights.clone();
    st.config.visible_offset = start;
    // If scrolled into future space, there are fewer data bars than visible slots
    st.config.visible_bar_slots = if visible_data.len() < st.zoom_pan.visible_bars {
        Some(st.zoom_pan.visible_bars)
    } else {
        None
    };

    let layout_info = render_full_chart_with_markers(&mut renderer, visible_data, &outputs, &marker_refs, &st.annotations, &st.config);
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
                Point { x: mouse.x, y: chart_top },
                Point { x: mouse.x, y: chart_bottom },
                &crosshair_style,
            );
            renderer.draw_line(
                Point { x: chart_left, y: mouse.y },
                Point { x: chart_right, y: mouse.y },
                &crosshair_style,
            );

            draw_tooltip(
                &mut renderer, mouse, visible_data, &outputs, &adjusted_markers,
                &layout_info, &st.config,
            );

            // Drawing preview
            if let Some(ref drawing) = st.drawing {
                draw_preview(&mut renderer, drawing, mouse, st, start);
            }
        }
    }
}

/// Draw a preview line/fibonacci while the user is placing the second point.
#[allow(clippy::cast_precision_loss)]
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
                let alpha = if level < f64::EPSILON || (level - 1.0).abs() < f64::EPSILON { 180 } else { 80 };
                renderer.draw_line(
                    Point { x: chart_rect.x, y },
                    Point { x: chart_rect.right(), y },
                    &LineStyle {
                        color: Color::rgba(255, 165, 0, alpha),
                        width: 0.5,
                    },
                );
            }
            // Also draw vertical connection line
            renderer.draw_line(start_pixel, mouse, &LineStyle {
                color: Color::rgba(255, 165, 0, 100),
                width: 0.5,
            });
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
                renderer.draw_line(p1, p2, &LineStyle {
                    color: Color::rgba(0, 150, 255, 100),
                    width: 1.0,
                });
            } else {
                // Before second click: show line from start to mouse
                renderer.draw_line(start_pixel, mouse, &style);
            }
        }
        DrawMode::None => {}
    }
}

/// Draw a panel-aware tooltip for the hovered bar.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::too_many_lines)]
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
    let active_panel = layout_info.panels.iter().find(|p| {
        mouse.y >= p.rect.y && mouse.y <= p.rect.bottom()
    });

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
    let vp = Viewport { rect: data_rect, time_range, price_range };
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
        &FillStyle { color: Color::rgba(22, 26, 37, 220) },
    );
    renderer.draw_rect_outline(
        Rect::new(tx, ty, tooltip_width, tooltip_height),
        &LineStyle { color: Color::GRAY, width: 0.5 },
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