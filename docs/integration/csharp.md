# FerroChart -- C# Integration Guide

> **Applies to:** `ferrochart-ffi` 0.1.0+

## Overview

FerroChart can be used from C# / .NET via P/Invoke. The FFI crate produces a
native shared library that .NET loads at runtime.

## Building the Shared Library

```bash
cargo build --release -p ferrochart-ffi
```

Output:
- Linux: `target/release/libferrochart_ffi.so`
- macOS: `target/release/libferrochart_ffi.dylib`
- Windows: `target/release/ferrochart_ffi.dll`

Copy the appropriate library next to your .NET executable or into a runtime
directory.

## P/Invoke Bindings

```csharp
using System;
using System.Runtime.InteropServices;

/// <summary>
/// Raw P/Invoke bindings for ferrochart-ffi.
/// </summary>
internal static class FerroChartNative
{
    // Adjust library name per platform.
    // On Windows: "ferrochart_ffi", Linux/macOS: "libferrochart_ffi"
    private const string LibName = "ferrochart_ffi";

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fc_chart_create();

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void fc_chart_destroy(IntPtr handle);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void fc_chart_set_data(
        IntPtr handle,
        long[] timestamps,
        double[] open,
        double[] high,
        double[] low,
        double[] close,
        double[] volume,
        UIntPtr len);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int fc_chart_set_data_json(
        IntPtr handle,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string json);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void fc_chart_set_size(
        IntPtr handle, double width, double height);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int fc_chart_set_type(
        IntPtr handle,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string name);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void fc_chart_set_renko_brick_size(
        IntPtr handle, double brickSize);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void fc_chart_set_pf_config(
        IntPtr handle, double boxSize, uint reversal);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void fc_chart_set_theme_dark(IntPtr handle);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void fc_chart_set_theme_light(IntPtr handle);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void fc_chart_set_log_scale(
        IntPtr handle, [MarshalAs(UnmanagedType.I1)] bool enabled);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int fc_chart_set_config_json(
        IntPtr handle,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string json);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int fc_chart_add_indicator(
        IntPtr handle,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string name,
        UIntPtr period);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void fc_chart_clear_indicators(IntPtr handle);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void fc_chart_add_marker(
        IntPtr handle,
        UIntPtr barIndex,
        uint shape,
        uint position,
        byte r, byte g, byte b,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string? label);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void fc_chart_clear_markers(IntPtr handle);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int fc_chart_import_annotations(
        IntPtr handle,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string json);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fc_chart_export_annotations(IntPtr handle);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void fc_chart_clear_annotations(IntPtr handle);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fc_chart_render_svg(IntPtr handle);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern UIntPtr fc_chart_bar_count(IntPtr handle);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void fc_string_free(IntPtr s);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fc_version();
}
```

## High-Level Wrapper

```csharp
using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Text.Json;

/// <summary>
/// High-level C# wrapper for FerroChart.
/// Implements IDisposable for deterministic cleanup.
/// </summary>
public sealed class FerroChart : IDisposable
{
    private IntPtr _handle;

    public enum MarkerShape : uint
    {
        ArrowUp = 0,
        ArrowDown = 1,
        Circle = 2,
        Diamond = 3,
    }

    public enum MarkerPosition : uint
    {
        AboveBar = 0,
        BelowBar = 1,
    }

    public FerroChart(double width = 900, double height = 600)
    {
        _handle = FerroChartNative.fc_chart_create();
        FerroChartNative.fc_chart_set_size(_handle, width, height);
    }

    public void Dispose()
    {
        if (_handle != IntPtr.Zero)
        {
            FerroChartNative.fc_chart_destroy(_handle);
            _handle = IntPtr.Zero;
        }
    }

    public void SetData(long[] timestamps, double[] open, double[] high,
                        double[] low, double[] close, double[] volume)
    {
        var len = (UIntPtr)timestamps.Length;
        FerroChartNative.fc_chart_set_data(
            _handle, timestamps, open, high, low, close, volume, len);
    }

    public void SetDataJson(string json)
    {
        if (FerroChartNative.fc_chart_set_data_json(_handle, json) != 0)
            throw new ArgumentException("Failed to parse OHLCV JSON");
    }

    public void SetType(string chartType)
    {
        if (FerroChartNative.fc_chart_set_type(_handle, chartType) != 0)
            throw new ArgumentException($"Unknown chart type: {chartType}");
    }

    public void SetThemeDark() => FerroChartNative.fc_chart_set_theme_dark(_handle);
    public void SetThemeLight() => FerroChartNative.fc_chart_set_theme_light(_handle);
    public void SetLogScale(bool enabled) =>
        FerroChartNative.fc_chart_set_log_scale(_handle, enabled);

    public void AddIndicator(string name, int period = 14)
    {
        if (FerroChartNative.fc_chart_add_indicator(
                _handle, name, (UIntPtr)period) != 0)
            throw new ArgumentException($"Unknown indicator: {name}");
    }

    public void ClearIndicators() =>
        FerroChartNative.fc_chart_clear_indicators(_handle);

    public void AddMarker(int barIndex, MarkerShape shape = MarkerShape.ArrowUp,
                          MarkerPosition position = MarkerPosition.BelowBar,
                          byte r = 0, byte g = 255, byte b = 0,
                          string? label = null)
    {
        FerroChartNative.fc_chart_add_marker(
            _handle, (UIntPtr)barIndex, (uint)shape, (uint)position,
            r, g, b, label);
    }

    public void ClearMarkers() =>
        FerroChartNative.fc_chart_clear_markers(_handle);

    public string RenderSvg()
    {
        IntPtr ptr = FerroChartNative.fc_chart_render_svg(_handle);
        if (ptr == IntPtr.Zero)
            throw new InvalidOperationException("No data loaded");
        string svg = Marshal.PtrToStringUTF8(ptr)!;
        FerroChartNative.fc_string_free(ptr);
        return svg;
    }

    public void SaveSvg(string path) => File.WriteAllText(path, RenderSvg());

    public int BarCount =>
        (int)FerroChartNative.fc_chart_bar_count(_handle);

    public static string Version
    {
        get
        {
            IntPtr ptr = FerroChartNative.fc_version();
            return Marshal.PtrToStringUTF8(ptr) ?? "unknown";
        }
    }
}
```

