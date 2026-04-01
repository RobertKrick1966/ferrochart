// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

//! `FerroChart` Render — renderer trait and backend implementations.

/// Chart layout, configuration, and rendering functions.
pub mod chart;
mod renderer;
/// Visual style primitives (colors, line styles, text styles).
pub mod style;
mod svg;

/// Re-export of the abstract rendering trait.
pub use renderer::Renderer;
/// Re-export of the SVG rendering backend.
pub use svg::SvgRenderer;
