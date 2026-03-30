# React Wrapper Component

## Installation

```bash
npm install powerchart
# or use the WASM pkg directly from your build
```

## Component

```tsx
import { useEffect, useRef } from 'react';
import init, { PowerChart } from 'powerchart';

interface OhlcvBar {
  timestamp: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

interface ChartProps {
  data: OhlcvBar[];
  indicators?: { name: string; period?: number }[];
  markers?: {
    barIndex: number;
    shape: 'arrow_up' | 'arrow_down' | 'circle' | 'diamond';
    position: 'above' | 'below';
    color: [number, number, number];
    label: string;
  }[];
  width?: number;
  height?: number;
}

let wasmReady: Promise<void> | null = null;

export function PowerChartComponent({
  data,
  indicators = [],
  markers = [],
  width,
  height,
}: ChartProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const chartRef = useRef<PowerChart | null>(null);

  // Initialize WASM once
  useEffect(() => {
    if (!wasmReady) {
      wasmReady = init();
    }
  }, []);

  // Create chart
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    wasmReady!.then(() => {
      const dpr = window.devicePixelRatio || 1;
      canvas.width = Math.round(canvas.clientWidth * dpr);
      canvas.height = Math.round(canvas.clientHeight * dpr);

      chartRef.current = new PowerChart(canvas);
    });

    return () => {
      chartRef.current?.free();
      chartRef.current = null;
    };
  }, []);

  // Update data
  useEffect(() => {
    const chart = chartRef.current;
    if (!chart || data.length === 0) return;

    chart.setData(
      new Float64Array(data.map((d) => d.timestamp)),
      new Float64Array(data.map((d) => d.open)),
      new Float64Array(data.map((d) => d.high)),
      new Float64Array(data.map((d) => d.low)),
      new Float64Array(data.map((d) => d.close)),
      new Float64Array(data.map((d) => d.volume)),
    );
  }, [data]);

  // Update indicators
  useEffect(() => {
    const chart = chartRef.current;
    if (!chart) return;

    chart.clearIndicators();
    for (const ind of indicators) {
      chart.addIndicator(ind.name, ind.period ?? null);
    }
  }, [indicators]);

  // Update markers
  useEffect(() => {
    const chart = chartRef.current;
    if (!chart) return;

    chart.clearMarkers();
    for (const m of markers) {
      chart.addMarker(
        m.barIndex,
        m.shape,
        m.position,
        m.color[0],
        m.color[1],
        m.color[2],
        m.label,
      );
    }
  }, [markers]);

  // Handle resize
  useEffect(() => {
    const canvas = canvasRef.current;
    const chart = chartRef.current;
    if (!canvas || !chart) return;

    const observer = new ResizeObserver(() => {
      const dpr = window.devicePixelRatio || 1;
      const w = Math.round(canvas.clientWidth * dpr);
      const h = Math.round(canvas.clientHeight * dpr);
      canvas.width = w;
      canvas.height = h;
      chart.resize(w, h);
    });

    observer.observe(canvas);
    return () => observer.disconnect();
  }, []);

  return (
    <canvas
      ref={canvasRef}
      style={{
        width: width ?? '100%',
        height: height ?? 500,
        cursor: 'crosshair',
      }}
    />
  );
}
```

## Usage in SMR

```tsx
import { PowerChartComponent } from './PowerChart';

function TradingView({ ohlcv, patterns }) {
  return (
    <PowerChartComponent
      data={ohlcv}
      indicators={[
        { name: 'sma', period: 20 },
        { name: 'ema', period: 50 },
        { name: 'rsi', period: 14 },
      ]}
      markers={patterns.map((p, i) => ({
        barIndex: p.barIndex,
        shape: p.bullish ? 'arrow_up' : 'arrow_down',
        position: p.bullish ? 'below' : 'above',
        color: p.bullish ? [0, 200, 0] : [200, 0, 0],
        label: p.patternName,
      }))}
    />
  );
}
```
