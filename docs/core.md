# ferrochart-core

> **Stand:** 2026-04-02 CEST

Core data structures, indicators, and layout engine. No I/O, no external dependencies.

## Module Overview

| Module | Types | Purpose |
|---|---|---|
| `data` | `Ohlcv`, `Series<T>`, `PriceRange`, `TimeRange` | Market data primitives (`Ohlcv` includes `institutional_ratio` for split-body candles, `PriceRange::from_closes` for line/area charts) |
| `geometry` | `Point`, `Rect` | Pixel-space primitives |
| `transform` | `Viewport`, `Transform`, `YScaleMode` | Bidirectional data ↔ pixel mapping (linear + logarithmic). Inverse: `pixel_y_to_price()`, `pixel_x_to_bar()` |
| `layout` | `PanelLayout`, `Panel` | Multi-panel vertical layout with weighted splitting |
| `zoom` | `ZoomPanState` | Zoom level, visible range, pan offset, future space |
| `candle` | `CandleGeometry` | Pixel coordinates per candlestick (carries `institutional_ratio` for split rendering) |
| `interaction` | `compute_zoom`, `compute_pan`, `is_in_chart_area` | Pure zoom/pan/hit-test functions (no state, no I/O) |
| `chart_type` | `ChartType`, `RenkoBar`, `PFColumn`, `PFDirection` | Chart type enum + `compute_heikin_ashi()`, `compute_renko()`, `compute_point_figure()` |
| `annotation` | `TrendLine`, `Corridor`, `FibonacciRetracement`, `Ray`, `Ellipse`, `AndrewsPitchfork`, `GannFan`, `MeasurementTool`, ... | 12 drawing tools + ML-Overlays (Triple Barrier, Confidence Band, Walk-Forward Zone, News Event, Horizontal Histogram/Level) |
| `marker` | `Marker`, `MarkerSet`, `MarkerShape`, `MarkerPosition` | Pattern markers (ArrowUp, ArrowDown, Circle, Diamond) |
| `indicator` | 22 Indikatoren — siehe unten | Alle implementieren `Indicator`-Trait |
| `decimation` | `min_max_decimate`, `lttb_decimate`, `decimate_target` | LOD: O(n) OHLCV-Gruppierung, Largest-Triangle für Linien, Auto-Erkennung |

## Indicator Library (22)

### Overlay
SMA, EMA, Bollinger Bands, Donchian Channels, Keltner Channels, Parabolic SAR, Supertrend, Ichimoku Cloud, Session VWAP, Anchored VWAP

### Sub-Panel
RSI, MACD, ATR, OBV, Stochastic (%K/%D), Williams %R, CCI, ADX/DMI, Volume SMA, CUSUM

### ML/SMR-spezifisch
Equity Curve, Volume Profile, Triple Barrier (als Annotation)

## Chart Types (7)

| Typ | Funktion | Zeitachse |
|---|---|---|
| Candlestick | Standard OHLC | uniform |
| Heikin-Ashi | `compute_heikin_ashi()` | uniform |
| Line | Close-only Linie | uniform |
| Area | Close-only gefüllt | uniform |
| OHLC Bars | Balken statt Kerzen | uniform |
| Renko | `compute_renko(data, brick_size)` → `Vec<RenkoBar>` | non-uniform |
| Point & Figure | `compute_point_figure(data, box_size, reversal)` → `Vec<PFColumn>` | non-uniform |

## Drawing Tools (12)

### Priority 1 (Grundbedürfnis)
TrendLine, FibonacciRetracement, Corridor, HorizontalRay, VerticalLine, RectangleZone, TextLabel

### Priority 2 (Advanced)
Ray, MeasurementTool, Ellipse, AndrewsPitchfork, GannFan

## ML/SMR-Annotations

| Typ | Beschreibung |
|---|---|
| `TripleBarrier` | Entry/TP/SL/Window-Fenster mit Outcome-Label |
| `ConfidenceBand` | Upper/Lower Float64Array mit Alpha-Fill |
| `WalkForwardZone` | Train/Validation-Segment mit Label |
| `NewsEvent` | Impact (-1..1) + Urgency (1..5) auf Bar |
| `HorizontalHistogram` | GEX/Options-Profil (Werte + Preise) |
| `HorizontalLevel` | Max-Pain, Strike-Preise mit Label |
| `EquityCurve` | Returns → kumulative Equity als Sub-Panel |

## Design Principles

- **Alle kleinen Typen sind `Copy`** — `Ohlcv`, `Point`, `Rect`, `PriceRange`, `TimeRange`, `Viewport`, `Transform`, `Panel`, `ZoomPanState`, `CandleGeometry`
- **Immutable State** — `ZoomPanState::zoom()` und `::pan()` geben neue Werte zurück
- **Y-Achsen-Inversion** in `Transform` behandelt — Renderer bekommen Pixel-Coords mit Y=0 oben
- **`Transform` precomputed** scale/offset für schnelles per-Candle-Mapping
- **`PanelLayout`** normalisiert Gewichte und verteilt Höhe proportional abzüglich Gaps
- **`Indicator`-Trait** — `compute(&[Ohlcv]) -> IndicatorOutput` mit `IndicatorPlacement` (Overlay / SubPanel / SubPanelAuto) und `SeriesStyle` (Line / Histogram / HorizontalLine)
- **Serde hinter Feature-Flag** — `#[cfg_attr(feature = "serde", derive(...))]` auf allen Annotation-Typen und `ChartType`

## Noch nicht implementiert

| Feature | Beschreibung |
|---|---|
| Session-Separation | Pre/Regular/Post-Market-Zonen |
| Plugin-System | Custom Indicators von außen registrieren (Trait-basiert) |
| Ichimoku Cloud-Fill | `fill_polygon` zwischen Senkou A/B |
| Price Channel | Parallele Trendlinien durch Highs/Lows |
| Edit-Modus | Zeichnungen selektieren, verschieben, löschen |
| Snap-to-OHLC | Zeichenpunkte rasten auf H/L/O/C ein |
| Undo/Redo | Für Zeichnungen |

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

// Indicators
let sma = indicator::Sma { period: 20 };
let output = sma.compute(&ohlcv_data);  // → IndicatorOutput

// Renko chart
let bricks = compute_renko(&ohlcv_data, 2.0);

// Annotations
let mut annotations = Annotations::new();
annotations.add_trend_line(TrendLine {
    start_bar: 5.0, start_price: 100.0,
    end_bar: 25.0, end_price: 130.0,
    color: (255, 235, 59), width: 2.0, extend_right: true,
});
annotations.add_pitchfork(AndrewsPitchfork {
    bar1: 2.0, price1: 95.0,
    bar2: 10.0, price2: 110.0,
    bar3: 18.0, price3: 100.0,
    color: (255, 165, 0), width: 1.5,
});
```
