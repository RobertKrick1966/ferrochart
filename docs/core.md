# ferrochart-core

> **Stand:** 2026-04-01 17:00 CEST

Core data structures and layout engine. No I/O, no external dependencies.

## Module Overview

| Module | Types | Purpose |
|---|---|---|
| `data` | `Ohlcv`, `Series<T>`, `PriceRange`, `TimeRange` | Market data primitives (`Ohlcv` includes `institutional_ratio` for split-body candles) |
| `geometry` | `Point`, `Rect` | Pixel-space primitives |
| `transform` | `Viewport`, `Transform`, `YScaleMode` | Bidirectional data <-> pixel coordinate mapping (linear + logarithmic) |
| `layout` | `PanelLayout`, `Panel` | Multi-panel vertical layout with weighted splitting |
| `zoom` | `ZoomPanState` | Zoom level, visible range, pan offset, future space |
| `candle` | `CandleGeometry` | Pixel coordinates per candlestick (carries `institutional_ratio` for split rendering) |
| `interaction` | `compute_zoom`, `compute_pan`, `is_in_chart_area` | Testbare Zoom/Pan/Hit-Test-Logik (pure functions, kein State) |
| `annotation` | `TrendLine`, `Corridor`, `FibonacciRetracement`, `Annotations` | Chart annotations mit serde-Support (opt-in Feature) |
| `marker` | `Marker`, `MarkerSet`, `MarkerShape`, `MarkerPosition` | Pattern markers auf Candles (ArrowUp, ArrowDown, Circle, Diamond) |
| `indicator` | `Sma`, `Ema`, `BollingerBands`, `Rsi`, `Macd`, `VolumeSma` | Technical indicators (alle implementieren `Indicator`-Trait) |

## Implementiert

- **Alle kleinen Typen sind `Copy`** -- `Ohlcv`, `Point`, `Rect`, `PriceRange`, `TimeRange`, `Viewport`, `Transform`, `Panel`, `ZoomPanState`, `CandleGeometry`
- **Immutable State** -- `ZoomPanState::zoom()` und `::pan()` geben neue Werte zurueck
- **Y-Achsen-Inversion** in `Transform` behandelt -- Renderer bekommen Pixel-Coords mit Y=0 oben
- **`Transform` precomputed** scale/offset fuer schnelles per-Candle-Mapping
- **`PanelLayout`** normalisiert Gewichte und verteilt Hoehe proportional abzueglich Gaps
- **Interaction Layer** -- `compute_zoom()`, `compute_pan()`, `is_in_chart_area()` als pure Functions (kein State, kein I/O), genutzt vom WASM-Layer fuer Mouse/Touch/Keyboard
- **Marker-System** -- `MarkerSet` mit `in_range()` und `nearest()` fuer effizientes Abfragen
- **Annotations** -- `TrendLine`, `Corridor`, `FibonacciRetracement` in Data-Space-Koordinaten; serde-Support hinter Feature-Flag; `Annotations`-Container mit `add_*()`, `clear()`, `is_empty()`
- **Institutional Split** -- `institutional_ratio` (0.0-1.0) im `Ohlcv`-Typ, durchgereicht bis `CandleGeometry`, Renderer zeichnet geteilte Candle-Bodies

## Noch nicht implementiert

| Feature | Beschreibung |
| Session-Separation | Keine Pre/Regular/Post-Market-Erkennung |
| Non-Uniform X-Achse | X-Achse geht von zeitlicher Aequidistanz aus (nur Candlestick-Bars) |
| Volume Profile | Nur `VolumeSma` vorhanden, kein horizontales Volume-Profil |
| Anchored VWAP | Nicht implementiert |
| Triple Barrier | Nicht implementiert |
| CUSUM State Sub-Pane | Generische Marker vorhanden, kein spezifischer CUSUM-State |
| ONNX Confidence | Kein ML-Model-Layer |
| Walk-Forward Zones | Nicht implementiert |
| GEX / Max Pain | Nicht implementiert |
| Multi-Chart Sync | Kein Sync-Mechanismus zwischen Chart-Instanzen |

## Usage

```rust
use ferrochart_core::*;

// Create a viewport
let vp = Viewport {
    rect: Rect::new(0.0, 0.0, 800.0, 600.0),
    time_range: TimeRange::new(0, 100),
    price_range: PriceRange::new(90.0, 210.0),
};

// Build transform for coordinate mapping
let transform = Transform::from_viewport(&vp);

// Compute candle geometry (with split candle support)
let candles = CandleGeometry::compute_all(&ohlcv_data, 0, &transform, 0.8);

// Multi-panel layout
let layout = PanelLayout::new(&[60.0, 20.0, 20.0], total_rect, 2.0);

// Interaction (pure functions)
let new_state = compute_zoom(zoom_pan, mouse_x, chart_left, chart_width, delta_y);
let new_state = compute_pan(zoom_pan, dx, chart_width, drag_start_offset);

// Annotations
let mut annotations = Annotations::new();
annotations.add_trend_line(TrendLine {
    start_bar: 5.0, start_price: 100.0,
    end_bar: 25.0, end_price: 130.0,
    color: (255, 235, 59), width: 2.0, extend_right: true,
});
```
