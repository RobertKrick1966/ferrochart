use super::{
    closes, compute_ema, Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeries,
    SeriesStyle,
};
use crate::Ohlcv;

/// Moving Average Convergence Divergence.
#[derive(Debug, Clone)]
pub struct Macd {
    pub fast_period: usize,
    pub slow_period: usize,
    pub signal_period: usize,
}

impl Indicator for Macd {
    fn name(&self) -> &'static str {
        "MACD"
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::SubPanelAuto
    }

    fn compute(&self, data: &[Ohlcv]) -> IndicatorOutput {
        let c = closes(data);
        let n = c.len();

        let fast_ema = compute_ema(&c, self.fast_period);
        let slow_ema = compute_ema(&c, self.slow_period);

        // MACD line = fast EMA - slow EMA
        let mut macd_line: Vec<f64> = (0..n)
            .map(|i| {
                if fast_ema[i].is_nan() || slow_ema[i].is_nan() {
                    f64::NAN
                } else {
                    fast_ema[i] - slow_ema[i]
                }
            })
            .collect();

        // Signal line = EMA of MACD line (skip NAN prefix)
        let first_valid = macd_line.iter().position(|v| !v.is_nan());
        let signal = if let Some(start) = first_valid {
            let valid_part = &macd_line[start..];
            let signal_ema = compute_ema(valid_part, self.signal_period);
            let mut full = vec![f64::NAN; start];
            full.extend_from_slice(&signal_ema);
            full
        } else {
            vec![f64::NAN; n]
        };

        // Histogram = MACD - signal
        let histogram: Vec<f64> = (0..n)
            .map(|i| {
                if macd_line[i].is_nan() || signal[i].is_nan() {
                    f64::NAN
                } else {
                    macd_line[i] - signal[i]
                }
            })
            .collect();

        // Set MACD line NaN where signal is not yet available for consistency
        if let Some(first_signal) = signal.iter().position(|v| !v.is_nan()) {
            for val in &mut macd_line[..first_signal] {
                *val = f64::NAN;
            }
        }

        IndicatorOutput {
            name: format!(
                "MACD({},{},{})",
                self.fast_period, self.slow_period, self.signal_period
            ),
            placement: self.placement(),
            series: vec![
                IndicatorSeries {
                    name: "MACD",
                    values: macd_line,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "signal",
                    values: signal,
                    style_hint: SeriesStyle::Line,
                },
                IndicatorSeries {
                    name: "histogram",
                    values: histogram,
                    style_hint: SeriesStyle::Histogram,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(close: f64) -> Ohlcv {
        Ohlcv {
            timestamp: 0,
            open: close,
            high: close,
            low: close,
            close,
            volume: 0.0,
        }
    }

    fn sample_data() -> Vec<Ohlcv> {
        // 40 bars of price data with a trend
        (0..40)
            .map(|i| {
                let price = 100.0 + f64::from(i) * 0.5 + (f64::from(i) * 0.3).sin() * 3.0;
                bar(price)
            })
            .collect()
    }

    #[test]
    fn macd_output_has_three_series() {
        let out = Macd {
            fast_period: 12,
            slow_period: 26,
            signal_period: 9,
        }
        .compute(&sample_data());
        assert_eq!(out.series.len(), 3);
        assert_eq!(out.series[0].name, "MACD");
        assert_eq!(out.series[1].name, "signal");
        assert_eq!(out.series[2].name, "histogram");
    }

    #[test]
    fn macd_histogram_equals_macd_minus_signal() {
        let out = Macd {
            fast_period: 12,
            slow_period: 26,
            signal_period: 9,
        }
        .compute(&sample_data());
        let macd = &out.series[0].values;
        let signal = &out.series[1].values;
        let hist = &out.series[2].values;

        for i in 0..macd.len() {
            if !macd[i].is_nan() && !signal[i].is_nan() {
                assert!(
                    (hist[i] - (macd[i] - signal[i])).abs() < 1e-9,
                    "histogram mismatch at {i}"
                );
            }
        }
    }

    #[test]
    fn macd_early_values_are_nan() {
        let out = Macd {
            fast_period: 12,
            slow_period: 26,
            signal_period: 9,
        }
        .compute(&sample_data());
        // Need at least slow_period bars before MACD line is valid
        for val in &out.series[0].values[..25] {
            assert!(val.is_nan());
        }
    }

    #[test]
    fn macd_empty_data() {
        let out = Macd {
            fast_period: 12,
            slow_period: 26,
            signal_period: 9,
        }
        .compute(&[]);
        assert!(out.series[0].values.is_empty());
    }

    #[test]
    fn macd_all_same_length() {
        let data = sample_data();
        let out = Macd {
            fast_period: 12,
            slow_period: 26,
            signal_period: 9,
        }
        .compute(&data);
        for s in &out.series {
            assert_eq!(s.values.len(), data.len());
        }
    }

    #[test]
    fn macd_placement_is_sub_panel_auto() {
        assert_eq!(
            Macd {
                fast_period: 12,
                slow_period: 26,
                signal_period: 9,
            }
            .placement(),
            IndicatorPlacement::SubPanelAuto
        );
    }
}
