# ferrochart-render

> **Stand:** 2026-04-01 17:00 CEST

Renderer trait, SVG-Backend und High-Level Chart-Rendering.

## Module Overview

| Module | Types | Purpose |
|---|---|---|
| `renderer` | `Renderer` trait | Abstrakte Zeichenschnittstelle (11 Methoden) |
| `style` | `Color`, `LineStyle`, `FillStyle`, `TextStyle`, `TextAnchor` | Visual styling types |
| `svg` | `SvgRenderer` | SVG-Backend (Test/Export) |
| `chart` | `ChartConfig`, `ChartLayoutInfo`, `PanelInfo`, `PanelKind` | High-Level Chart-Rendering |

## Renderer Trait

```rust
pub trait Renderer {
    fn draw_line(&mut self, from: Point, to: Point, style: &LineStyle);
    fn draw_rect(&mut self, rect: Rect, fill: &FillStyle);
    fn draw_rect_outline(&mut self, rect: Rect, style: &LineStyle);
    fn draw_text(&mut self, text: &str, pos: Point, style: &TextStyle, anchor: TextAnchor);
    fn draw_path(&mut self, points: &[Point], style: &LineStyle);
    fn draw_circle(&mut self, center: Point, radius: f64, fill: &FillStyle);
    fn fill_polygon(&mut self, points: &[Point], fill: &FillStyle);
    fn set_background(&mut self, color: Color);
    fn clip(&mut self, rect: Rect);
    fn restore_clip(&mut self);
    fn finish(&self) -> Vec<u8>;
}
```

Zwei Implementierungen:
- **`SvgRenderer`** (in `ferrochart-render`) -- erzeugt SVG-Dokument als `Vec<u8>`
- **`CanvasRenderer`** (in `ferrochart-wasm`) -- zeichnet direkt auf HTML Canvas 2D Context

> **Soll-Architektur:** `CanvasRenderer` soll nach `ferrochart-render/src/canvas2d.rs` verschoben
> werden hinter Feature-Flag `canvas2d`. Details: [wasm-api.md](wasm-api.md)

## Rendering Functions

| Function | Beschreibung |
|---|---|
| `render_candlestick_chart` | Nur Preis-Panel (Candles + Achsen) |
| `render_with_volume` | Preis + Volume-Panel |
| `render_full_chart` | Preis + Volume + Indikatoren |
| `render_full_chart_with_markers` | Vollstaendiges Chart mit Markers + Annotations |

## Implementierte Chart-Features

### Candlestick-Rendering
- Bullish (gruen) / Bearish (rot) Candle-Farben
- **Split Candles** -- `institutional_ratio` teilt den Body in zwei Farbbereiche (institutional + retail)
- Wicks (High-Low-Linien)

### Multi-Panel Layout
- Preis-Panel, Volume-Panel, beliebig viele Indikator-Sub-Panels
- Custom Panel-Gewichte ueber `ChartConfig::panel_weights`
- Panel-Clipping (Inhalte bluten nicht in andere Panels)
- Panel-Border-Outlines

### Achsen
- **Y-Achse**: Preis-Grid-Linien mit Labels, pro Panel eigene Achse
- **X-Achse**: Auto-Detection Daily/Hourly/Minute; Tag + Monat/Jahr Labels
- Volume-Achse mit K/M-Suffixen

### Indikatoren
- **Overlay** (auf Preis-Panel): SMA, EMA, Bollinger Bands (3 Serien: Upper/Middle/Lower)
- **Sub-Panel**: RSI (0-100, Overbought/Oversold-Referenzlinien), MACD (Signal + Histogramm)
- 8-Farben-Palette, Panel-Legenden mit Namen + Farbe
- Histogramm-Rendering fuer MACD

### Markers
- Shapes: ArrowUp, ArrowDown, Circle (filled), Diamond
- Position: AboveBar / BelowBar
- Labels unter/ueber dem Marker
- Marker-Info im Hover-Tooltip

### Annotations
- **Trendlines** -- Linie zwischen zwei Data-Space-Punkten, optional `extend_right` bis Chartrand
- **Corridors** -- Zwei parallele Trendlinien mit semi-transparentem Polygon-Fill dazwischen
- **Fibonacci Retracements** -- 7 horizontale Levels (0%, 23.6%, 38.2%, 50%, 61.8%, 78.6%, 100%) mit Labels
- Annotations nutzen absolute Bar-Indices; Renderer subtrahiert `config.visible_offset`

