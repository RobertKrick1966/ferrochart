# FerroChart -- Roadmap & Todo

> **Stand:** 2026-04-02 CEST
> **Tests:** 346 (297 core + 43 render + 6 ffi), Clippy-pedantic clean

---

## Phase 1 -- Fundament ✅

### Repo & Workspace
- [x] GitHub-Repo, README, Lizenz, `.gitignore`
- [x] Cargo-Workspace: `core`, `render`, `wasm`, `examples`
- [x] CI via GitHub Actions (`cargo test` + `clippy` + WASM build)

### Core-Datenstrukturen (`ferrochart-core`)
- [x] `Ohlcv`, `Series<T>`, `PriceRange`, `TimeRange`
- [x] `Viewport`, `Rect`, `Point`, `Transform` (lineares + logarithmisches Koordinaten-Mapping)
- [x] `PanelLayout` -- Multi-Panel mit Gewichtung
- [x] `ZoomPanState` -- Zoom-Level, sichtbarer Range, Offset, Future Space
- [x] `CandleGeometry` -- Pixel-Koordinaten pro Kerze (inkl. `institutional_ratio`)
- [x] `interaction` -- `compute_zoom()`, `compute_pan()`, `is_in_chart_area()` (pure functions)
- [x] `indicator` -- SMA, EMA, Bollinger Bands, RSI, MACD, VolumeSMA, CUSUM, AnchoredVWAP, EquityCurve

### Renderer-Trait + SVG-Backend (`ferrochart-render`)
- [x] `Renderer`-Trait: 11 Methoden inkl. `fill_polygon`, `draw_circle`
- [x] `SvgRenderer`, `CanvasRenderer` (in wasm)
- [x] Achsen: X auto-detect (Daily/Hourly/Minute), Y (Preis, Log-Space-Ticks)

### Interaktivitaet (WASM)
- [x] Mouse/Touch/Keyboard, Crosshair, Panel-Tooltip, Y-Drag, Panel-Splitter
- [x] Zeichenwerkzeuge: Trendline, Fibonacci, Corridor (interaktiv)
- [x] Annotations-Persistierung (export/import JSON)
- [x] Realtime: `updateLastCandle`, `pushCandle` (Auto-Scroll)
- [x] DirtyFlags (CANDLES/INDICATORS/ANNOTATIONS/OVERLAY)
- [x] Log Y-Achse, Dark/Light Theme

### API-Alignment ✅
- [x] `setConfig(json)`, `setDataJson(json)`, `onWheel`/`onPan`
- [x] `@ferrochart/web` TS-Wrapper (`packages/web/`)

---

## Phase 2 -- SMR-Kern ✅

- [x] CUSUM, Triple Barrier, Imbalance Coloring, Anchored VWAP, Volume Profile

## Phase 3 -- ML-Integration ✅

- [x] Confidence Band, Walk-Forward Zones, News Event Overlay

## Phase 4 -- Erweitert ✅

- [x] GEX Profile, Max Pain, Multi-Chart Sync, Equity Curve

---

## Phase 5 -- Performance & Skalierung

> Blocker fuer "besser als TradingView" bei grossen Datenmengen

| Feature | Beschreibung | Prioritaet |
|---|---|---|
| LOD / Decimation | ✅ `min_max_decimate` (O(n), OHLCV-gruppierung), `lttb_decimate` (Indikator-Linien), `decimate_target` (Auto-Erkennung), thin-candle Fast-Path im Renderer | ✅ |
| Virtualisiertes Rendering | ✅ Nur sichtbare Bars gerendert (Slice in render_frame), Decimation wenn sub-pixel. Skaliert bis ~200k Candles auf Canvas 2D. | ✅ |
| WebGL/wgpu Renderer | GPU-beschleunigtes Rendering fuer Tick-Daten jenseits 200k. `wgpu` fuer Desktop, WebGL2 fuer Browser. | Offen |

---

## Phase 6 -- Chart-Typen ✅

