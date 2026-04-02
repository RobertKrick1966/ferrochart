// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

//! `FerroChart` Core — data structures and layout engine.
//!
//! This crate contains no I/O and no external dependencies.

/// Chart annotation types (trend lines, Fibonacci retracements, corridors).
pub mod annotation;
mod candle;
mod data;
/// Level-of-Detail decimation for large datasets.
pub mod decimation;
mod geometry;
/// Technical indicator computations (SMA, EMA, RSI, MACD, Bollinger Bands).
pub mod indicator;
/// Mouse and keyboard interaction helpers (zoom, pan, hit-testing).
pub mod interaction;
mod layout;
/// Chart markers (buy/sell signals, pattern labels).
pub mod marker;
mod transform;
mod zoom;

/// Re-exported annotation types.
pub use annotation::{
    Annotations, BarrierOutcome, ConfidenceBand, Corridor, FibonacciRetracement,
    HorizontalHistogram, HorizontalLevel, NewsEvent, TrendLine, TripleBarrier, WalkForwardZone,
};
/// Re-exported candlestick geometry.
pub use candle::CandleGeometry;
/// Re-exported data primitives.
pub use data::{Ohlcv, PriceRange, Series, TimeRange};
/// Re-exported geometry primitives.
pub use geometry::{Point, Rect};
/// Re-exported indicator types.
pub use indicator::{Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle};
/// Re-exported layout types.
pub use layout::{Panel, PanelLayout};
/// Re-exported marker types.
pub use marker::{Marker, MarkerPosition, MarkerSet, MarkerShape};
/// Re-exported coordinate transform types.
pub use transform::{Transform, Viewport, YScaleMode};
/// Re-exported zoom/pan state.
pub use zoom::ZoomPanState;
