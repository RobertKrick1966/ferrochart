// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

//! `FerroChart` Render — renderer trait and backend implementations.

pub mod chart;
mod renderer;
pub mod style;
mod svg;

pub use renderer::Renderer;
pub use svg::SvgRenderer;