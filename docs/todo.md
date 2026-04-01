# FerroChart -- Roadmap & Todo

> **Stand:** 2026-04-01 19:45 CEST
> **Tests:** 220 (177 core + 43 render), Clippy-pedantic clean

---

## Phase 1 -- Fundament ✅

### Repo & Workspace
- [x] GitHub-Repo, README, Lizenz, `.gitignore`
- [x] Cargo-Workspace: `core`, `render`, `wasm`, `examples`
- [x] CI via GitHub Actions (`cargo test` + `clippy` + WASM build)

### Core-Datenstrukturen (`ferrochart-core`)
- [x] `Ohlcv`, `Series<T>`, `PriceRange`, `TimeRange`
- [x] `Viewport`, `Rect`, `Point`, `Transform` (lineares Koordinaten-Mapping)
- [x] `PanelLayout` -- Multi-Panel mit Gewichtung
- [x] `ZoomPanState` -- Zoom-Level, sichtbarer Range, Offset, Future Space
- [x] `CandleGeometry` -- Pixel-Koordinaten pro Kerze (inkl. `institutional_ratio`)
- [x] `interaction` -- `compute_zoom()`, `compute_pan()`, `is_in_chart_area()` (pure functions)
- [x] `indicator` -- SMA, EMA, Bollinger Bands, RSI, MACD, VolumeSMA (alle `Indicator`-Trait)

### Renderer-Trait + SVG-Backend (`ferrochart-render`)
- [x] `Renderer`-Trait: `draw_line`, `draw_rect`, `draw_text`, `draw_path`, `draw_circle`, `fill_polygon`, `clip`, `restore_clip`, `finish`
- [x] Style-Typen: `Color`, `LineStyle`, `FillStyle`, `TextStyle`, `TextAnchor`
- [x] `SvgRenderer` implementiert `Renderer`-Trait (inkl. Clipping via `clipPath`)
- [x] Achsen-Labels: X-Achse (Tag + Monat/Jahr, auto-detect Daily/Hourly/Minute), Y-Achse (Preis)

### WASM Canvas-Renderer (`ferrochart-wasm`)
- [x] `FerroChart` WASM-Klasse mit `new(canvas)`, `setData()`, `resize()`
- [x] `CanvasRenderer` via `web-sys`: 2D Context API
- [x] `RequestAnimationFrame`-Loop (dirty-flag, nur bei Aenderung rendern)

### Interaktivitaet (WASM)
- [x] Mouse-Events: Zoom (Scroll, zentriert auf Maus), Pan (Drag), Keyboard (+/-/Pfeile/Home/End)
- [x] Touch-Events: Pinch-Zoom, Drag-Pan
- [x] Crosshair: vertikale + horizontale Linie, synchronisiert ueber alle Panels
- [x] Panel-spezifischer Hover-Tooltip (OHLCV / Volume / Indikator-Werte)
- [x] Y-Achse Drag-Skalierung (rechter Rand ziehen, Doppelklick = Reset)
- [x] Panel-Splitter: Drag auf Grenzlinie zwischen Panels
- [x] Future Space: Rechts-Scroll 33% ueber Daten hinaus

### Multi-Panel + Indikatoren
- [x] Synchronisierter X-Zoom ueber alle Panels
- [x] Volume-Panel (Balken gruen/rot), separate Y-Achse
- [x] Overlay: SMA, EMA, Bollinger (Preis-Panel Y-Range beruecksichtigt Overlay-Werte)
- [x] Sub-Panel: RSI (0-100, Overbought/Oversold-Linien), MACD (Signal + Histogramm)
- [x] Indikator-Berechnung einmal auf Gesamtdaten (Warmup), Cache + Slice
- [x] 8-Farben-Palette, Panel-Legenden, Panel-Clipping

