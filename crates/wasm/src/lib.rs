//! `FerroChart` WASM — WebAssembly bindings.

mod bindings;
mod canvas;
mod chart;

pub use canvas::CanvasRenderer;
pub use chart::FerroChart;
