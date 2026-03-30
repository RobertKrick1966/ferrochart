use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use powerchart_core::indicator::{BollingerBands, Ema, Macd, Rsi, Sma};
use powerchart_core::interaction::{compute_pan, compute_zoom, is_in_chart_area};
use powerchart_core::{Indicator, IndicatorOutput, Ohlcv, Point, ZoomPanState};
use powerchart_render::chart::{render_full_chart, ChartConfig};
use powerchart_render::style::{Color, LineStyle};
use powerchart_render::Renderer;

use crate::CanvasRenderer;

type RafClosure = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

/// Internal mutable state shared between the chart and event closures.
struct ChartState {
    canvas: HtmlCanvasElement,
    data: Vec<Ohlcv>,
    config: ChartConfig,
    zoom_pan: ZoomPanState,
    indicators: Vec<Box<dyn Indicator>>,
    /// Cached indicator outputs computed on the full dataset.
    cached_outputs: Vec<IndicatorOutput>,
    mouse_pos: Option<Point>,
    is_dragging: bool,
    drag_start_x: f64,
    drag_start_offset: usize,
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
pub struct PowerChart {
    state: Rc<RefCell<ChartState>>,
    _closures: Vec<Closure<dyn FnMut(web_sys::MouseEvent)>>,
    _wheel_closure: Option<Closure<dyn FnMut(web_sys::WheelEvent)>>,
    _raf_closure: RafClosure,
}

#[wasm_bindgen]
impl PowerChart {
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
    pub fn new(canvas: &HtmlCanvasElement) -> Result<PowerChart, JsValue> {
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
            mouse_pos: None,
            is_dragging: false,
            drag_start_x: 0.0,
            drag_start_offset: 0,
            dirty: true,
        }));

        let mut closures: Vec<Closure<dyn FnMut(web_sys::MouseEvent)>> = Vec::new();
        attach_mouse_events(canvas, &state, &mut closures)?;
        let on_wheel = attach_wheel_event(canvas, &state)?;
        let raf_handle = start_render_loop(&state);

        Ok(PowerChart {
            state,
            _closures: closures,
            _wheel_closure: Some(on_wheel),
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
        st.zoom_pan = ZoomPanState::new(total, 100.min(total));
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

    /// Remove all indicators.
    #[wasm_bindgen(js_name = clearIndicators)]
    pub fn clear_indicators(&self) {
        let mut st = self.state.borrow_mut();
        st.indicators.clear();
        st.cached_outputs.clear();
        st.dirty = true;
    }

    /// Update the chart dimensions (call after canvas resize).
    pub fn resize(&self, width: u32, height: u32) {
        let mut st = self.state.borrow_mut();
        st.config.width = f64::from(width);
        st.config.height = f64::from(height);
        st.dirty = true;
    }
}

/// Helper: get mouse position relative to canvas.
fn mouse_pos(e: &web_sys::MouseEvent, canvas: &HtmlCanvasElement) -> Point {
    let rect = canvas.get_bounding_client_rect();
    Point {
        x: f64::from(e.client_x()) - rect.left(),
        y: f64::from(e.client_y()) - rect.top(),
    }
}

fn attach_mouse_events(
    canvas: &HtmlCanvasElement,
    state: &Rc<RefCell<ChartState>>,
    closures: &mut Vec<Closure<dyn FnMut(web_sys::MouseEvent)>>,
) -> Result<(), JsValue> {
    // Mouse move (crosshair + drag)
    let s = Rc::clone(state);
    let on_mousemove = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
        let mut st = s.borrow_mut();
        let pos = mouse_pos(&e, &st.canvas);
        st.mouse_pos = Some(pos);

        if st.is_dragging {
            let dx = pos.x - st.drag_start_x;
            let chart_width = st.config.width - st.config.margin.left - st.config.margin.right;
            st.zoom_pan = compute_pan(st.zoom_pan, dx, chart_width, st.drag_start_offset);
        }
        st.dirty = true;
    }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    canvas.add_event_listener_with_callback("mousemove", on_mousemove.as_ref().unchecked_ref())?;
    closures.push(on_mousemove);

    // Mouse down (start drag)
    let s = Rc::clone(state);
    let on_mousedown = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
        let mut st = s.borrow_mut();
        let pos = mouse_pos(&e, &st.canvas);
        st.is_dragging = true;
        st.drag_start_x = pos.x;
        st.drag_start_offset = st.zoom_pan.offset;
    }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    canvas.add_event_listener_with_callback("mousedown", on_mousedown.as_ref().unchecked_ref())?;
    closures.push(on_mousedown);

    // Mouse up (stop drag)
    let s = Rc::clone(state);
    let on_mouseup = Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
        s.borrow_mut().is_dragging = false;
    }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    canvas.add_event_listener_with_callback("mouseup", on_mouseup.as_ref().unchecked_ref())?;
    closures.push(on_mouseup);

    // Mouse leave (hide crosshair)
    let s = Rc::clone(state);
    let on_mouseleave = Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
        let mut st = s.borrow_mut();
        st.mouse_pos = None;
        st.is_dragging = false;
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
        let mouse_x = f64::from(e.client_x()) - rect.left();
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

    // Return the Rc — keeps the closure alive as long as PowerChart exists
    raf_closure
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

    render_full_chart(&mut renderer, visible_data, &outputs, &st.config);

    // Crosshair
    if let Some(mouse) = st.mouse_pos {
        let chart_left = st.config.margin.left;
        let chart_right = st.config.width - st.config.margin.right;
        let chart_top = st.config.margin.top;
        let chart_bottom = st.config.height - st.config.margin.bottom;

        if is_in_chart_area(mouse, chart_left, chart_right, chart_top, chart_bottom) {
            let style = LineStyle {
                color: Color::rgba(200, 200, 200, 100),
                width: 0.5,
            };
            renderer.draw_line(
                Point { x: mouse.x, y: chart_top },
                Point { x: mouse.x, y: chart_bottom },
                &style,
            );
            renderer.draw_line(
                Point { x: chart_left, y: mouse.y },
                Point { x: chart_right, y: mouse.y },
                &style,
            );
        }
    }
}
