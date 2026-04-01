// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

//! `FerroChart` WASM — WebAssembly bindings.

mod bindings;
mod canvas;
mod chart;

/// Canvas 2D rendering backend for browsers.
pub use canvas::CanvasRenderer;
/// Interactive candlestick chart rendered on an HTML canvas.
pub use chart::FerroChart;
