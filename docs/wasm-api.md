# ferrochart-wasm -- API-Referenz

> **Stand:** 2026-04-02 CEST (Replay + Plugin + Cloud-Fill)
> **WASM-Build:** `wasm-pack build crates/wasm --target web`

---

## Crate-Struktur

```
ferrochart/
├── crates/core/           ← Datentypen, Indikatoren, Transforms (kein WASM-Bezug)
├── crates/render/         ← Renderer-Trait, SvgRenderer, ChartConfig
├── crates/wasm/           ← WASM-Bindings, CanvasRenderer, FerroChart-Klasse
└── examples/
    ├── src/main.rs        ← SVG-Beispiele (cargo run -p ferrochart-examples)
    └── web/index.html     ← Browser-Demo
```

---

## FerroChart -- vollständige API

### Konstruktor

```typescript
new FerroChart(canvas: HTMLCanvasElement): FerroChart
```

Initialisiert Chart mit Canvas-Dimensionen, startet internen rAF-Loop und
registriert Mouse/Wheel/Touch/Keyboard-Events auf dem Canvas-Element.

---

### Daten

```typescript
// Parallel-Arrays (Float64Array je n Elemente)
setData(
  timestamps: Float64Array,
  opens:      Float64Array,
  highs:      Float64Array,
  lows:       Float64Array,
  closes:     Float64Array,
  volumes:    Float64Array
): void

// Wie setData, zusätzlich institutional_ratios für Split-Candle-Rendering
setDataWithRatios(
  timestamps:            Float64Array,
  opens:                 Float64Array,
  highs:                 Float64Array,
  lows:                  Float64Array,
  closes:                Float64Array,
  volumes:               Float64Array,
  institutional_ratios:  Float64Array
): void

// JSON-basiert: Array<{timestamp,open,high,low,close,volume,institutional_ratio?}>
setDataJson(json: string): void

// Letzten Balken aktualisieren (Realtime-Tick innerhalb laufender Periode)
updateLastCandle(
  timestamp: number, open: number, high: number,
  low: number, close: number, volume: number
): void

// Neuen Balken anhängen (Periodenabschluss / neue Kerze)
pushCandle(
  timestamp: number, open: number, high: number,
  low: number, close: number, volume: number
): void
```

---

### Chart-Typ

```typescript
// name: "candlestick" | "heikin_ashi" | "line" | "area" | "ohlc"
//       "renko" | "point_figure"
setChartType(name: string): void

// Renko-Parameter (brick_size in Preiseinheiten)
setRenkoConfig(brick_size: number): void

// Point-&-Figure-Parameter
setPfConfig(box_size: number, reversal: number): void
```

---

### Indikatoren

```typescript
// Indikatoren-Namen und Standard-Perioden:
//   Overlay:    "sma"(20)  "ema"(10)  "bollinger"(20)  "donchian"(20)
//               "keltner"(20)  "parabolic_sar"(0)  "supertrend"(10)
//               "ichimoku"(9)  "session_vwap"(0)
//   Sub-Panel:  "rsi"(14)  "macd"(26)  "atr"(14)  "obv"(0)
//               "stochastic"(14)  "williams_r"(14)  "cci"(20)  "adx"(14)
//               "volume_sma"(20)  "cusum"(30)
addIndicator(name: string, period?: number): void

// Anchored VWAP ab einem bestimmten Bar-Index
addAnchoredVwap(anchor_bar: number): void

// Equity-Curve aus Returns-Array
addEquityCurve(returns: Float64Array): void

removeIndicator(name: string): void
clearIndicators(): void
```

---

### Marker

```typescript
// shape:    "arrow_up" | "arrow_down" | "diamond" | "circle"
// position: "above" | "below"
addMarker(
  bar_index: number,
  shape:     string,
  position:  string,
  r: number, g: number, b: number,
  label:     string
): void

clearMarkers(): void
```

---

### Annotations -- ML/SMR-Overlays

