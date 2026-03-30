//! `PowerChart` Core — data structures and layout engine.
//!
//! This crate contains no I/O and no external dependencies.

mod candle;
mod data;
mod geometry;
pub mod indicator;
pub mod interaction;
mod layout;
pub mod marker;
mod transform;
mod zoom;

pub use candle::CandleGeometry;
pub use data::{Ohlcv, PriceRange, Series, TimeRange};
pub use geometry::{Point, Rect};
pub use indicator::{Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries, SeriesStyle};
pub use layout::{Panel, PanelLayout};
pub use marker::{Marker, MarkerPosition, MarkerSet, MarkerShape};
pub use transform::{Transform, Viewport};
pub use zoom::ZoomPanState;