| Feature | Beschreibung | Status |
|---|---|---|
| Heikin-Ashi | ✅ `compute_heikin_ashi()`, `setChartType("heikin_ashi")` | ✅ |
| OHLC Bars | ✅ `draw_ohlc_bars()`, `setChartType("ohlc")` | ✅ |
| Line / Area Chart | ✅ `draw_line_chart()`, `draw_area_chart()`, `setChartType("line"/"area")` | ✅ |
| Renko | ✅ `compute_renko()`, `render_renko_chart()`, `setChartType("renko")`, `setRenkoConfig(brick_size)` | ✅ |
| Point & Figure | ✅ `compute_point_figure()`, `render_point_figure_chart()`, `setChartType("point_figure")`, `setPfConfig(box_size, reversal)` | ✅ |
| Session-Separation | Pre/Regular/Post-Market Zonen | offen |

---

## Phase 7 -- Drawing Tools (teilweise ✅)

> Aktuell 13 Tools (Trendline, Fibonacci, Corridor, HorizontalRay, VerticalLine, RectangleZone, TextLabel, Ray, MeasurementTool, Ellipse, AndrewsPitchfork, GannFan, PriceChannel). TradingView hat ~50.

### Prioritaet 1 (Haendler-Grundbeduerfnis)
- [x] Horizontale Linie (`HorizontalRay`, `addHorizontalRay()`)
- [x] Vertikale Linie (`VerticalLine`, `addVerticalLine()`)
- [x] Rechteck / Box (`RectangleZone`, `addRectangle()`)
- [x] Text-Label (`TextLabel`, `addTextLabel()`)
- [x] Price Channel (`PriceChannel`, `addPriceChannel()`)

### Prioritaet 2 (Advanced)
- [x] Andrews Pitchfork
- [x] Gann Fan
- [x] Ellipse
- [x] Measurement Tool (Preis-/Zeitdifferenz anzeigen)
- [x] Ray (Halbgerade ab einem Punkt)

### Infrastruktur
- [ ] Zeichnungen selektieren, verschieben, loeschen (Edit-Modus)
- [ ] Snap-to-OHLC (Zeichenpunkte rasten auf High/Low/Open/Close ein)
- [ ] Undo/Redo fuer Zeichnungen

---

## Phase 8 -- Indikator-Bibliothek ✅

> 22 Indikatoren implementiert.

### Standard-Indikatoren
- [x] SMA, EMA, Bollinger Bands, RSI, MACD, VolumeSMA
- [x] CUSUM, Triple Barrier, AnchoredVWAP, VolumeProfile, EquityCurve
- [x] ATR (Average True Range, Wilder-Smoothing)
- [x] OBV (On-Balance Volume)
- [x] Session VWAP (Reset pro Handelstag)
- [x] Stochastic Oscillator (%K, %D)
- [x] Donchian Channels (Upper/Mid/Lower)
- [x] Keltner Channels (EMA ± ATR-Multiplikator)
- [x] Williams %R
- [x] CCI (Commodity Channel Index)
- [x] ADX / DMI (Wilder-Smoothing, +DI/-DI/ADX)
- [x] Parabolic SAR (State Machine, AF-Step/Max konfigurierbar)
- [x] Supertrend (ATR-basiert)
- [x] Ichimoku Cloud (5 Linien: Tenkan, Kijun, Senkou A/B, Chikou)

### Infrastruktur
- [x] **Plugin-System:** `addCustomOverlay(name, values, series_count)` und `addCustomSubPanel()` --
  JS-berechnete Indikatoren als First-Class Overlays/Sub-Panels. Kein Core-Change noetig.
- [x] Ichimoku Cloud-Fill (gruen/rot fill_polygon zwischen Senkou A/B mit Crossover-Handling)

---

## Phase 9 -- Erweiterte Konzepte

| Feature | Beschreibung | Aufwand | Prio |
|---|---|---|---|
| ~~**Replay-Modus**~~ | ✅ `replayStart(bar)`, `replayStep()`, `replayPlay(speed_ms)`, `replayPause()`, `replayStop()`, `replayPosition()`. Slices data to `[0..cursor]`, recomputes indicators, auto-play via setInterval. | ✅ | ✅ |
| ~~**Plugin-System**~~ | ✅ `addCustomOverlay(name, values, series_count)`, `addCustomSubPanel()`. JS-berechnete Werte als First-Class Indikatoren. | ✅ | ✅ |
| Multi-Symbol Overlay | Zweites Symbol als Overlay-Linie (relative Performance %). Braucht zweite Y-Achse oder Normalisierung. | Mittel | Mittel |
| Rechte + Linke Y-Achse | Zwei unabhaengige Preis-Achsen fuer Overlay-Vergleich | Mittel | Mittel |
| Chart-Template | Indikatoren + Zeichnungen + Layout als eine Einheit speichern/laden. `exportAnnotations` ist zu granular. | Klein | Mittel |
| Alert-Datenstruktur | Price/Indicator-Crossing Alerts als Core-Typ (nicht UI, nur Daten) | Klein | Niedrig |
| Footprint Charts | Bid-Ask-Volumen pro Preisniveau pro Kerze (Order Flow) | Gross | Niedrig |
| Market Profile / TPO | Time-Price-Opportunity, anders als Volume Profile (Buchstaben-Saeulen) | Gross | Niedrig |