### Themes
- `ChartConfig::dark()` (Default) -- dunkler Hintergrund
- `ChartConfig::light()` -- heller Hintergrund

## WASM-spezifische Features (ferrochart-wasm)

Diese Features existieren nur im interaktiven WASM-Chart (`FerroChart`), nicht im statischen SVG-Renderer:

| Feature | Beschreibung |
|---|---|
| Crosshair | Vertikale + horizontale Linie, folgt Maus, synchronisiert ueber alle Panels |
| Tooltip | Panel-spezifisch (Preis: OHLCV, Volume: Volumen, Indikator: Werte) |
| Zoom | Maus-Scroll (zentriert auf Mausposition), Ctrl+Scroll, +/- Tasten |
| Pan | Drag horizontal, Pfeiltasten links/rechts, Home/End |
| Y-Achse Skalierung | Drag im rechten Rand, Doppelklick = Reset |
| Panel-Splitter | Drag auf Grenzlinie zwischen Panels aendert Gewichte |
| Touch | Pinch-Zoom, Drag-Pan |
| Zeichenwerkzeuge | Trendline (2 Klicks), Fibonacci (2 Klicks), Corridor (3 Klicks) |
| Annotations-Persistierung | `exportAnnotations()` -> JSON, `importAnnotations(json)` |
| Realtime | `updateLastCandle()` (in-place), `pushCandle()` (Auto-Scroll wenn am Ende) |
| Log Y-Achse | `setLogScale(enabled)` -- Preis-Panel logarithmisch, Volume/Indikatoren linear |
| DirtyFlags | Layer-granular (CANDLES/INDICATORS/ANNOTATIONS/OVERLAY) -- Crosshair nur OVERLAY |
| Future Space | Rechts-Scroll 33% ueber Daten hinaus |

## Noch nicht implementiert

| Feature | Beschreibung |
|---|---|
| Volume Profile Histogram | Horizontales Volumen-Profil am rechten Rand |
| Anchored VWAP | Click-to-Anchor VWAP-Linie |
| Triple Barrier Overlay | Take-Profit/Stop-Loss/Time-Barrier Visualisierung |
| CUSUM State Sub-Pane | Separates Panel fuer CUSUM-State |
| Imbalance Bar Coloring | Farb-Encoding basierend auf Order-Flow-Imbalance (Split-Candles vorhanden, aber kein Imbalance-spezifisches Coloring) |
| News Event Overlay | Zeitpunkt-Marker fuer Nachrichten |
| ONNX Confidence Overlay | ML-Confidence als Band/Overlay |
| Walk-Forward Zones | Zeitbereiche fuer Train/Test-Splits |
| GEX Profile | Gamma Exposure Profil |
| Max Pain | Options Max-Pain Level |
| Multi-Chart Sync | Synchronisierter Zoom/Pan ueber mehrere Chart-Instanzen |
| Backtest Equity Curve | Equity-Kurve als Sub-Panel |

## Usage

```rust
use ferrochart_core::{Ohlcv, Annotations, TrendLine};
use ferrochart_render::{SvgRenderer, Renderer};
use ferrochart_render::chart::{ChartConfig, render_full_chart_with_markers};

let config = ChartConfig::default();
let mut renderer = SvgRenderer::new(config.width, config.height);

let mut annotations = Annotations::new();
annotations.add_trend_line(TrendLine {
    start_bar: 5.0, start_price: 100.0,
    end_bar: 25.0, end_price: 130.0,
    color: (255, 235, 59), width: 2.0, extend_right: true,
});

render_full_chart_with_markers(
    &mut renderer, &ohlcv_data, &indicators, &markers, &annotations, &config,
);

let svg_bytes = renderer.finish();
std::fs::write("chart.svg", &svg_bytes).unwrap();
```

## Running the Examples

```bash
cargo run --package ferrochart-examples
# Erzeugt SVGs in output/:
#   01_candlestick.svg   04_sma_overlay.svg   07_annotations.svg
#   02_volume.svg        05_volume_sma.svg
#   03_split_candle.svg  06_markers.svg
```
