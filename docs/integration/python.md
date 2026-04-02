# FerroChart -- Python Integration Guide

> **Applies to:** `ferrochart-ffi` 0.1.0+

## Overview

FerroChart can be used from Python via `ctypes` (standard library) or `cffi`.
The FFI crate (`ferrochart-ffi`) produces a shared library that Python loads
at runtime.

## Building the Shared Library

```bash
cargo build --release -p ferrochart-ffi
```

Output:
- Linux: `target/release/libferrochart_ffi.so`
- macOS: `target/release/libferrochart_ffi.dylib`
- Windows: `target/release/ferrochart_ffi.dll`

## Quick Start (ctypes)

```python
import ctypes
import json
import platform
from pathlib import Path

# --- Load library -----------------------------------------------------------

def _load_lib():
    base = Path(__file__).resolve().parent / "target" / "release"
    system = platform.system()
    if system == "Linux":
        return ctypes.CDLL(str(base / "libferrochart_ffi.so"))
    elif system == "Darwin":
        return ctypes.CDLL(str(base / "libferrochart_ffi.dylib"))
    elif system == "Windows":
        return ctypes.CDLL(str(base / "ferrochart_ffi.dll"))
    raise RuntimeError(f"Unsupported platform: {system}")

lib = _load_lib()

# --- Declare function signatures ---------------------------------------------

# Opaque handle
class FcChart(ctypes.Structure):
    pass

FcChartPtr = ctypes.POINTER(FcChart)

lib.fc_chart_create.restype = FcChartPtr
lib.fc_chart_create.argtypes = []

lib.fc_chart_destroy.restype = None
lib.fc_chart_destroy.argtypes = [FcChartPtr]

lib.fc_chart_set_data.restype = None
lib.fc_chart_set_data.argtypes = [
    FcChartPtr,
    ctypes.POINTER(ctypes.c_int64),   # timestamps
    ctypes.POINTER(ctypes.c_double),   # open
    ctypes.POINTER(ctypes.c_double),   # high
    ctypes.POINTER(ctypes.c_double),   # low
    ctypes.POINTER(ctypes.c_double),   # close
    ctypes.POINTER(ctypes.c_double),   # volume
    ctypes.c_size_t,                   # len
]

lib.fc_chart_set_data_json.restype = ctypes.c_int32
lib.fc_chart_set_data_json.argtypes = [FcChartPtr, ctypes.c_char_p]

lib.fc_chart_set_size.restype = None
lib.fc_chart_set_size.argtypes = [FcChartPtr, ctypes.c_double, ctypes.c_double]

lib.fc_chart_set_type.restype = ctypes.c_int32
lib.fc_chart_set_type.argtypes = [FcChartPtr, ctypes.c_char_p]

lib.fc_chart_set_theme_dark.restype = None
lib.fc_chart_set_theme_dark.argtypes = [FcChartPtr]

lib.fc_chart_set_theme_light.restype = None
lib.fc_chart_set_theme_light.argtypes = [FcChartPtr]

lib.fc_chart_set_log_scale.restype = None
lib.fc_chart_set_log_scale.argtypes = [FcChartPtr, ctypes.c_bool]

lib.fc_chart_add_indicator.restype = ctypes.c_int32
lib.fc_chart_add_indicator.argtypes = [FcChartPtr, ctypes.c_char_p, ctypes.c_size_t]

lib.fc_chart_clear_indicators.restype = None
lib.fc_chart_clear_indicators.argtypes = [FcChartPtr]

lib.fc_chart_add_marker.restype = None
lib.fc_chart_add_marker.argtypes = [
    FcChartPtr, ctypes.c_size_t, ctypes.c_uint32, ctypes.c_uint32,
    ctypes.c_uint8, ctypes.c_uint8, ctypes.c_uint8, ctypes.c_char_p,
]

lib.fc_chart_clear_markers.restype = None
lib.fc_chart_clear_markers.argtypes = [FcChartPtr]

lib.fc_chart_import_annotations.restype = ctypes.c_int32
lib.fc_chart_import_annotations.argtypes = [FcChartPtr, ctypes.c_char_p]

lib.fc_chart_export_annotations.restype = ctypes.c_char_p
lib.fc_chart_export_annotations.argtypes = [FcChartPtr]

lib.fc_chart_clear_annotations.restype = None
lib.fc_chart_clear_annotations.argtypes = [FcChartPtr]

lib.fc_chart_render_svg.restype = ctypes.c_char_p
lib.fc_chart_render_svg.argtypes = [FcChartPtr]

lib.fc_chart_bar_count.restype = ctypes.c_size_t
lib.fc_chart_bar_count.argtypes = [FcChartPtr]

lib.fc_string_free.restype = None
lib.fc_string_free.argtypes = [ctypes.c_char_p]

lib.fc_version.restype = ctypes.c_char_p
lib.fc_version.argtypes = []
```

