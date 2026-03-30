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
- [x] Crosshair: vertikale + horizontale Linie, folgt Maus (DPR-sync)
- [x] Responsive Canvas: skaliert mit Fenstergröße + devicePixelRatio
- [x] WASM-Package bauen: `wasm-pack build --target web`

---

## Phase 5 — Multi-Panel + Indikatoren ✅

### Multi-Panel Layout
- [x] Synchronisierter X-Zoom über alle Panels
- [x] Volume-Panel (Balken, grün/rot) mit Grid-Linien
- [x] Separate Y-Achse pro Panel mit Labels
- [x] Dynamische Panel-Gewichtung je nach Anzahl Sub-Panels
- [x] Panel-Legende (farbige Linien + Namen) im Preis-Panel
- [x] Panel-Labels in Volume/RSI/MACD
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

## Phase 7 — SMR-Integration & JS-API *(Priorität 1)*

### 7.1 TypeScript-Typen & Build
- [x] TypeScript-Typen generiert (automatisch via `wasm-bindgen`)
- [x] ES-Module Build (`--target web`)
- [x] `package.json` mit Build-Scripts (web, bundler, node)

### 7.2 SMR-Backend-Anbindung
- [x] `Ohlcv`, `Marker`, `MarkerShape`, `MarkerPosition` serde-fähig (opt-in Feature)
- [x] Git-Dependency Doku: `powerchart-core = { git = "...", features = ["serde"] }`
- [x] Axum-Endpoint Beispiel (`docs/integration/axum-endpoint.md`)

### 7.3 SMR-Frontend-Integration
- [x] React-Wrapper-Komponente dokumentiert (`docs/integration/react-wrapper.md`)
- [ ] SMR Pattern-Signale als Marker rendern (Candlestick-Patterns → Pfeile)
- [ ] npm-publish workflow (GitHub Actions)

---

## Phase 8 — Polish & Extras *(nice-to-have)*

### Mobile
- [ ] Touch-Events: Pinch-Zoom, Drag-Pan

### UI-Verfeinerungen
- [ ] Panel-Splitter: Drag zum Resize
- [ ] Rechts-Scroll über Daten hinaus (Future Space für Trendlinien)
- [ ] Y-Achse Drag-Skalierung (Preis-Range manuell anpassen)

### Native Desktop (optional)
- [ ] `winit`-Fenster als Host für den Chart
- [ ] `WgpuRenderer` oder `tiny-skia` auf CPU
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
| 6 | Pattern-Marker | ✅ |
| 7 | SMR-Integration & JS-API | ✅ |
| 8 | Polish & Extras | offen |

**152 Tests** (128 core + 24 render), Clippy-pedantic clean.
