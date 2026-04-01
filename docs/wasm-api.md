# ferrochart-wasm -- API Design & Ist/Soll-Abgleich

> **Stand:** 2026-04-01 16:30 CEST

## Crate-Struktur

```
ferrochart/
├── ferrochart-core/       ← rein, kein WASM-Bezug
├── ferrochart-render/     ← SvgRenderer (vorhanden), Canvas2dRenderer (Soll: hierher, Ist: in wasm)
├── ferrochart-wasm/       ← WASM-Bindings + JS-API + CanvasRenderer (Ist)
└── ferrochart-examples/   ← SVG-Beispiele + Web-Demo
```

---

## 1. CanvasRenderer -- Ist vs. Soll

### Ist-Stand

`CanvasRenderer` lebt in `ferrochart-wasm/src/canvas.rs`. Implementiert `Renderer`-Trait direkt
gegen `web-sys::CanvasRenderingContext2d`. Kein Feature-Flag, fest an WASM gebunden.

```rust
// ferrochart-wasm/src/canvas.rs (Ist)
pub struct CanvasRenderer {
    ctx: CanvasRenderingContext2d,
    width: f64,
    height: f64,
}
```

### Soll-Design

`Canvas2dRenderer` in `ferrochart-render` hinter Feature-Flag `canvas2d`.
Vorteil: Renderer-Tests auch ohne WASM moeglich, saubere Trennung.

```rust
// ferrochart-render/src/canvas2d.rs (Soll)
// Feature-Flag: #[cfg(feature = "canvas2d")]
pub struct Canvas2dRenderer {
    ctx: CanvasRenderingContext2d,
    width: f64,
    height: f64,
    clip_stack: Vec<Rect>,        // fuer geschachtelte clip()/restore_clip()
}
```

### Delta

| Aspekt | Ist | Soll |
|---|---|---|
| Location | `ferrochart-wasm/src/canvas.rs` | `ferrochart-render/src/canvas2d.rs` |
| Feature-Flag | keiner (immer kompiliert mit wasm) | `canvas2d` in ferrochart-render |
| Clip-Stack | kein Stack, nutzt `ctx.save()/restore()` | expliziter `Vec<Rect>` Stack |
| Dashed Lines | nicht implementiert | `LineStyle::Dashed` -> `set_line_dash()` |

---

## 2. FerroChart WASM-Klasse -- Ist vs. Soll

### Ist-Stand (implementierte API)

```typescript
// Konstruktor
new FerroChart(canvas: HTMLCanvasElement): FerroChart

// Daten
setData(timestamps, opens, highs, lows, closes, volumes: Float64Array): void
setDataWithRatios(..., institutional_ratios: Float64Array): void

// Indikatoren (einzeln add/remove)
addIndicator(name: string, period?: number): void
removeIndicator(name: string): void
clearIndicators(): void

// Marker (einzeln add)
addMarker(barIndex, shape, position, r, g, b, label): void
clearMarkers(): void

// Annotations (einzeln add + bulk import/export)
addTrendLine(startBar, startPrice, endBar, endPrice, r, g, b, extendRight): void
addFibonacci(highBar, highPrice, lowBar, lowPrice, r, g, b): void
setDrawMode(mode: "none"|"trendline"|"fibonacci"|"corridor"): void
clearAnnotations(): void
exportAnnotations(): string     // JSON
importAnnotations(json: string): void

// Config
setTheme(theme: "dark"|"light"): void
resize(width: number, height: number): void
```

**Architektur Ist:**
- State in `Rc<RefCell<ChartState>>` (shared mit Event-Closures)
- Event-Handler intern registriert im Konstruktor (Mouse, Wheel, Touch, Keyboard)
- rAF-Loop intern gestartet, rendert bei `dirty: bool` Flag
- Dirty ist ein einzelnes `bool`, keine Layer-Granularitaet

### Soll-Design (Ziel-API)

```typescript
// Konstruktor
new FerroChart(canvas: HTMLCanvasElement): FerroChart

// Daten (JSON-basiert, nicht parallel arrays)
set_data(data_json: string): void                    // Array<OhlcvDto>
update_last_candle(candle_json: string): void         // Realtime-Tick
push_candle(candle_json: string): void                // Neue Periode

// Bulk-Setter (JSON)
set_indicators(json: string): void
set_markers(json: string): void
set_annotations(json: string): void

// Config
set_theme(dark: boolean): void                        // bool statt string
set_config(config_json: string): void
resize(width: number, height: number): void

// Input (explizit, nicht intern registriert)
on_wheel(delta: number, cursor_x: number): void
on_pan(dx: number): void

// Rendering (explizit, nicht interner rAF-Loop)
render_if_needed(): void                              // fuer externen rAF-Loop
render(): void                                        // erzwungener Redraw
```

### Delta

