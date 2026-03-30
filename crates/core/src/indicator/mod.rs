//! Technical indicator computations.
//!
//! All indicators implement the [`Indicator`] trait and are pure functions
//! (no I/O, no state between calls).

mod bollinger;
mod ema;
mod macd;
mod rsi;
mod sma;

pub use bollinger::BollingerBands;
pub use ema::Ema;
pub use macd::Macd;
pub use rsi::Rsi;
pub use sma::Sma;

use crate::Ohlcv;

/// Where an indicator should be rendered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IndicatorPlacement {
    /// Drawn on top of the price panel (SMA, EMA, Bollinger Bands).
    Overlay,
    /// Own sub-panel with a fixed Y range (e.g. RSI: 0–100).
    SubPanel { y_min: f64, y_max: f64 },
    /// Own sub-panel with auto-scaled Y range (e.g. MACD).
    SubPanelAuto,
}

/// Rendering hint for a series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeriesStyle {
    Line,
    Histogram,
    /// Horizontal reference line at a fixed value.
    HorizontalLine,
}

/// A single named output series from an indicator.
#[derive(Debug, Clone)]
pub struct IndicatorSeries {
    pub name: &'static str,
    pub values: Vec<f64>,
    pub style_hint: SeriesStyle,
}

/// Complete output of an indicator computation.
#[derive(Debug, Clone)]
pub struct IndicatorOutput {
    pub name: String,
    pub placement: IndicatorPlacement,
    pub series: Vec<IndicatorSeries>,
}

/// Core indicator trait. Stateless — takes data, returns output.
pub trait Indicator {
    fn name(&self) -> &'static str;
    fn placement(&self) -> IndicatorPlacement;
    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput;
}

/// Extract closing prices from OHLCV data.
fn closes(data: &[Ohlcv]) -> Vec<f64> {
    data.iter().map(|b| b.close).collect()
}

/// Compute a simple moving average over `values` with the given `period`.
/// Returns a `Vec<f64>` of the same length; first `period - 1` entries are `NAN`.
#[allow(clippy::cast_precision_loss)]
fn compute_sma(values: &[f64], period: usize) -> Vec<f64> {
    let n = values.len();
    let mut result = vec![f64::NAN; n];
    if period == 0 || period > n {
        return result;
    }

    let mut sum: f64 = values[..period].iter().sum();
    result[period - 1] = sum / period as f64;

    for i in period..n {
        sum += values[i] - values[i - period];
        result[i] = sum / period as f64;
    }
    result
}

/// Compute an exponential moving average over `values` with the given `period`.
/// First `period - 1` entries are `NAN`, entry at `period - 1` is the seed SMA.
#[allow(clippy::cast_precision_loss)]
fn compute_ema(values: &[f64], period: usize) -> Vec<f64> {
    let n = values.len();
    let mut result = vec![f64::NAN; n];
    if period == 0 || period > n {
        return result;
    }

    let k = 2.0 / (period as f64 + 1.0);
    // Seed with SMA
    let seed: f64 = values[..period].iter().sum::<f64>() / period as f64;
    result[period - 1] = seed;

    for i in period..n {
        result[i] = values[i] * k + result[i - 1] * (1.0 - k);
    }
    result
}