## High-Level Wrapper

```python
class FerroChart:
    """High-level Python wrapper around the FerroChart FFI."""

    # Marker shapes
    ARROW_UP = 0
    ARROW_DOWN = 1
    CIRCLE = 2
    DIAMOND = 3

    # Marker positions
    ABOVE_BAR = 0
    BELOW_BAR = 1

    def __init__(self, width: float = 900.0, height: float = 600.0):
        self._handle = lib.fc_chart_create()
        lib.fc_chart_set_size(self._handle, width, height)

    def __del__(self):
        if hasattr(self, "_handle") and self._handle:
            lib.fc_chart_destroy(self._handle)

    def __enter__(self):
        return self

    def __exit__(self, *_):
        lib.fc_chart_destroy(self._handle)
        self._handle = None

    def set_data(self, timestamps, open_, high, low, close, volume):
        """Set OHLCV data from Python lists or numpy arrays."""
        n = len(timestamps)
        ts  = (ctypes.c_int64  * n)(*timestamps)
        o   = (ctypes.c_double * n)(*open_)
        h   = (ctypes.c_double * n)(*high)
        l   = (ctypes.c_double * n)(*low)
        c   = (ctypes.c_double * n)(*close)
        v   = (ctypes.c_double * n)(*volume)
        lib.fc_chart_set_data(self._handle, ts, o, h, l, c, v, n)

    def set_data_json(self, bars: list[dict]):
        """Set OHLCV data from a list of dicts."""
        raw = json.dumps(bars).encode("utf-8")
        rc = lib.fc_chart_set_data_json(self._handle, raw)
        if rc != 0:
            raise ValueError("Failed to parse OHLCV JSON")

    def set_type(self, name: str):
        """Set chart type: candlestick, heikin_ashi, ohlc, line, area, renko, point_figure."""
        rc = lib.fc_chart_set_type(self._handle, name.encode("utf-8"))
        if rc != 0:
            raise ValueError(f"Unknown chart type: {name}")

    def set_theme(self, theme: str):
        """Set theme: 'dark' or 'light'."""
        if theme == "dark":
            lib.fc_chart_set_theme_dark(self._handle)
        elif theme == "light":
            lib.fc_chart_set_theme_light(self._handle)
        else:
            raise ValueError(f"Unknown theme: {theme}")

    def set_log_scale(self, enabled: bool):
        lib.fc_chart_set_log_scale(self._handle, enabled)

    def add_indicator(self, name: str, period: int = 14):
        """Add a technical indicator. Returns self for chaining."""
        rc = lib.fc_chart_add_indicator(
            self._handle, name.encode("utf-8"), period
        )
        if rc != 0:
            raise ValueError(f"Unknown indicator: {name}")
        return self

    def clear_indicators(self):
        lib.fc_chart_clear_indicators(self._handle)

    def add_marker(self, bar_index: int, shape: int = 0,
                   position: int = 1, color=(0, 255, 0), label: str = ""):
        r, g, b = color
        lib.fc_chart_add_marker(
            self._handle, bar_index, shape, position,
            r, g, b, label.encode("utf-8") if label else None,
        )

    def clear_markers(self):
        lib.fc_chart_clear_markers(self._handle)

    def render_svg(self) -> str:
        """Render chart to SVG string."""
        ptr = lib.fc_chart_render_svg(self._handle)
        if not ptr:
            raise RuntimeError("No data loaded")
        svg = ptr.decode("utf-8")
        # Note: ctypes returns a copy for c_char_p, but we call free to be safe
        # with the raw pointer approach. For c_char_p restype, ctypes manages it.
        return svg

    def save_svg(self, path: str):
        """Render and save to file."""
        svg = self.render_svg()
        Path(path).write_text(svg)

    @staticmethod
    def version() -> str:
        return lib.fc_version().decode("utf-8")
```

