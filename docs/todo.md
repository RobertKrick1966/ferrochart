# PowerChart — Roadmap & Todo

## Phase 1 — Repo & Workspace *(1–2 Tage)*

### GitHub Setup
- [x] GitHub-Repo erstellen: `powerchart`
- [x] README.md mit Vision + API-Preview
- [x] MIT-Lizenz hinzufügen
- [x] `.gitignore` für Rust/Node

### Cargo Workspace
- [x] Cargo-Workspace anlegen mit Crates: `core`, `render`, `wasm`, `examples`
- [x] CI via GitHub Actions (`cargo test` + `clippy`)

---

## Phase 2 — Core-Datenstrukturen *(1 Woche)*

### powerchart-core
> Keine I/O, keine externen Dependencies

- [x] `Ohlcv`, `Series<T>`, `PriceRange`, `TimeRange` Typen
- [x] `Viewport`, `Rect`, `Point`, `Transform` (Koordinaten-Mapping)
- [x] `PanelLayout` — Multi-Panel mit Gewichtung (z.B. 60/20/10/10)
- [x] `ZoomPanState` — Zoom-Level, sichtbarer Index-Range, Offset
- [x] `CandleGeometry` — Pixel-Koordinaten pro Kerze (x, open, close, high, low)
- [x] Unit-Tests für alle Layout-Berechnungen (66 Tests)

---

## Phase 3 — Renderer-Trait + SVG-Backend *(1 Woche)*

### Renderer Abstraction
- [ ] `Renderer`-Trait definieren: `draw_line`, `draw_rect`, `draw_text`, `draw_path`, `flush`
- [ ] Style-Typen: `LineStyle`, `FillStyle`, `TextStyle`

### SVG Renderer (Test-Backend)
> Für TDD — kein WASM nötig, SVG einfach im Browser öffnen

- [ ] `SvgRenderer` implementiert `Renderer`-Trait
- [ ] Candlestick-Rendering via SVG ausgeben
- [ ] Snapshot-Tests: SVG-Output gegen Referenz-Dateien
- [ ] Achsen-Labels: X-Achse (Zeit), Y-Achse (Preis)

---

## Phase 4 — WASM Canvas-Renderer *(3–4 Wochen)*

### WASM Setup
- [ ] `wasm-pack` in Workspace integrieren
- [ ] PowerChart WASM-Bindung: `new PowerChart(canvas, config)`
- [ ] `setData(data: OhlcvArray)`, `addIndicator(name, params)`

### Canvas Renderer
- [ ] `CanvasRenderer` via `web-sys`: 2D Context API
- [ ] `tiny-skia` als Rasterizer einbinden (WASM-kompatibel)
- [ ] Font-Rendering via `fontdue` — ASCII reicht für v0.1
- [ ] `RequestAnimationFrame`-Loop für 60fps

### Interaktivität
- [ ] Mouse/Pointer-Events: Zoom (Scroll), Pan (Drag)
- [ ] Touch-Events für Mobile
- [ ] Crosshair: vertikale + horizontale Linie, folgt Maus
- [ ] WASM-Package bauen: `wasm-pack build --target bundler`

---

## Phase 5 — Multi-Panel + Indikatoren *(2–3 Wochen)*

### Multi-Panel Layout
- [ ] Synchronisierter X-Zoom über alle Panels
- [ ] Volume-Panel (Balken, grün/rot)
- [ ] Separate Y-Achse pro Panel
- [ ] Panel-Splitter: Drag zum Resize

### Indikatoren
- [ ] Overlay-Indikatoren: SMA, EMA, Bollinger Bands (Line-Series über Kerzen)
- [ ] Sub-Panel: RSI (0–100, Overbought/Oversold-Linien)
- [ ] Sub-Panel: MACD (Linie + Signal + Histogramm)
- [ ] `Indicator`-Trait: berechnet aus `Series<f64>`, gibt `Series<f64>` zurück

### Tooltip
- [ ] Hover-Tooltip: OHLCV + alle aktiven Indikatoren
- [ ] Tooltip-Positionierung (kein Clipping am Rand)

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
- [ ] Vanilla-JS Demo-Seite (`examples/web/`)
- [ ] `package.json`, npm-publish workflow

### SMR-Integration
- [ ] Git-Dependency in SMR `Cargo.toml` eintragen
  ```toml
  powerchart = { git = "https://github.com/deinname/powerchart", features = ["wasm"] }
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

| Phase | Inhalt | Aufwand |
|---|---|---|
| 1 | Repo & Workspace | 1–2 Tage |
| 2 | Core-Datenstrukturen | 1 Woche |
| 3 | Renderer-Trait + SVG | 1 Woche |
| 4 | WASM Canvas-Renderer | 3–4 Wochen |
| 5 | Multi-Panel + Indikatoren | 2–3 Wochen |
| 6 | Pattern-Marker | 1 Woche |
| 7 | JS-API & npm-Paket | 1 Woche |
| 8 | Native Desktop (optional) | 2–3 Wochen |

**Gesamt bis v0.1 (ohne Phase 8): ~3 Monate**