### Marker & Annotations
- [x] `MarkerShape`: ArrowUp, ArrowDown, Circle (filled), Diamond
- [x] `MarkerSet` mit `in_range()` und `nearest()`, Marker-Info im Tooltip
- [x] `TrendLine` -- Linie zwischen zwei Punkten, `extend_right`
- [x] `Corridor` -- parallele Trendlinien mit Polygon-Fill
- [x] `FibonacciRetracement` -- 7 Standard-Levels mit Labels
- [x] Interaktives Zeichnen (2 Klicks TL/Fib, 3 Klicks Corridor)
- [x] `exportAnnotations()` / `importAnnotations(json)` -- JSON-Persistierung
- [x] Serde-Support (opt-in Feature) fuer alle Annotation-Typen

### Split Candles
- [x] `institutional_ratio` im `Ohlcv`-Typ (0.0-1.0)
- [x] Renderer zeichnet geteilte Bodies (institutional + retail Farbbereich)

### Themes & Build
- [x] Dark/Light Theme Presets (`setTheme("dark"/"light")`)
- [x] TypeScript-Typen (automatisch via `wasm-bindgen`)
- [x] ES-Module Build (`--target web`)
- [x] Vanilla-JS Demo (`examples/web/`)

---

### Logarithmische Y-Achse ✅
- [x] `YScaleMode` Enum (`Linear` / `Logarithmic`) in `Transform`
- [x] `from_viewport_with_mode()` -- Log-Mapping via `ln(price)`, Fallback auf Linear bei `min <= 0`
- [x] `log_y: bool` in `ChartConfig` -- nur Preis-Panel, Volume/Indikatoren bleiben linear
- [x] Y-Achsen-Labels gleichmaessig in Log-Space verteilt
- [x] `setLogScale(enabled)` WASM-API + Web-Demo Toggle
- [x] Round-Trip-Tests (`to_pixel` -> `to_data`), geometrischer Mittelwert = Bildmitte

### Realtime-API ✅
- [x] `updateLastCandle(timestamp, o, h, l, c, v)` -- in-place Update, kein Viewport-Reset
- [x] `pushCandle(timestamp, o, h, l, c, v)` -- neue Kerze, Auto-Scroll wenn am Ende

### DirtyFlags ✅
- [x] Layer-granulares Bitfield: `CANDLES | INDICATORS | ANNOTATIONS | OVERLAY`
- [x] Crosshair-Bewegung markiert nur `OVERLAY` (haeufigster Event)
- [x] Annotation-Edits markieren nur `ANNOTATIONS`
- [x] Indikator-Changes markieren `INDICATORS | CANDLES`

## Phase 1 -- Offen

| Feature | Beschreibung | Blockiert |
|---|---|---|
| Session-Separation | Pre/Regular/Post-Market Erkennung + visuelle Trennung | -- |
| Non-Uniform X-Achse | Fuer Alt-Bars (Renko, Heikin-Ashi etc.) | -- |

### API-Alignment ✅

- [x] `setConfig(json)` -- ChartConfig per JSON setzen (serde auf ChartConfig/Color/ChartMargin)
- [x] `setDataJson(json)` -- OHLCV-Daten als JSON-Array parallel zu Float64Array-Methode
- [x] `onWheel(deltaY, mouseX)` / `onPan(dx)` -- externe Event-Handler fuer Framework-Integration
- [x] `@ferrochart/web` TS-Wrapper -- NPM-Package mit `FerroChart.create()` Factory + rAF-Loop (`packages/web/`)
- [ ] Canvas2dRenderer verschieben -- bewusst in `wasm/` belassen (web-sys Dependency wuerde render-Crate WASM-abhaengig machen)

---

## Phase 2 -- SMR-Kern

