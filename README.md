# FerroChart

High-performance financial charting library built in Rust, targeting WebAssembly and native platforms.

## Features

- Candlestick charts with volume panel
- Technical indicators: SMA, EMA, Bollinger Bands, RSI, MACD, Volume SMA
- Pattern markers (arrows, circles, diamonds)
- Interactive: zoom (scroll), pan (drag), crosshair, tooltip
- Y-axis scaling (drag on right margin, double-click to reset)
- Panel splitter (drag between panels to resize)
- Touch support: pinch-zoom, drag-pan
- Responsive: auto-scales with window size
- 60fps rendering via requestAnimationFrame (dirty-flag optimization)
- 152 unit tests, clippy-pedantic clean

## Quick Start

### From Source

```bash
# Prerequisites: Rust toolchain, wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Clone and build
git clone https://github.com/RobertKrick1966/ferrochart.git
cd ferrochart
wasm-pack build crates/wasm --target web

# Run the demo
python3 -m http.server 8080
# Open http://localhost:8080/examples/web/index.html
```

### JavaScript Usage

```html
<canvas id="chart" width="900" height="700"></canvas>
<script type="module">
import init, { FerroChart } from './pkg/ferrochart_wasm.js';

await init();

const chart = new FerroChart(document.getElementById('chart'));

// Set OHLCV data (parallel Float64Arrays)
chart.setData(timestamps, opens, highs, lows, closes, volumes);

// Add indicators
chart.addIndicator('sma', 20);
chart.addIndicator('ema', 10);
chart.addIndicator('bollinger', 20);
chart.addIndicator('rsi', 14);
chart.addIndicator('macd', 26);
chart.addIndicator('volsma', 20);

// Add pattern markers
chart.addMarker(42, 'arrow_up', 'below', 0, 200, 0, 'Hammer');
chart.addMarker(58, 'arrow_down', 'above', 200, 0, 0, 'Shooting Star');

// Handle resize
window.addEventListener('resize', () => {
  const dpr = window.devicePixelRatio || 1;
  canvas.width = Math.round(canvas.clientWidth * dpr);
  canvas.height = Math.round(canvas.clientHeight * dpr);
  chart.resize(canvas.width, canvas.height);
});
</script>
```

### Rust (Backend Integration)

```toml
# In your Cargo.toml
[dependencies]
ferrochart-core = { git = "https://github.com/RobertKrick1966/ferrochart", features = ["serde"] }
```

```rust
use ferrochart_core::Ohlcv;
// Ohlcv implements Serialize/Deserialize with the "serde" feature
let data: Vec<Ohlcv> = fetch_from_database();
let json = serde_json::to_string(&data)?;
```

## API Reference

### `FerroChart` (WASM/JavaScript)

| Method | Description |
|---|---|
| `new FerroChart(canvas)` | Create interactive chart on canvas element |
| `setData(ts, o, h, l, c, v)` | Set OHLCV data from parallel `Float64Array`s |
| `addIndicator(name, period?)` | Add indicator: `"sma"`, `"ema"`, `"bollinger"`, `"rsi"`, `"macd"`, `"volsma"` |
| `removeIndicator(name)` | Remove indicator by name (e.g. `"sma"`) |
| `clearIndicators()` | Remove all indicators |
| `addMarker(idx, shape, pos, r, g, b, label)` | Add marker: shapes `"arrow_up"`, `"arrow_down"`, `"circle"`, `"diamond"` |
| `clearMarkers()` | Remove all markers |
| `addTrendLine(startBar, startPrice, endBar, endPrice, r, g, b, extendRight)` | Draw a trendline |
| `addFibonacci(highBar, highPrice, lowBar, lowPrice, r, g, b)` | Draw Fibonacci retracement levels |
| `clearAnnotations()` | Remove all trendlines and Fibonacci |
| `setDrawMode(mode)` | Interactive drawing: `"trendline"`, `"fibonacci"`, `"none"` |
| `setTheme(name)` | Switch theme: `"dark"` (default) or `"light"` |
| `resize(width, height)` | Update chart dimensions after canvas resize |

### Interactions

| Action | Effect |
|---|---|
| Scroll wheel | Zoom in/out (centered on cursor) |
| Click + drag (chart) | Pan left/right |
| Click + drag (right margin) | Scale Y-axis up/down |
| Double-click (right margin) | Reset Y-axis to auto |
| Drag between panels | Resize panels |
| Hover | Crosshair + tooltip (panel-specific) |
| Pinch (touch) | Zoom in/out |
| Single touch drag | Pan |

## Workspace Structure

| Crate | Description |
|---|---|
| `ferrochart-core` | Data structures, indicators, layout, coordinate transforms |
| `ferrochart-render` | Renderer trait + SVG/Canvas backends |
| `ferrochart-wasm` | WebAssembly bindings, event handling, interactive chart |
| `ferrochart-examples` | Example applications |

## Building

```bash
# Run tests
cargo test --workspace --exclude ferrochart-wasm

# Clippy
cargo clippy --workspace --exclude ferrochart-wasm --all-targets -- -D warnings
cargo clippy --package ferrochart-wasm --target wasm32-unknown-unknown -- -D warnings

# Build WASM
wasm-pack build crates/wasm --target web        # for <script type="module">
wasm-pack build crates/wasm --target bundler     # for Webpack/Vite
wasm-pack build crates/wasm --target nodejs      # for Node.js

# Generate SVG examples
cargo run --package ferrochart-examples
```

## Integration Guides

- [Axum Backend Endpoint](docs/integration/axum-endpoint.md)
- [React Wrapper Component](docs/integration/react-wrapper.md)

## License

AGPL-3.0-or-later
