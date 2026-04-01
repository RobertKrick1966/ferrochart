# FerroChart -- Roadmap & Todo

> **Stand:** 2026-04-01 17:00 CEST
> **Tests:** 191 (151 core + 40 render), Clippy-pedantic clean

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

### API-Alignment (siehe `docs/wasm-api.md` fuer Details)

| Feature | Beschreibung | Prioritaet |
|---|---|---|
| `set_config(json)` | ChartConfig per JSON setzen | Mittel |
| Canvas2dRenderer verschieben | von `wasm/canvas.rs` nach `render/canvas2d.rs` + Feature-Flag | Mittel |
| JSON-basierte Daten-API | `set_data(json)` parallel zu Float64Array-Methode | Niedrig |
| `@ferrochart/web` TS-Wrapper | NPM-Package mit `FerroChart.create()` Factory + rAF-Loop | Niedrig |
| Externe Event-Handler | `on_wheel()`/`on_pan()` als Alternative zu internen Handlern | Niedrig |

---

## Phase 2 -- SMR-Kern (nicht begonnen)

| Feature | Beschreibung | Abhaengigkeit |
|---|---|---|
| Volume Profile Histogram | Horizontales Volumen-Profil (z.B. rechter Rand oder Overlay) | Neuer Indikator-Typ |
| Anchored VWAP | Click-to-Anchor VWAP-Linie, live berechnet ab Ankerpunkt | Interaction Layer (vorhanden) |
| Triple Barrier Overlay | Take-Profit / Stop-Loss / Time-Barrier Visualisierung | Annotation-System (vorhanden) |
| CUSUM Event Marker | CUSUM-State als Marker + eigenes Sub-Pane | Marker-System (vorhanden), neuer Indikator |
| Imbalance Bar Coloring | Farb-Encoding basierend auf Order-Flow-Imbalance | Split-Candle-Infrastruktur (vorhanden) |

---

## Phase 3 -- ML-Integration (nicht begonnen)

| Feature | Beschreibung | Abhaengigkeit |
|---|---|---|
| ONNX Confidence Overlay | ML-Confidence als Band/Overlay auf Preis-Panel | Neuer Overlay-Typ |
| Walk-Forward Boundary Zones | Zeitbereiche fuer Train/Test-Splits markieren | Annotation-System (vorhanden) |
| News Event Overlay | Zeitpunkt-Marker fuer Nachrichten/Events | Marker-System (vorhanden) |

---

## Phase 4 -- Erweitert (nicht begonnen)

| Feature | Beschreibung | Abhaengigkeit |
|---|---|---|
| GEX Profile | Gamma Exposure Profil (horizontales Histogramm) | Aehnlich Volume Profile |
| Max Pain | Options Max-Pain Level als horizontale Linie | Einfaches Overlay |
| Multi-Chart Sync | Synchronisierter Zoom/Pan ueber mehrere Chart-Instanzen | JS-Layer / Event-Bus |
| Backtest Equity Curve | Equity-Kurve als Sub-Panel | Neuer Panel-Typ |

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
| 2 | SMR-Kern (Volume Profile, VWAP, Triple Barrier, CUSUM) | -- |
| 3 | ML-Integration (ONNX, Walk-Forward, News) | -- |
| 4 | Erweitert (GEX, Max Pain, Multi-Chart, Backtest) | -- |

### Strukturelle Basis fuer Phase 2+

Zwei Voraussetzungen die Phase 2 braucht sind bereits implementiert:

1. **Interaction Layer** -- `compute_zoom()`, `compute_pan()`, `is_in_chart_area()` existieren als pure Functions. WASM nutzt sie fuer Mouse/Touch/Keyboard. Anchored VWAP (Click-to-Anchor) kann darauf aufbauen.

2. **Marker/Annotation System** -- `TrendLine`, `Corridor`, `FibonacciRetracement`, `MarkerSet` sind vorhanden. Triple Barrier, CUSUM, Walk-Forward Zones und News Events koennen als neue Annotation/Marker-Typen hinzugefuegt werden.