### Implementiert ✅
- [x] **CUSUM Indikator** -- `Cusum { threshold }` als Sub-Panel (S+, S-, Event-Histogramm)
- [x] **Triple Barrier Overlay** -- `TripleBarrier` Annotation mit TP/SL-Linien, Time-Barrier, Polygon-Fill, Exit-Marker
- [x] **Imbalance Bar Coloring** -- Split-Candles via `institutional_ratio` (seit Phase 1)
- [x] **Anchored VWAP** -- `AnchoredVwap { anchor_bar }` Overlay, WASM: `addAnchoredVwap(bar)`
- [x] **Volume Profile** -- `VolumeProfile::compute(data, buckets)` mit proportionaler Volumen-Verteilung, horizontale Bars am rechten Rand des Preis-Panels
- [x] WASM-API: `addIndicator("cusum", period)`, `addTripleBarrier(...)`, `addAnchoredVwap(bar)`, `showVolumeProfile(buckets)`

---

## Phase 3 -- ML-Integration ✅

- [x] **Confidence Band** -- `ConfidenceBand { upper, lower, color, alpha }` Polygon-Fill auf Preis-Panel
- [x] **Walk-Forward Zones** -- `WalkForwardZone { start_bar, end_bar, is_train, label }` vertikale Farbzonen (blau=Train, orange=Val)
- [x] **News Event Overlay** -- `NewsEvent { bar_index, label, impact, urgency }` vertikale Linien + Labels, Farbe nach Impact, Alpha nach Urgency
- [x] WASM-API: `addConfidenceBand(upper, lower, r, g, b, alpha)`, `addWalkForwardZone(start, end, isTrain, label)`, `addNewsEvent(bar, label, impact, urgency)`

---

## Phase 4 -- Erweitert ✅

- [x] **GEX Profile** -- `HorizontalHistogram` Annotation, horizontale Bars am Preis-Panel (positive rechts, negative links)
- [x] **Max Pain** -- `HorizontalLevel` Annotation, horizontale Linie mit Label
- [x] **Multi-Chart Sync** -- `getZoomPanState()` / `setZoomPanState(visible, offset)` WASM-API fuer JS-seitige Synchronisierung
- [x] **Backtest Equity Curve** -- `EquityCurve` Indicator (Sub-Panel, kumulative P&L aus per-Bar Returns)
- [x] WASM-API: `addHorizontalHistogram(...)`, `addHorizontalLevel(...)`, `addEquityCurve(returns)`, `getZoomPanState()`, `setZoomPanState(visible, offset)`

---

## Backlog

- [ ] npm-publish workflow (GitHub Actions, auf Release)
- [ ] SMR Pattern-Signale als Marker durchschleifen
- [ ] `winit` Desktop-Fenster + `tiny-skia` CPU-Renderer (optional)
- [ ] `wgpu` GPU-Renderer (optional per Feature-Flag)

---

## Zusammenfassung

| Phase | Inhalt | Status |
|---|---|---|
| 1 | Fundament (Core + Render + WASM + Interaktion + Annotations) | ✅ (3 offene Punkte) |
| 2 | SMR-Kern (CUSUM, Triple Barrier, VWAP, Volume Profile, Imbalance) | ✅ |
| 3 | ML-Integration (Confidence Band, Walk-Forward, News Events) | ✅ |
| 4 | Erweitert (GEX, Max Pain, Multi-Chart Sync, Equity Curve) | ✅ |

### Strukturelle Basis fuer Phase 2+

Zwei Voraussetzungen die Phase 2 braucht sind bereits implementiert:

1. **Interaction Layer** -- `compute_zoom()`, `compute_pan()`, `is_in_chart_area()` existieren als pure Functions. WASM nutzt sie fuer Mouse/Touch/Keyboard. Anchored VWAP (Click-to-Anchor) kann darauf aufbauen.

2. **Marker/Annotation System** -- `TrendLine`, `Corridor`, `FibonacciRetracement`, `MarkerSet` sind vorhanden. Triple Barrier, CUSUM, Walk-Forward Zones und News Events koennen als neue Annotation/Marker-Typen hinzugefuegt werden.
