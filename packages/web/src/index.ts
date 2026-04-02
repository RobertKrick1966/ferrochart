// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Robert Krick

/**
 * @ferrochart/web — thin TypeScript wrapper over the FerroChart WASM module.
 *
 * Usage:
 * ```ts
 * import { FerroChart } from "@ferrochart/web";
 *
 * const chart = await FerroChart.create(canvas, { theme: "dark" });
 * chart.setData(candles);
 * ```
 */

// The WASM module must be provided by the consumer (import map or bundler alias).
// Default path assumes the wasm pkg is at ../../crates/wasm/pkg/ relative to this file.
let wasmInit: (() => Promise<void>) | null = null;
let WasmFerroChart: any = null;

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
  logY?: boolean;
  indicators?: IndicatorConfig[];
}

export type IndicatorConfig =
  | { type: "sma"; period: number }
  | { type: "ema"; period: number }
  | { type: "bollinger"; period: number }
  | { type: "rsi"; period: number }
  | { type: "macd"; period?: number }
  | { type: "cusum"; threshold?: number }
  | { type: "volsma"; period: number };

/**
 * Initialize the WASM module. Must be called before creating charts.
 * Pass the `init` and `FerroChart` exports from the WASM package.
 */
export function initWasm(init: () => Promise<void>, ferroChartClass: any): void {
  wasmInit = init;
  WasmFerroChart = ferroChartClass;
}

export class FerroChart {
  private wasm: any;
  private rafId = 0;

  private constructor(wasm: any) {
    this.wasm = wasm;
  }

  /**
   * Create a new interactive chart on the given canvas.
   * Call `initWasm()` first to provide the WASM module.
   */
  static async create(
    canvas: HTMLCanvasElement,
    options: ChartOptions = {}
  ): Promise<FerroChart> {
    if (!wasmInit || !WasmFerroChart) {
      throw new Error(
        "Call initWasm(init, FerroChart) before creating charts"
      );
    }
    await wasmInit();

    const wasm = new WasmFerroChart(canvas);
    const chart = new FerroChart(wasm);

    if (options.theme) {
      wasm.setTheme(options.theme);
    }
    if (options.logY) {
      wasm.setLogScale(true);
    }
    if (options.indicators) {
      for (const ind of options.indicators) {
        wasm.addIndicator(ind.type, (ind as any).period ?? undefined);
      }
    }

    chart.startRafLoop();
    return chart;
  }

  /** Set OHLCV data from an array of objects. */
  setData(candles: OhlcvDto[]): void {
    this.wasm.setDataJson(JSON.stringify(candles));
  }

  /** Update the last candle in-place (realtime tick). */
  updateLastCandle(c: OhlcvDto): void {
    this.wasm.updateLastCandle(
      c.timestamp, c.open, c.high, c.low, c.close, c.volume
    );
  }

  /** Append a new candle (new trading period). */
  pushCandle(c: OhlcvDto): void {
    this.wasm.pushCandle(
      c.timestamp, c.open, c.high, c.low, c.close, c.volume
    );
  }

  /** Resize the chart (call after canvas size changes). */
  resize(width: number, height: number): void {
    this.wasm.resize(width, height);
  }

  /** Toggle logarithmic Y-axis. */
  setLogScale(enabled: boolean): void {
    this.wasm.setLogScale(enabled);
  }

  /** Show/hide volume profile. Pass 0 to hide. */
  showVolumeProfile(buckets: number): void {
    this.wasm.showVolumeProfile(buckets);
  }

  /** Add an anchored VWAP from the given bar index. */
  addAnchoredVwap(anchorBar: number): void {
    this.wasm.addAnchoredVwap(anchorBar);
  }

  /** Clear all annotations (trendlines, barriers, etc.). */
  clearAnnotations(): void {
    this.wasm.clearAnnotations();
  }

  /** Export annotations as JSON string. */
  exportAnnotations(): string {
    return this.wasm.exportAnnotations();
  }

  /** Import annotations from JSON string. */
  importAnnotations(json: string): void {
    this.wasm.importAnnotations(json);
  }

  /** Get zoom/pan state for multi-chart sync. */
  getZoomPanState(): [number, number, number] {
    return this.wasm.getZoomPanState();
  }

  /** Set zoom/pan state for multi-chart sync. */
  setZoomPanState(visibleBars: number, offset: number): void {
    this.wasm.setZoomPanState(visibleBars, offset);
  }

  /** Handle wheel event externally. */
  onWheel(deltaY: number, mouseX: number): void {
    this.wasm.onWheel(deltaY, mouseX);
  }

  /** Handle pan event externally. */
  onPan(dx: number): void {
    this.wasm.onPan(dx);
  }

  /** Stop the render loop and free WASM resources. */
  destroy(): void {
    cancelAnimationFrame(this.rafId);
    this.wasm.free();
  }

  private startRafLoop(): void {
    const tick = () => {
      // WASM render loop handles dirty checking internally
      this.rafId = requestAnimationFrame(tick);
    };
    this.rafId = requestAnimationFrame(tick);
  }
}
