// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

//! `FerroChart` WASM — WebAssembly bindings.

mod bindings;
mod canvas;
mod chart;

pub use canvas::CanvasRenderer;
pub use chart::FerroChart;