| Aspekt | Ist | Soll | Prioritaet |
|---|---|---|---|
| Daten-Format | Parallel `Float64Array` | JSON `Array<OhlcvDto>` | Mittel |
| `update_last_candle` | nicht vorhanden | Realtime-Tick Update | Hoch |
| `push_candle` | nicht vorhanden | Neue Kerze anhaengen | Hoch |
| Indikator-API | `addIndicator(name, period)` einzeln | `set_indicators(json)` bulk | Niedrig |
| Marker-API | `addMarker(...)` einzeln | `set_markers(json)` bulk | Niedrig |
| Event-Handler | intern registriert im Konstruktor | extern via `on_wheel`/`on_pan` | Niedrig |
| rAF-Loop | intern gestartet | extern via `render_if_needed()` | Niedrig |
| Dirty-Flags | `bool` | Layer-granular (`DirtyFlags` bitfield) | Mittel |
| `set_theme` | String `"dark"/"light"` | `bool dark` | Niedrig |
| `set_config` | nicht vorhanden | JSON-basiert | Mittel |

---

## 3. DirtyFlags -- Soll-Design

Ist: einzelnes `dirty: bool` in `ChartState`.

Soll: Layer-granulare Flags fuer selektiven Redraw.

```rust
#[derive(Default)]
struct DirtyFlags(u8);

#[repr(u8)]
enum Layer {
    Candles     = 0b0001,
    Indicators  = 0b0010,
    Annotations = 0b0100,
    Overlay     = 0b1000,   // Crosshair, Tooltip
    All         = 0b1111,
}

impl DirtyFlags {
    fn mark(&mut self, layer: Layer) { self.0 |= layer as u8; }
    fn mark_all(&mut self)           { self.0 = 0b1111; }
    fn is_clean(&self) -> bool       { self.0 == 0 }
    fn clear(&mut self)              { self.0 = 0; }
}
```

**Nutzen:** Bei Realtime-Ticks nur `Layer::Candles` dirty markieren, Indikatoren/Annotations
muessen nicht neu berechnet werden. Bei Crosshair-Bewegung nur `Layer::Overlay`.

---

## 4. TypeScript-Wrapper -- Soll-Design (noch nicht implementiert)

NPM-Package `@ferrochart/web` als schlanker Wrapper ueber die WASM-Klasse:

```typescript
export interface OhlcvDto {
  timestamp: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  institutional_ratio?: number;
}

export interface ChartOptions {
  theme?: "dark" | "light";
  panelWeights?: number[];
  indicators?: IndicatorConfig[];
}

export type IndicatorConfig =
  | { type: "sma"; period: number; color?: string }
  | { type: "ema"; period: number; color?: string }
  | { type: "bollinger"; period: number; stddev?: number }
  | { type: "rsi"; period: number }
  | { type: "macd"; fast?: number; slow?: number; signal?: number };

export class FerroChart {
  static async create(
    canvas: HTMLCanvasElement,
    options?: ChartOptions
  ): Promise<FerroChart>;

  setData(candles: OhlcvDto[]): void;
  updateLastCandle(candle: OhlcvDto): void;
  pushCandle(candle: OhlcvDto): void;
  resize(width: number, height: number): void;
  destroy(): void;
}
```

**Verwendung (5 Zeilen):**

```typescript
import { FerroChart } from "@ferrochart/web";

const chart = await FerroChart.create(canvas, {
  theme: "dark",
  indicators: [{ type: "ema", period: 20 }],
});
chart.setData(await fetchOhlcv("BTCUSDT", "1h"));

// Realtime via WebSocket:
ws.onmessage = (e) => chart.updateLastCandle(JSON.parse(e.data));
```

---

## 5. Build-Pipeline

### Ist

```bash
wasm-pack build crates/wasm --target web --out-dir pkg
# Output: crates/wasm/pkg/ferrochart_wasm.js + .wasm + .d.ts
```

### Soll (zusaetzlich)

```bash
# TypeScript-Wrapper bauen + npm publish
cd packages/web && npm run build && npm publish
```

```
ferrochart/
├── crates/wasm/pkg/               ← WASM-Output (generiert)
└── packages/web/                  ← @ferrochart/web npm Package (Soll)
    ├── src/index.ts               ← TS-Wrapper
    ├── package.json
    └── tsconfig.json
```

---

## 6. Abhaengigkeitsgraph

```
ferrochart-core              (kein WASM-Bezug, bleibt rein)
       |
ferrochart-render            + feature "canvas2d" -> Canvas2dRenderer (Soll)
       |
ferrochart-wasm              wasm-bindgen + web-sys -> FerroChart (WASM-Klasse)
       |
@ferrochart/web (npm)        TS-Wrapper -> FerroChart (JS-Klasse) + rAF-Loop (Soll)
```

---

## 7. Migrationsstrategie

Die Ist-API funktioniert vollstaendig. Migration zur Soll-API in Schritten:

1. **Hoch-Prioritaet (Realtime):** `update_last_candle()` + `push_candle()` hinzufuegen --
   kein Breaking Change, erweitert nur die API
2. **DirtyFlags:** `bool` -> Bitfield -- internes Refactoring, kein API-Change
3. **Canvas2dRenderer verschieben:** nach `ferrochart-render` mit Feature-Flag --
   internes Refactoring, WASM-Crate importiert dann von render
4. **JSON-basierte Setter:** parallel zu bestehenden Array-Methoden anbieten,
   alte Methoden mit `#[deprecated]` markieren
5. **TS-Wrapper:** `@ferrochart/web` Package anlegen, WASM als Dependency
6. **Externe Event-Handler:** Optional -- interne Handler bleiben als Default,
   `on_wheel`/`on_pan` als Alternative fuer Framework-Integration
