# powerchart-core

Core data structures and layout engine. No I/O, no external dependencies.

## Module Overview

| Module | Types | Purpose |
|---|---|---|
| `data` | `Ohlcv`, `Series<T>`, `PriceRange`, `TimeRange` | Market data primitives |
| `geometry` | `Point`, `Rect` | Pixel-space primitives |
| `transform` | `Viewport`, `Transform` | Bidirectional data ↔ pixel coordinate mapping |
| `layout` | `PanelLayout`, `Panel` | Multi-panel vertical layout with weighted splitting |
| `zoom` | `ZoomPanState` | Zoom level, visible range, pan offset |
| `candle` | `CandleGeometry` | Pixel coordinates per candlestick |

## Key Design Decisions

- **All small types are `Copy`** — `Ohlcv`, `Point`, `Rect`, `PriceRange`, `TimeRange`, `Viewport`, `Transform`, `Panel`, `ZoomPanState`, `CandleGeometry`.
- **Immutable state** — `ZoomPanState::zoom()` and `::pan()` return new values instead of mutating.
- **Y-axis inversion** handled inside `Transform` — renderers receive pixel coords with Y=0 at top.
- **`Transform` precomputes** scale/offset for fast per-candle mapping.
- **`PanelLayout`** normalizes weights and distributes height proportionally minus gaps.

## Usage

```rust
use powerchart_core::*;

// Create a viewport
let vp = Viewport {
    rect: Rect::new(0.0, 0.0, 800.0, 600.0),
    time_range: TimeRange::new(0, 100),
    price_range: PriceRange::new(90.0, 210.0),
};

// Build transform for coordinate mapping
let transform = Transform::from_viewport(&vp);

// Compute candle geometry
let candles = CandleGeometry::compute_all(&ohlcv_data, 0, &transform, 0.8);

// Multi-panel layout
let layout = PanelLayout::new(&[60.0, 20.0, 20.0], total_rect, 2.0);
```
