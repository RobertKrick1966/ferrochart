//! `PowerChart` Render — renderer trait and backend implementations.

pub mod chart;
mod renderer;
pub mod style;
mod svg;

pub use renderer::Renderer;
pub use svg::SvgRenderer;