## Usage Example

```csharp
using var chart = new FerroChart(1200, 600);

chart.SetThemeDark();
chart.SetType("candlestick");

// Sample data
var timestamps = new long[] { 1700000000, 1700086400, 1700172800, 1700259200, 1700345600 };
var open  = new double[] { 100.0, 102.0, 101.0, 105.0, 103.0 };
var high  = new double[] { 105.0, 106.0, 107.0, 108.0, 109.0 };
var low   = new double[] {  98.0,  99.0,  98.5, 101.0, 100.0 };
var close = new double[] { 102.0, 101.0, 105.0, 103.0, 107.0 };
var vol   = new double[] { 1000.0, 1100.0, 1200.0, 900.0, 1500.0 };

chart.SetData(timestamps, open, high, low, close, vol);

// Add indicators
chart.AddIndicator("sma", 3);
chart.AddIndicator("rsi", 14);

// Add buy signal
chart.AddMarker(2, FerroChart.MarkerShape.ArrowUp,
                FerroChart.MarkerPosition.BelowBar,
                0, 255, 0, "BUY");

// Render
chart.SaveSvg("chart.svg");
Console.WriteLine($"FerroChart {FerroChart.Version} -- chart saved!");
```

## ASP.NET Integration

```csharp
// In a controller or minimal API endpoint:
app.MapGet("/chart.svg", () =>
{
    using var chart = new FerroChart(1200, 600);
    chart.SetThemeDark();
    // ... load data, add indicators ...
    var svg = chart.RenderSvg();
    return Results.Content(svg, "image/svg+xml");
});
```

## NativeLibrary Resolution (.NET 7+)

For cross-platform deployments, use `NativeLibrary.SetDllImportResolver`:

```csharp
using System.Reflection;
using System.Runtime.InteropServices;

NativeLibrary.SetDllImportResolver(
    Assembly.GetExecutingAssembly(),
    (libraryName, assembly, searchPath) =>
    {
        if (libraryName != "ferrochart_ffi") return IntPtr.Zero;

        string rid = RuntimeInformation.RuntimeIdentifier;
        string ext = RuntimeInformation.IsOSPlatform(OSPlatform.Windows)
            ? ".dll"
            : RuntimeInformation.IsOSPlatform(OSPlatform.OSX)
                ? ".dylib"
                : ".so";
        string prefix = ext == ".dll" ? "" : "lib";
        string path = Path.Combine("runtimes", rid, "native",
                                   $"{prefix}ferrochart_ffi{ext}");
        return NativeLibrary.Load(path, assembly, searchPath);
    });
```

## Available Indicators

| Name | Period | Description |
|---|---|---|
| `sma` | Yes | Simple Moving Average |
| `ema` | Yes | Exponential Moving Average |
| `bollinger` | Yes | Bollinger Bands |
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
| `ichimoku` | No | Ichimoku Cloud |
| `session_vwap` | No | Session VWAP |
| `volume_sma` | Yes | Volume SMA |

## Thread Safety

`FerroChart` instances are **not** thread-safe. Use separate instances per
thread, or synchronize access externally.
