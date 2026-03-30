use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use powerchart_core::indicator::{BollingerBands, Ema, Macd, Rsi, Sma};
use powerchart_core::interaction::{compute_pan, compute_zoom, is_in_chart_area};
use powerchart_core::{
    Indicator, IndicatorOutput, Ohlcv, Point, PriceRange, Rect, SeriesStyle, TimeRange, Transform,
    Viewport, ZoomPanState,
};
use powerchart_render::chart::{render_full_chart, ChartConfig};
use powerchart_render::style::{Color, FillStyle, LineStyle, TextAnchor, TextStyle};
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

            // Determine hovered bar index
            draw_tooltip(
                &mut renderer, mouse, visible_data, &outputs, &st.config,
            );
        }
    }
}

/// Draw a tooltip showing OHLCV + indicator values for the hovered bar.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn draw_tooltip(
    renderer: &mut CanvasRenderer,
    mouse: Point,
    data: &[Ohlcv],
    indicators: &[IndicatorOutput],
    config: &ChartConfig,
) {
    if data.is_empty() {
        return;
    }

    // Reconstruct transform to map mouse position back to bar index
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

    // Build tooltip lines
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "O:{:.2}  H:{:.2}  L:{:.2}  C:{:.2}",
        bar.open, bar.high, bar.low, bar.close
    ));
    lines.push(format!("Vol: {}", format_vol(bar.volume)));

    for output in indicators {
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
