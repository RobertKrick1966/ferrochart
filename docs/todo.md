# FerroChart — Roadmap & Todo

## Phase 1 — Repo & Workspace ✅

### GitHub Setup
- [x] GitHub-Repo erstellen: `ferrochart`
- [x] README.md mit Vision + API-Preview
- [x] MIT-Lizenz hinzufügen
- [x] `.gitignore` für Rust/Node

### Cargo Workspace
- [x] Cargo-Workspace anlegen mit Crates: `core`, `render`, `wasm`, `examples`
- [x] CI via GitHub Actions (`cargo test` + `clippy` + WASM build)

---

## Phase 2 — Core-Datenstrukturen ✅

### ferrochart-core
> Keine I/O, keine externen Dependencies (nur opt-in serde)

- [x] `Ohlcv`, `Series<T>`, `PriceRange`, `TimeRange` Typen
- [x] `Viewport`, `Rect`, `Point`, `Transform` (Koordinaten-Mapping)
- [x] `PanelLayout` — Multi-Panel mit Gewichtung
- [x] `ZoomPanState` — Zoom-Level, sichtbarer Index-Range, Offset, Future Space
- [x] `CandleGeometry` — Pixel-Koordinaten pro Kerze
- [x] `interaction` — testbare Zoom/Pan/Hit-Test Logik
- [x] `indicator` — SMA, EMA, Bollinger Bands, RSI, MACD, VolumeSMA mit Indicator-Trait
- [x] `marker` — MarkerShape, MarkerPosition, MarkerSet
- [x] Serde-Support (opt-in Feature) für Ohlcv, Marker-Typen

---

## Phase 3 — Renderer-Trait + SVG-Backend ✅

### Renderer Abstraction
- [x] `Renderer`-Trait: `draw_line`, `draw_rect`, `draw_text`, `draw_path`, `clip`, `restore_clip`, `finish`
- [x] Style-Typen: `Color`, `LineStyle`, `FillStyle`, `TextStyle`, `TextAnchor`

### SVG Renderer (Test-Backend)
- [x] `SvgRenderer` implementiert `Renderer`-Trait (inkl. Clipping via `clipPath`)
- [x] Candlestick-Rendering via SVG (inkl. Volume-Panel)
- [x] Unit-Tests für SVG-Output
- [x] Achsen-Labels: X-Achse (Tag + Monat/Jahr), Y-Achse (Preis)

---

## Phase 4 — WASM Canvas-Renderer ✅

### WASM Setup
- [x] `wasm-pack` in Workspace integrieren
- [x] `FerroChart` WASM-Klasse: `new FerroChart(canvas)`
- [x] `setData(timestamps, opens, highs, lows, closes, volumes)`
- [x] `addIndicator(name, period)`, `removeIndicator(name)`, `clearIndicators()`
- [x] `addMarker(...)`, `clearMarkers()`
- [x] `resize(width, height)` für dynamische Größenanpassung

### Canvas Renderer
- [x] `CanvasRenderer` via `web-sys`: 2D Context API (inkl. `clip`/`restore`)
- [x] `RequestAnimationFrame`-Loop (dirty-flag, nur bei Änderung rendern)
- [x] Console-Error-Panic-Hook für WASM-Debugging

### Interaktivität
- [x] Mouse-Events: Zoom (Scroll, zentriert auf Maus), Pan (Drag)
- [x] Touch-Events: Pinch-Zoom, Drag-Pan
- [x] Crosshair: vertikale + horizontale Linie, folgt Maus (DPR-sync)
- [x] Y-Achse Drag-Skalierung (rechter Rand ziehen, Doppelklick = Reset)
- [x] Panel-Splitter: Drag auf Grenzlinie zwischen Panels
- [x] Responsive Canvas: skaliert mit Fenstergröße + devicePixelRatio
- [x] WASM-Package bauen: `wasm-pack build --target web`

---

## Phase 5 — Multi-Panel + Indikatoren ✅

### Multi-Panel Layout
- [x] Synchronisierter X-Zoom über alle Panels
- [x] Volume-Panel (Balken, grün/rot) mit Grid-Linien
- [x] Separate Y-Achse pro Panel mit Labels
- [x] Dynamische Panel-Gewichtung je nach Anzahl Sub-Panels
- [x] Custom Panel-Weights über ChartConfig
- [x] Panel-Legende (farbige Linien + Namen) im Preis-Panel
- [x] Panel-Labels in Volume/RSI/MACD
- [x] Panel-Clipping (Inhalte bluten nicht in andere Panels)