## Usage Example

```python
# pip install numpy pandas  (optional, for data loading)

with FerroChart(width=1200, height=600) as chart:
    chart.set_theme("dark")
    chart.set_type("candlestick")

    # Load data
    chart.set_data(
        timestamps=[1700000000 + i * 86400 for i in range(50)],
        open_= [100 + i * 0.5 for i in range(50)],
        high  = [102 + i * 0.5 for i in range(50)],
        low   = [ 98 + i * 0.5 for i in range(50)],
        close = [101 + i * 0.5 for i in range(50)],
        volume= [1000 + i * 10 for i in range(50)],
    )

    # Add indicators
    chart.add_indicator("sma", 10)
    chart.add_indicator("bollinger", 20)
    chart.add_indicator("rsi", 14)

    # Add buy/sell markers
    chart.add_marker(15, FerroChart.ARROW_UP, FerroChart.BELOW_BAR,
                     color=(0, 255, 0), label="BUY")
    chart.add_marker(35, FerroChart.ARROW_DOWN, FerroChart.ABOVE_BAR,
                     color=(255, 0, 0), label="SELL")

    # Render
    chart.save_svg("my_chart.svg")
    print(f"FerroChart {FerroChart.version()} -- chart saved!")
```

## With Pandas / NumPy

```python
import pandas as pd
import numpy as np

df = pd.read_csv("ohlcv.csv")  # columns: timestamp, open, high, low, close, volume

with FerroChart(1200, 600) as chart:
    chart.set_theme("dark")
    chart.set_data(
        timestamps=df["timestamp"].values.astype(np.int64),
        open_=df["open"].values,
        high=df["high"].values,
        low=df["low"].values,
        close=df["close"].values,
        volume=df["volume"].values,
    )
    chart.add_indicator("ichimoku", 0)
    chart.save_svg("ichimoku_chart.svg")
```

## Available Indicators

| Name | Period | Description |
|---|---|---|
| `sma` | Yes | Simple Moving Average |
| `ema` | Yes | Exponential Moving Average |
| `bollinger` | Yes | Bollinger Bands (2 std dev) |
| `rsi` | Yes | Relative Strength Index |
| `macd` | No | MACD (12, 26, 9) |
| `atr` | Yes | Average True Range |
| `obv` | No | On-Balance Volume |
| `stochastic` | Yes | Stochastic Oscillator |
| `williams_r` | Yes | Williams %R |
| `cci` | Yes | Commodity Channel Index |
| `adx` | Yes | ADX / DMI |
| `donchian` | Yes | Donchian Channels |
| `keltner` | Yes | Keltner Channels |
| `parabolic_sar` | No | Parabolic SAR |
| `supertrend` | Yes | Supertrend |
| `ichimoku` | No | Ichimoku Cloud (9, 26, 52) |
| `session_vwap` | No | Session VWAP |
| `volume_sma` | Yes | Volume SMA |

## Error Handling

FFI functions return integer error codes. The Python wrapper translates these
into `ValueError` exceptions. `render_svg()` raises `RuntimeError` when no
data is loaded.
