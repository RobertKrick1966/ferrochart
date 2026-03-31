# ferrochart-render

Renderer trait and backend implementations.

## Module Overview

| Module | Types | Purpose |
|---|---|---|
| `renderer` | `Renderer` trait | Abstract drawing interface |
| `style` | `Color`, `LineStyle`, `FillStyle`, `TextStyle`, `TextAnchor` | Visual styling types |
| `svg` | `SvgRenderer` | SVG backend (test/export) |
| `chart` | `ChartConfig`, `render_candlestick_chart`, `render_with_volume` | High-level chart rendering |

## Renderer Trait

```rust
pub trait Renderer {
    fn draw_line(&mut self, from: Point, to: Point, style: &LineStyle);
    fn draw_rect(&mut self, rect: Rect, fill: &FillStyle);
    fn draw_rect_outline(&mut self, rect: Rect, style: &LineStyle);
    fn draw_text(&mut self, text: &str, pos: Point, style: &TextStyle, anchor: TextAnchor);
    fn draw_path(&mut self, points: &[Point], style: &LineStyle);
    fn set_background(&mut self, color: Color);
    fn finish(&self) -> Vec<u8>;
}
```

## Usage

```rust
use ferrochart_core::Ohlcv;
use ferrochart_render::{SvgRenderer, Renderer};
use ferrochart_render::chart::{render_candlestick_chart, render_with_volume, ChartConfig};

let config = ChartConfig::default();
let mut renderer = SvgRenderer::new(config.width, config.height);

// Simple candlestick chart
render_candlestick_chart(&mut renderer, &ohlcv_data, &config);

// Or with volume panel
render_with_volume(&mut renderer, &ohlcv_data, &config);

let svg_bytes = renderer.finish();
std::fs::write("chart.svg", &svg_bytes).unwrap();
```

## Chart Features

- Dark theme by default (configurable via `ChartConfig`)
- Y-axis grid lines with price labels
- X-axis date labels
- Bullish (green) / bearish (red) candle coloring
- Volume panel with `render_with_volume()`
- Multi-panel layout using `PanelLayout`

## Running the Example

```bash
cargo run --package ferrochart-examples
# Produces: candlestick.svg, candlestick_volume.svg
```

Open the SVG files in any browser to view the charts.
