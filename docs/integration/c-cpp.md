# FerroChart -- C/C++ Integration Guide

> **Applies to:** `ferrochart-ffi` 0.1.0+

## Overview

`ferrochart-ffi` exposes the full FerroChart rendering pipeline through a
C-compatible API (`extern "C"`). This makes it usable from **C, C++, and any
language with C FFI support** (Python, C#, Go, etc.).

The library ships as:

| Artifact | Platform |
|---|---|
| `libferrochart_ffi.so` | Linux |
| `libferrochart_ffi.dylib` | macOS |
| `ferrochart_ffi.dll` | Windows |
| `libferrochart_ffi.a` | Static (all platforms) |

## Building

```bash
# Shared library (cdylib) + static library
cargo build --release -p ferrochart-ffi

# Output:
#   target/release/libferrochart_ffi.so      (Linux)
#   target/release/libferrochart_ffi.dylib   (macOS)
#   target/release/ferrochart_ffi.dll        (Windows)
#   target/release/libferrochart_ffi.a       (all)
```

### Regenerate the C header

```bash
cbindgen --crate ferrochart-ffi \
         --config crates/ffi/cbindgen.toml \
         --output crates/ffi/ferrochart.h
```

The generated header is at `crates/ffi/ferrochart.h`.

## Quick Start (C)

```c
#include "ferrochart.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    // 1. Create chart
    FcChart *chart = fc_chart_create();

    // 2. Configure
    fc_chart_set_size(chart, 1200.0, 600.0);
    fc_chart_set_theme_dark(chart);
    fc_chart_set_type(chart, "candlestick");

    // 3. Load data (parallel arrays)
    int64_t  ts[] = {1700000000, 1700086400, 1700172800, 1700259200, 1700345600};
    double open[] = {100.0, 102.0, 101.0, 105.0, 103.0};
    double high[] = {105.0, 106.0, 107.0, 108.0, 109.0};
    double  low[] = { 98.0,  99.0,  98.5, 101.0, 100.0};
    double  cls[] = {102.0, 101.0, 105.0, 103.0, 107.0};
    double  vol[] = {1000.0, 1100.0, 1200.0, 900.0, 1500.0};

    fc_chart_set_data(chart, ts, open, high, low, cls, vol, 5);

    // 4. Add indicators
    fc_chart_add_indicator(chart, "sma", 3);
    fc_chart_add_indicator(chart, "rsi", 14);

    // 5. Add a buy signal marker
    fc_chart_add_marker(chart, 2, 0 /* ArrowUp */, 1 /* BelowBar */,
                        0, 255, 0, "BUY");

    // 6. Render to SVG
    char *svg = fc_chart_render_svg(chart);
    if (svg) {
        FILE *f = fopen("chart.svg", "w");
        fputs(svg, f);
        fclose(f);
        fc_string_free(svg);  // IMPORTANT: free library-allocated strings
    }

    // 7. Cleanup
    fc_chart_destroy(chart);
    return 0;
}
```

### Compile & link

```bash
# Linux
gcc -o chart_demo chart_demo.c -L target/release -lferrochart_ffi -lpthread -ldl -lm

# macOS
clang -o chart_demo chart_demo.c -L target/release -lferrochart_ffi

# Run
LD_LIBRARY_PATH=target/release ./chart_demo   # Linux
DYLD_LIBRARY_PATH=target/release ./chart_demo  # macOS
```

## Quick Start (C++)

```cpp
#include "ferrochart.h"
#include <fstream>
#include <string>
#include <vector>
#include <memory>

// RAII wrapper for FcChart
struct ChartDeleter {
    void operator()(FcChart* c) const { fc_chart_destroy(c); }
};
using ChartPtr = std::unique_ptr<FcChart, ChartDeleter>;

// RAII wrapper for library-allocated strings
struct StringDeleter {
    void operator()(char* s) const { fc_string_free(s); }
};
using FcString = std::unique_ptr<char, StringDeleter>;

int main() {
    ChartPtr chart(fc_chart_create());

    fc_chart_set_size(chart.get(), 1200.0, 600.0);
    fc_chart_set_theme_dark(chart.get());

    // Data from vectors
    std::vector<int64_t> ts  = {1700000000, 1700086400, 1700172800};
    std::vector<double>  o   = {100.0, 102.0, 101.0};
    std::vector<double>  h   = {105.0, 106.0, 107.0};
    std::vector<double>  l   = { 98.0,  99.0,  98.5};
    std::vector<double>  c   = {102.0, 101.0, 105.0};
    std::vector<double>  vol = {1000.0, 1100.0, 1200.0};

    fc_chart_set_data(chart.get(),
                      ts.data(), o.data(), h.data(),
                      l.data(), c.data(), vol.data(),
                      ts.size());

    fc_chart_add_indicator(chart.get(), "bollinger", 20);
    fc_chart_add_indicator(chart.get(), "macd", 0);

    FcString svg(fc_chart_render_svg(chart.get()));
    if (svg) {
        std::ofstream out("chart.svg");
        out << svg.get();
    }

    return 0;
}
```

## API Reference

### Lifecycle

| Function | Description |
|---|---|
| `FcChart* fc_chart_create()` | Create a new chart with default config |
| `void fc_chart_destroy(FcChart*)` | Free a chart handle |

### Data

| Function | Description |
|---|---|
| `void fc_chart_set_data(…, len)` | Set OHLCV from parallel arrays |
| `int32_t fc_chart_set_data_json(…, json)` | Set OHLCV from JSON string |
| `uintptr_t fc_chart_bar_count(…)` | Number of loaded bars |

### Configuration

| Function | Description |
|---|---|
| `void fc_chart_set_size(…, w, h)` | Set chart dimensions (pixels) |
| `int32_t fc_chart_set_type(…, name)` | Chart type: `"candlestick"`, `"heikin_ashi"`, `"ohlc"`, `"line"`, `"area"`, `"renko"`, `"point_figure"` |
| `void fc_chart_set_renko_brick_size(…, size)` | Renko brick size |
| `void fc_chart_set_pf_config(…, box, rev)` | Point & Figure config |
| `void fc_chart_set_theme_dark(…)` | Dark theme |
| `void fc_chart_set_theme_light(…)` | Light theme |
| `void fc_chart_set_log_scale(…, bool)` | Log/linear Y-axis |
| `int32_t fc_chart_set_config_json(…, json)` | Full config as JSON |

### Indicators (18 built-in)

| Function | Description |
|---|---|
| `int32_t fc_chart_add_indicator(…, name, period)` | Add indicator by name |
| `void fc_chart_clear_indicators(…)` | Remove all indicators |

Supported names: `sma`, `ema`, `bollinger`, `rsi`, `macd`, `atr`, `obv`,
`stochastic`, `williams_r`, `cci`, `adx`, `donchian`, `keltner`,
`parabolic_sar`, `supertrend`, `ichimoku`, `session_vwap`, `volume_sma`.

### Markers

| Function | Description |
|---|---|
| `void fc_chart_add_marker(…)` | Add buy/sell signal marker |
| `void fc_chart_clear_markers(…)` | Remove all markers |

Shape: 0=ArrowUp, 1=ArrowDown, 2=Circle, 3=Diamond.
Position: 0=AboveBar, 1=BelowBar.

### Annotations (JSON)

| Function | Description |
|---|---|
| `int32_t fc_chart_import_annotations(…, json)` | Import from JSON |
| `char* fc_chart_export_annotations(…)` | Export as JSON (caller frees) |
| `void fc_chart_clear_annotations(…)` | Remove all |

### Rendering

| Function | Description |
|---|---|
| `char* fc_chart_render_svg(…)` | Render to SVG string (caller frees) |

### Utilities

| Function | Description |
|---|---|
| `void fc_string_free(char*)` | Free a library-allocated string |
| `const char* fc_version()` | Library version (static, do NOT free) |

## Memory Management Rules

1. **Strings returned by the library** (e.g. `fc_chart_render_svg`,
   `fc_chart_export_annotations`) must be freed with `fc_string_free()`.
2. **The version string** from `fc_version()` is static -- do NOT free it.
3. **The chart handle** must be freed with `fc_chart_destroy()`.
4. **Input strings** (JSON, indicator names, etc.) are borrowed -- the library
   does not take ownership.

## Chart Types

| Name | Description |
|---|---|
| `"candlestick"` | Standard OHLC candlesticks |
| `"heikin_ashi"` | Heikin-Ashi smoothed candles |
| `"ohlc"` | OHLC bar chart |
| `"line"` | Close-price line chart |
| `"area"` | Filled area chart |
| `"renko"` | Renko bricks (configure with `fc_chart_set_renko_brick_size`) |
| `"point_figure"` | Point & Figure (configure with `fc_chart_set_pf_config`) |

## JSON Data Format

For `fc_chart_set_data_json`:

```json
[
  {"timestamp": 1700000000, "open": 100.0, "high": 105.0, "low": 98.0, "close": 102.0, "volume": 1000.0},
  {"timestamp": 1700086400, "open": 102.0, "high": 106.0, "low": 99.0, "close": 101.0, "volume": 1100.0}
]
```

Optional field: `"institutional_ratio": 0.6` (0.0--1.0, for split-candle rendering).

## Thread Safety

`FcChart` is **not** thread-safe. Each handle must be used from a single thread
at a time. Create separate handles for concurrent rendering.

## Error Handling

Functions that can fail return `int32_t`:
- `0` = success
- `-1` = error (invalid input, parse failure)

Functions returning `char*` return `NULL` on failure (e.g. rendering with no data).