```typescript
// Konfidenzband (upper/lower als Float64Array, NaN = kein Wert)
addConfidenceBand(
  upper: Float64Array, lower: Float64Array,
  r: number, g: number, b: number,
  alpha: number
): void

// Walk-Forward-Zone (grün = Training, blau = Validierung)
addWalkForwardZone(
  start_bar: number, end_bar: number,
  is_train:  boolean,
  label:     string
): void

// Nachrichtenereignis (impact: -1..1, urgency: 1..5)
addNewsEvent(
  bar_index: number,
  label:     string,
  impact:    number,
  urgency:   number
): void

// Triple-Barrier-Label (take-profit / stop-loss / timeout-Fenster)
// outcome: "tp" | "sl" | "timeout"
addTripleBarrier(
  entry_bar:    number,
  entry_price:  number,
  take_profit:  number,
  stop_loss:    number,
  window_bars:  number,
  end_bar:      number,
  outcome:      string,
  r: number, g: number, b: number
): void

// GEX/Options-Profil als horizontales Histogramm
addHorizontalHistogram(
  values:  Float64Array,
  prices:  Float64Array,
  r: number, g: number, b: number, alpha: number
): void

// Horizontales Preisniveau mit Label (z.B. Max-Pain, Strike-Preise)
addHorizontalLevel(
  price: number, label: string,
  r: number, g: number, b: number,
  width: number
): void
```

---

### Annotations -- Drawing-Tools

```typescript
// Interaktives Zeichnen (2 Klicks, außer corridor/pitchfork = 3 Klicks)
// mode: "none" | "trendline" | "fibonacci" | "corridor"
//       "ray" | "measurement" | "ellipse" | "pitchfork" | "gann_fan"
setDrawMode(mode: string): void

// Direkt-Add (keine Benutzerinteraktion nötig)
addTrendLine(
  start_bar: number, start_price: number,
  end_bar:   number, end_price:   number,
  r: number, g: number, b: number,
  extend_right: boolean
): void

addFibonacci(
  high_bar: number, high_price: number,
  low_bar:  number, low_price:  number,
  r: number, g: number, b: number
): void

// Horizontale Preislinie (color_hex: "#RRGGBB")
addHorizontalRay(price: number, color_hex: string, width: number): void

// Vertikale Zeitlinie (bar_index im Gesamt-Datensatz)
addVerticalLine(bar_index: number, color_hex: string, width: number): void

// Preisrechteck (border_hex + fill_hex als "#RRGGBB")
addRectangle(
  start_bar:    number,
  end_bar:      number,
  top_price:    number,
  bottom_price: number,
  border_hex:   string,
  fill_hex:     string,
  width:        number
): void

// Text-Label an Preis/Bar-Position
addTextLabel(
  bar_index: number, price: number,
  text: string, color_hex: string
): void

// Ray (Halbgerade: startet bei start, läuft durch end bis zum rechten Rand)
addRay(
  start_bar: number, start_price: number,
  end_bar:   number, end_price:   number,
  color_hex: string, width: number
): void

// Measurement Tool (zeigt Δ$, Δ%, Δ Bars zwischen zwei Punkten)
addMeasurement(
  start_bar: number, start_price: number,
  end_bar:   number, end_price:   number,
  r: number, g: number, b: number
): void

// Ellipse (Bounding-Box-Ecken als Anker)
addEllipse(
  start_bar:  number, start_price: number,
  end_bar:    number, end_price:   number,
  border_hex: string, fill_hex:    string,
  width:      number
): void

// Andrews Pitchfork (3 Anker: Griff + 2 Zinken)
addPitchfork(
  bar1: number, price1: number,
  bar2: number, price2: number,
  bar3: number, price3: number,
  color_hex: string, width: number
): void

// Gann Fan (8 Fächerlinien von Anker; scale = Preiseinheiten pro Bar für 1×1-Linie)
addGannFan(
  anchor_bar: number, anchor_price: number,
  scale:      number,
  color_hex:  string
): void

// Price Channel (zwei parallele Trendlinien mit Fill)
addPriceChannel(
  start_bar: number, end_bar: number,
  upper_start: number, upper_end: number,
  lower_start: number, lower_end: number,
  color_hex: string, fill_hex: string, width: number
): void

// Alle Drawing-Tool-Annotations und ML-Overlays löschen
clearAnnotations(): void

// JSON-Export/Import (persisted state)
exportAnnotations(): string
importAnnotations(json: string): void
```

---

### Plugin-System — Custom Indicators

```typescript
// JS-berechnete Werte als Overlay-Indikator (z.B. eigene SMA, ML-Score)
// values: Float64Array, row-major: series0[0..n], series1[0..n], ...
addCustomOverlay(name: string, values: Float64Array, series_count: number): void

// JS-berechnete Werte als eigenes Sub-Panel (auto-skalierte Y-Achse)
addCustomSubPanel(name: string, values: Float64Array, series_count: number): void
```

