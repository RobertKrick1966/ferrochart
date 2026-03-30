# PowerChart — Roadmap & Todo

## Phase 1 — Repo & Workspace ✅

### GitHub Setup
- [x] GitHub-Repo erstellen: `powerchart`
- [x] README.md mit Vision + API-Preview
- [x] MIT-Lizenz hinzufügen
- [x] `.gitignore` für Rust/Node

### Cargo Workspace
- [x] Cargo-Workspace anlegen mit Crates: `core`, `render`, `wasm`, `examples`
- [x] CI via GitHub Actions (`cargo test` + `clippy` + WASM build)

---

## Phase 2 — Core-Datenstrukturen ✅

### powerchart-core
> Keine I/O, keine externen Dependencies

- [x] `Ohlcv`, `Series<T>`, `PriceRange`, `TimeRange` Typen
- [x] `Viewport`, `Rect`, `Point`, `Transform` (Koordinaten-Mapping)
- [x] `PanelLayout` — Multi-Panel mit Gewichtung (z.B. 60/20/10/10)
- [x] `ZoomPanState` — Zoom-Level, sichtbarer Index-Range, Offset
- [x] `CandleGeometry` — Pixel-Koordinaten pro Kerze (x, open, close, high, low)
- [x] `interaction` — testbare Zoom/Pan/Hit-Test Logik
- [x] Unit-Tests für alle Layout-Berechnungen

---

## Phase 3 — Renderer-Trait + SVG-Backend ✅

### Renderer Abstraction
- [x] `Renderer`-Trait definieren: `draw_line`, `draw_rect`, `draw_text`, `draw_path`, `finish`
- [x] Style-Typen: `Color`, `LineStyle`, `FillStyle`, `TextStyle`, `TextAnchor`

### SVG Renderer (Test-Backend)
- [x] `SvgRenderer` implementiert `Renderer`-Trait
- [x] Candlestick-Rendering via SVG ausgeben (inkl. Volume-Panel)
- [x] Unit-Tests für SVG-Output
- [x] Achsen-Labels: X-Achse (Tag + Monat/Jahr), Y-Achse (Preis)

---

## Phase 4 — WASM Canvas-Renderer ✅

### WASM Setup
- [x] `wasm-pack` in Workspace integrieren
- [x] `PowerChart` WASM-Klasse: `new PowerChart(canvas)`
- [x] `setData(timestamps, opens, highs, lows, closes, volumes)`
- [x] `addIndicator(name, period)`, `clearIndicators()`
- [x] `resize(width, height)` für dynamische Größenanpassung

### Canvas Renderer
- [x] `CanvasRenderer` via `web-sys`: 2D Context API
- [x] `RequestAnimationFrame`-Loop (dirty-flag, nur bei Änderung rendern)
- [x] Console-Error-Panic-Hook für WASM-Debugging

### Interaktivität
- [x] Mouse-Events: Zoom (Scroll, zentriert auf Maus), Pan (Drag)
- [ ] Touch-Events für Mobile (Pinch-Zoom, Drag)
- [x] Crosshair: vertikale + horizontale Linie, folgt Maus
- [x] Responsive Canvas: skaliert mit Fenstergröße + devicePixelRatio
- [x] WASM-Package bauen: `wasm-pack build --target web`

---

## Phase 5 — Multi-Panel + Indikatoren ✅

### Multi-Panel Layout
- [x] Synchronisierter X-Zoom über alle Panels
- [x] Volume-Panel (Balken, grün/rot) mit Grid-Linien
- [x] Separate Y-Achse pro Panel mit Labels
- [x] Dynamische Panel-Gewichtung je nach Anzahl Sub-Panels
- [ ] Panel-Splitter: Drag zum Resize

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
- [x] Hover-Tooltip: OHLCV + alle aktiven Indikatoren
- [x] Tooltip-Positionierung (kein Clipping am Rand)

---

## Phase 6 — Pattern-Marker & Annotations *(1 Woche)*

### Marker-System
- [ ] `MarkerShape`: Pfeil oben/unten, Kreis, Raute
- [ ] `addMarker(timestamp, shape, color, label)`
- [ ] Marker-Tooltip bei Hover
- [ ] SMR-Integration: Pattern-Signale als Marker rendern
  > Pfeile für erkannte Candlestick-Patterns

---

## Phase 7 — JS-API & npm-Paket *(1 Woche)*

### Public API
- [ ] TypeScript-Typen generieren (`wasm-bindgen --typescript`)
- [ ] ES-Module + CommonJS Builds
- [x] Vanilla-JS Demo-Seite (`examples/web/`)
- [ ] `package.json`, npm-publish workflow

### SMR-Integration
- [ ] Git-Dependency in SMR `Cargo.toml` eintragen
  ```toml
  powerchart = { git = "https://github.com/RobertKrick1966/powerchart", features = ["wasm"] }
  ```
- [ ] React-Wrapper-Komponente für SMR-Frontend
- [ ] Axum-Endpoint gibt OHLCV + Indikatoren als JSON zurück
  > Backend unverändert, nur Datenformat sicherstellen

---

## Phase 8 — Native Desktop *(2–3 Wochen, optional)*

### Native Renderer
- [ ] `winit`-Fenster als Host für den Chart
- [ ] `WgpuRenderer` oder `tiny-skia` auf CPU (Entscheidung nach Bedarf)
- [ ] Keyboard-Shortcuts: `+`/`-` Zoom, Pfeiltasten Pan
- [ ] `examples/desktop/` — standalone Binary

---

## Zusammenfassung

| Phase | Inhalt | Status |
|---|---|---|
| 1 | Repo & Workspace | ✅ |
| 2 | Core-Datenstrukturen | ✅ |
| 3 | Renderer-Trait + SVG | ✅ |
| 4 | WASM Canvas-Renderer | ✅ |
| 5 | Multi-Panel + Indikatoren | ✅ |
| 6 | Pattern-Marker | offen |
| 7 | JS-API & npm-Paket | teilweise |
| 8 | Native Desktop (optional) | offen |

**143 Tests** (119 core + 24 render), Clippy-pedantic clean.
