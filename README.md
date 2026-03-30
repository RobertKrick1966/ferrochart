# PowerChart

High-performance financial charting library built in Rust, targeting WebAssembly and native platforms.

## Vision

PowerChart delivers GPU-grade candlestick charts at 60 fps — as a lightweight WASM module or native binary. Multi-panel layouts, technical indicators, and pattern markers come built-in.

## API Preview

```js
import { PowerChart } from "powerchart";

const chart = new PowerChart(canvas, {
  panels: [
    { weight: 60, series: "candlestick" },
    { weight: 20, series: "volume" },
    { weight: 20, series: "rsi", params: { period: 14 } },
  ],
});

chart.setData(ohlcvData);
chart.addIndicator("ema", { period: 20, color: "#2196f3" });
chart.addMarker({ timestamp: 1710000000, shape: "arrow_up", label: "Hammer" });
```

## Workspace Structure

| Crate | Description |
|---|---|
| `powerchart-core` | Data structures, layout engine, coordinate transforms |
| `powerchart-render` | Renderer trait + backend implementations (SVG, Canvas) |
| `powerchart-wasm` | WebAssembly bindings via `wasm-bindgen` |
| `powerchart-examples` | Example applications |

## License

MIT