---

### Replay-Modus

```typescript
// Replay starten ab Bar (1-based). Chart zeigt nur die ersten N Bars.
replayStart(start_bar: number): void

// Einen Bar weiter. Rückgabe: neue Cursor-Position (0 = nicht im Replay).
replayStep(): number

// Auto-Play starten (speed_ms = Millisekunden zwischen Bars)
replayPlay(speed_ms: number): void

// Auto-Play pausieren (Replay bleibt aktiv)
replayPause(): void

// Replay komplett beenden, Chart zeigt wieder alle Daten
replayStop(): void

// Aktuelle Replay-Position (0 = kein Replay aktiv)
replayPosition(): number
```

---

### Crosshair-Abfrage

```typescript
// Aktueller Preis unter dem Cursor (NaN wenn außerhalb des Charts)
// Basiert auf der internen price_transform — exakte Übereinstimmung mit Tooltip
getCrosshairPrice(): number

// Aktueller Bar-Index (im Gesamt-Datensatz) unter dem Cursor, -1 wenn außerhalb
getCrosshairBar(): number
```

---

### Viewport / Zoom-Pan

```typescript
// Zoom via Mausrad (delta_y in Pixeln, mouse_x für Zoom-Zentrum)
onWheel(delta_y: number, mouse_x: number): void

// Pan um dx Pixel (positiv = nach rechts)
onPan(dx: number): void

// Zoom-Pan-State lesen/schreiben (für Multi-Chart-Sync)
getZoomPanState(): Uint32Array  // [visible_bars, offset]
setZoomPanState(visible_bars: number, offset: number): void
```

---

### Konfiguration

```typescript
resize(width: number, height: number): void

// theme: "dark" | "light"
setTheme(theme: string): void

setLogScale(enabled: boolean): void

// Volumen-Profil einblenden (num_buckets=0 = ausblenden)
showVolumeProfile(num_buckets: number): void

// Vollständige ChartConfig als JSON setzen
// Felder: price_scale, panel_weights, log_y, chart_type, ...
setConfig(json: string): void
```

---

## Ist-Stand vs. offene Punkte

| Feature | Status |
|---|---|
| `setData` / `setDataJson` / `updateLastCandle` / `pushCandle` | ✅ |
| `addIndicator` (22 Indikatoren) | ✅ |
| `setChartType` (7 Typen inkl. Renko + P&F) | ✅ |
| `addMarker`, `addTrendLine`, `addFibonacci`, `addTripleBarrier` | ✅ |
| `addConfidenceBand`, `addWalkForwardZone`, `addNewsEvent` | ✅ |
| `addHorizontalRay`, `addVerticalLine`, `addRectangle`, `addTextLabel` | ✅ |
| `addRay`, `addMeasurement`, `addEllipse`, `addPitchfork`, `addGannFan` | ✅ |
| `addPriceChannel` | ✅ |
| `addCustomOverlay`, `addCustomSubPanel` (Plugin-System) | ✅ |
| `replayStart/Step/Play/Pause/Stop/Position` (Replay-Modus) | ✅ |
| `getCrosshairPrice`, `getCrosshairBar` | ✅ |
| `getZoomPanState` / `setZoomPanState` (Multi-Chart-Sync) | ✅ |
| `onWheel` / `onPan` (externe Event-Integration) | ✅ |
| `setConfig(json)` | ✅ |
| DirtyFlags (layer-granular) | ✅ |
| Ichimoku Cloud-Fill (render-only) | ✅ |
| CanvasRenderer in `ferrochart-render` verschieben | offen |
| `@ferrochart/web` TypeScript-Wrapper (npm) | offen |
| Drawing-Tools selektieren / verschieben / löschen | offen |
| Undo/Redo für Zeichnungen | offen |

---

## Build

```bash
# WASM-Paket bauen
wasm-pack build crates/wasm --target web

# SVG-Beispiele generieren (output/01_candlestick.svg … 17_point_figure.svg)
cargo run -p ferrochart-examples

# Tests
cargo test --workspace

# Web-Demo starten (vom Projekt-Root!)
python3 -m http.server 8080
# → http://localhost:8080/examples/web/
```
