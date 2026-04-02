# FerroChart -- Roadmap & Todo

> **Stand:** 2026-04-01 20:15 CEST
> **Tests:** 233 (190 core + 43 render), Clippy-pedantic clean

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

## Phase 6 -- Chart-Typen

> Aktuell nur Candlesticks. Non-Uniform X-Achse ist Blocker fuer Renko/P&F.

| Feature | Beschreibung | Abhaengigkeit |
|---|---|---|
| Heikin-Ashi | Berechnete Candles (HA-Open/Close), gleiche X-Achse | Neuer CandleGeometry-Modus |
| OHLC Bars | Klassische Balken statt Candles (Strich links=Open, rechts=Close) | Renderer-Variante |
| Line / Area Chart | Close-only als Linie oder gefuellte Flaeche | Einfache Renderer-Variante |
| Renko | Zeitunabhaengige Bricks basierend auf Preisbewegung | **Non-Uniform X-Achse** |
| Point & Figure | X/O-Saeulen, zeitunabhaengig | **Non-Uniform X-Achse** |
| Session-Separation | Pre/Regular/Post-Market Zonen | Timestamp-basierte Erkennung |

---

## Phase 7 -- Drawing Tools

> Aktuell 3 Tools (Trendline, Fibonacci, Corridor). TradingView hat ~50.

### Prioritaet 1 (Haendler-Grundbeduerfnis)
- [ ] Horizontale Linie (Preis-Level, frei platzierbar)
- [ ] Vertikale Linie (Zeitpunkt markieren)
- [ ] Rechteck / Box (Preis x Zeit Zone)
- [ ] Text-Label (frei platzierbar)
- [ ] Price Channel (parallele Trendlinien durch Highs/Lows)

### Prioritaet 2 (Advanced)
- [ ] Andrews Pitchfork
- [ ] Gann Fan
- [ ] Ellipse
- [ ] Measurement Tool (Preis-/Zeitdifferenz anzeigen)
- [ ] Ray (Halbgerade ab einem Punkt)

### Infrastruktur
- [ ] Zeichnungen selektieren, verschieben, loeschen (Edit-Modus)
- [ ] Snap-to-OHLC (Zeichenpunkte rasten auf High/Low/Open/Close ein)
- [ ] Undo/Redo fuer Zeichnungen

---

## Phase 8 -- Indikator-Bibliothek

> Aktuell 9 Indikatoren. Ziel: ~25-30 Alltagswerkzeuge.

### Fehlende Standard-Indikatoren
- [ ] ATR (Average True Range)
- [ ] Stochastic Oscillator (%K, %D)
- [ ] Williams %R
- [ ] CCI (Commodity Channel Index)
- [ ] ADX / DMI (Directional Movement)
- [ ] Ichimoku Cloud (Tenkan, Kijun, Senkou A/B, Chikou)
- [ ] Parabolic SAR
- [ ] Session VWAP (Reset pro Handelstag, nicht anchored)
- [ ] OBV (On-Balance Volume)
- [ ] Donchian Channels
- [ ] Keltner Channels
- [ ] Supertrend

### Infrastruktur
- [ ] Plugin-System: Custom Indicators von aussen registrieren (Trait-basiert, kein Pine Script noetig)
- [ ] Custom Renderer fuer Indicators (z.B. Ichimoku Cloud braucht fill_polygon zwischen Senkou A/B)

---

## Phase 9 -- Erweiterte Konzepte

| Feature | Beschreibung | Aufwand |
|---|---|---|
| Replay-Modus | Bar-by-Bar historisches Abspielen, Play/Pause/Speed. Unverzichtbar fuer Backtesting-Workflows und SMR-ML-Training-Visualisierung. | Mittel |
| Multi-Symbol Overlay | Zweites Symbol als Overlay-Linie (relative Performance %). Braucht zweite Y-Achse oder Normalisierung. | Mittel |
| Rechte + Linke Y-Achse | Zwei unabhaengige Preis-Achsen fuer Overlay-Vergleich | Mittel |
| Chart-Template | Indikatoren + Zeichnungen + Layout als eine Einheit speichern/laden. `exportAnnotations` ist zu granular. | Klein |
| Alert-Datenstruktur | Price/Indicator-Crossing Alerts als Core-Typ (nicht UI, nur Daten) | Klein |
| Footprint Charts | Bid-Ask-Volumen pro Preisniveau pro Kerze (Order Flow) | Gross |
| Market Profile / TPO | Time-Price-Opportunity, anders als Volume Profile (Buchstaben-Saeulen) | Gross |

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
| 6 | Chart-Typen (Heikin-Ashi, OHLC, Line, Renko, P&F) | -- |
| 7 | Drawing Tools (~15 Tools + Edit/Snap/Undo) | -- |
| 8 | Indikator-Bibliothek (~25-30 + Plugin-System) | -- |
| 9 | Erweiterte Konzepte (Replay, Multi-Symbol, Templates, Footprint) | -- |

### Strategische Einordnung

**Differenzierung gegenueber TradingView:** Die ML-spezifischen Features (Confidence Bands,
Walk-Forward Zones, Triple Barrier, CUSUM, Equity Curve) sind das eigentliche
Alleinstellungsmerkmal. TradingView hat nichts davon. Diese Staerke weiter ausbauen statt
TradingView auf deren Terrain (50 Drawing Tools, Pine Script) zu kopieren.

**Groesste Hebel fuer "besser als TradingView":**
1. LOD/Decimation -- ohne das skaliert nichts bei Tick-Daten
2. Chart-Typen -- Heikin-Ashi + Line/Area sind Haendler-Grundbeduerfnis
3. Replay-Modus -- einzigartiger Vorteil fuer SMR/ML-Workflow
4. Indikator-Bibliothek auf ~25 bringen
5. Drawing Tools auf ~10-15 bringen