### Indikatoren
- [x] `Indicator`-Trait: berechnet aus `&[Ohlcv]`, gibt `IndicatorOutput` zurück
- [x] Overlay-Indikatoren: SMA, EMA, Bollinger Bands (auf Preis-Panel)
- [x] Sub-Panel: RSI (0–100, Overbought/Oversold-Linien, Grid)
- [x] Sub-Panel: MACD (Linie + Signal + Histogramm, Grid)
- [x] Preis-Panel Y-Range berücksichtigt Overlay-Indikator-Werte
- [x] Indikator-Berechnung einmal auf Gesamtdaten (Warmup), Cache + Slice
- [x] 8-Farben-Palette für Indikatoren
- [x] `IndicatorOutput::slice()` für sichtbaren Bereich

### Tooltip
- [x] Hover-Tooltip: panel-spezifisch (nur relevante Daten pro Panel)
- [x] Tooltip-Positionierung (kein Clipping am Rand)

---

## Phase 6 — Pattern-Marker & Annotations ✅

### Marker-System
- [x] `MarkerShape`: ArrowUp, ArrowDown, Circle, Diamond
- [x] `addMarker(barIndex, shape, position, r, g, b, label)`
- [x] `clearMarkers()`
- [x] `MarkerSet` mit `in_range()` und `nearest()` (9 Tests)
- [x] Marker-Rendering: Shapes auf Price-Panel, Labels darunter/darüber
- [x] Marker-Info im Hover-Tooltip

---

## Phase 7 — SMR-Integration & JS-API ✅

### TypeScript-Typen & Build
- [x] TypeScript-Typen generiert (automatisch via `wasm-bindgen`)
- [x] ES-Module Build (`--target web`)
- [x] `package.json` mit Build-Scripts (web, bundler, node)

### SMR-Backend-Anbindung
- [x] `Ohlcv`, `Marker`, `MarkerShape`, `MarkerPosition` serde-fähig (opt-in Feature)
- [x] Git-Dependency Doku: `ferrochart-core = { git = "...", features = ["serde"] }`
- [x] Axum-Endpoint Beispiel (`docs/integration/axum-endpoint.md`)

### SMR-Frontend-Integration
- [x] React-Wrapper-Komponente dokumentiert (`docs/integration/react-wrapper.md`)
- [x] Vanilla-JS Demo-Seite (`examples/web/`)
- [ ] npm-publish workflow (GitHub Actions)

---

## Phase 8 — Polish & Extras ✅

### Mobile
- [x] Touch-Events: Pinch-Zoom, Drag-Pan

### UI-Verfeinerungen
- [x] Panel-Splitter: Drag zum Resize (Gewichte live anpassen)
- [x] Rechts-Scroll über Daten hinaus (Future Space, 33% vom Datenbereich)
- [x] Y-Achse Drag-Skalierung (Drag im rechten Rand, Doppelklick = Reset)
- [x] Panel-Clipping (Inhalte bleiben in ihrem Panel)
- [x] `removeIndicator(name)` API

### Dokumentation
- [x] README: Quick Start, API Reference, Interactions, Build-Anleitung
- [x] Integration Guides: Axum, React

---

## Backlog — Zukünftige Features

### Native Desktop (optional)
- [ ] `winit`-Fenster als Host für den Chart
- [ ] `tiny-skia` als CPU-Renderer (Default), `wgpu` optional per Feature-Flag
- [ ] Keyboard-Shortcuts: `+`/`-` Zoom, Pfeiltasten Pan
- [ ] `examples/desktop/` — standalone Binary

### Weitere Verbesserungen
- [x] npm-publish workflow (GitHub Actions, auf Release)
- [ ] SMR Pattern-Signale als Marker durchschleifen
- [x] Trendlinien-Zeichnung (Linie zwischen zwei Punkten, extend_right)
- [x] Fibonacci-Retracement (7 Standard-Levels, Labels + Linien)
- [x] Zeitachse: Stunden/Minuten-Ticks für Intraday-Daten (auto-detect)
- [x] Dark/Light Theme Presets (`setTheme("dark"/"light")`)

---

## Zusammenfassung

| Phase | Inhalt | Status |
|---|---|---|
| 1 | Repo & Workspace | ✅ |
| 2 | Core-Datenstrukturen | ✅ |
| 3 | Renderer-Trait + SVG | ✅ |
| 4 | WASM Canvas-Renderer | ✅ |
| 5 | Multi-Panel + Indikatoren | ✅ |
| 6 | Pattern-Marker | ✅ |
| 7 | SMR-Integration & JS-API | ✅ |
| 8 | Polish & Extras | ✅ |

**160 Tests** (133 core + 27 render), Clippy-pedantic clean.