---

## Phase 10 -- FFI / Sprachanbindungen ✅

> C-kompatibles FFI fuer Integration in C++, Python, C#, Go und weitere Sprachen.

| Feature | Beschreibung | Status |
|---|---|---|
| `ferrochart-ffi` Crate | Opaker Handle, `extern "C"` Funktionen, `cdylib` + `staticlib` | ✅ |
| C-Header (cbindgen) | Automatisch generierter `ferrochart.h` Header | ✅ |
| C/C++ Doku | Integration Guide mit RAII-Wrapper, Compile-Anleitung | ✅ |
| Python Doku | ctypes-Bindings, High-Level Wrapper, NumPy/Pandas Beispiele | ✅ |
| C# Doku | P/Invoke Bindings, IDisposable Wrapper, ASP.NET Beispiel | ✅ |
| API-Abdeckung | Lifecycle, Data (Arrays + JSON), Config, 18 Indikatoren, Markers, Annotations (JSON), SVG-Rendering | ✅ |

---

## Backlog

- [ ] npm-publish workflow (GitHub Actions, auf Release)
- [ ] SMR Pattern-Signale als Marker durchschleifen
- [ ] `winit` Desktop-Fenster + `tiny-skia` CPU-Renderer (optional)

---

## Zusammenfassung

| Phase | Inhalt | Status |
|---|---|---|
| 1 | Fundament (Core + Render + WASM + Interaktion + Annotations) | ✅ |
| 2 | SMR-Kern (CUSUM, Triple Barrier, VWAP, Volume Profile, Imbalance) | ✅ |
| 3 | ML-Integration (Confidence Band, Walk-Forward, News Events) | ✅ |
| 4 | Erweitert (GEX, Max Pain, Multi-Chart Sync, Equity Curve) | ✅ |
| 5 | Performance & Skalierung (LOD ✅, Virtualisierung ✅, WebGL offen) | teilweise ✅ |
| 6 | Chart-Typen (HA ✅, OHLC ✅, Line ✅, Area ✅, Renko ✅, P&F ✅) | ✅ |
| 7 | Drawing Tools (13 Tools ✅, Edit/Snap/Undo offen) | teilweise ✅ |
| 8 | Indikator-Bibliothek (22 Indikatoren ✅, Plugin-System ✅, Cloud-Fill ✅) | ✅ |
| 9 | Erweiterte Konzepte (Replay ✅, Plugin ✅, Multi-Symbol/Templates/Footprint offen) | teilweise ✅ |
| 10 | FFI / Sprachanbindungen (C/C++, Python, C# -- Crate + Header + Docs) | ✅ |

### Strategische Einordnung

**Differenzierung gegenueber TradingView:** Die ML-spezifischen Features (Confidence Bands,
Walk-Forward Zones, Triple Barrier, CUSUM, Equity Curve) sind das eigentliche
Alleinstellungsmerkmal. TradingView hat nichts davon. Diese Staerke weiter ausbauen statt
TradingView auf deren Terrain (50 Drawing Tools, Pine Script) zu kopieren.

**Groesste Hebel fuer "besser als TradingView":**
1. ~~LOD/Decimation~~ ✅
2. ~~Chart-Typen~~ ✅ -- 7 Typen inkl. Renko + P&F
3. ~~**Replay-Modus**~~ ✅ -- einzigartiger Vorteil fuer SMR/ML-Workflow
4. ~~**Plugin-System**~~ ✅ -- JS-berechnete Indikatoren als First-Class Overlays
5. ~~Drawing Tools auf ~10-15~~ ✅ -- 13 Tools
6. Edit-Modus / Snap / Undo fuer Zeichnungen
