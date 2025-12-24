use std::collections::HashMap;

use log::info;

use crate::time::now_millis;
use crate::{
    message_bus::KlineHist,
    types::{Bar1m, Pair, RingBuffer, Timeframe},
};

use super::indicator::Indicator;

#[derive(Debug, Clone)]
pub struct Volatility {
    stage: Stage,
    pair: Pair,
    tf: Timeframe,
    buffer: RingBuffer<Bar1m>,
    window_1m_bars: Option<RingBuffer<Bar1m>>,
    aggr_closed_bars: Option<BarAggregation>,
    value: Option<f32>,
}

#[derive(Debug, Clone)]
struct BarAggregation {
    high: f64,
    high_ts: i64,
    open: f64,
    low: f64,
    low_ts: i64,
}

#[derive(Debug, Clone)]
pub enum Stage {
    New,
    WarmUp,
    Ready,
}

impl Volatility {
    pub fn new(tf: &Timeframe, pair: &Pair) -> Self {
        Self {
            stage: Stage::New,
            pair: pair.clone(),
            tf: *tf,
            buffer: RingBuffer::new(10),
            window_1m_bars: None,
            aggr_closed_bars: None,
            value: None,
        }
    }
    fn update_window(&mut self, bar: Bar1m) -> Option<f32> {
        let Some(window) = self.window_1m_bars.as_mut() else {
            return None;
        };

        match window.back() {
            None => {
                window.push(bar);
            }
            Some(last) if last.open_time < bar.open_time => {
                window.push(bar);
                self.set_aggregate();
            }
            Some(last) if !last.closed && last.open_time == bar.open_time => {
                window.replace_last(bar);
            }
            _ => {}
        }

        None
    }
    fn set_value(&mut self) {
        let Some(window) = self.window_1m_bars.as_ref() else {
            return;
        };
        let Some(last) = window.back() else {
            return;
        };

        if let Some(aggr) = self.aggr_closed_bars.as_ref() {
            let high = aggr.high.max(last.high);
            let low = aggr.low.min(last.low);
            let open = aggr.open;
            let sign = if aggr.high_ts > aggr.low_ts {
                1.0
            } else {
                -1.0
            };
            let extent = ((high - low) / low) * 100.0 * sign; // +/- fraction
            let value = (extent * 10000.0).trunc() / 10000.0; // 4-decimals
            self.value = Some(value as f32);
        } else {
            let sign = if last.open < last.close { 1.0 } else { -1.0 };
            let extent = ((last.high - last.low) / last.low) * 100.0 * sign; // +/- fraction
            let value = (extent * 10000.0).trunc() / 10000.0; // 4-decimals
            self.value = Some(value as f32);
        }
    }
    fn set_aggregate(&mut self) {
        self.aggr_closed_bars = if self.tf == Timeframe::M1 {
            None
        } else {
            let now_ms = now_millis();
            let current_1m_start = Timeframe::M1.nearest_ms(now_ms);
            let Some(window) = self.window_1m_bars.as_ref() else {
                return;
            };

            let Some(first_bar) = window.front() else {
                return;
            };

            let open = first_bar.open;
            let mut high: Option<(f64, i64)> = None;
            let mut low: Option<(f64, i64)> = None;

            for bar in window.iter() {
                if bar.open_time == current_1m_start || !bar.closed {
                    continue;
                }

                match high {
                    Some((h, _)) if bar.high > h => high = Some((bar.high, bar.open_time)),
                    None => high = Some((bar.high, bar.open_time)),
                    _ => {}
                }

                match low {
                    Some((l, _)) if bar.low < l => low = Some((bar.low, bar.open_time)),
                    None => low = Some((bar.low, bar.open_time)),
                    _ => {}
                }
            }

            match (high, low) {
                (Some((high, high_ts)), Some((low, low_ts))) => Some(BarAggregation {
                    high,
                    high_ts,
                    low,
                    low_ts,
                    open,
                }),
                _ => None,
            }
        };
    }
    fn update_buffer(&mut self, bar: Bar1m) {
        match self.buffer.back() {
            None => {
                self.buffer.push(bar);
            }
            Some(last) if last.open_time < bar.open_time => {
                self.buffer.push(bar);
            }
            Some(last) if !last.closed && last.open_time == bar.open_time => {
                self.buffer.replace_last(bar);
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VolatilityInput {
    pub bar_1m: Bar1m,
}

impl Indicator for Volatility {
    type Input = VolatilityInput;
    type Output = Option<f32>;

    fn update_khist(&mut self, input: KlineHist) {
        let now_ms = now_millis();
        let tf = input.indicator_tf;
        let window_size = tf.window_minutes();
        let window_1m_size = Timeframe::M1.window_millis();
        let current_1m_start = Timeframe::M1.nearest_ms(now_ms);
        let window_start_ts = current_1m_start - (window_size as i64 - 1) * window_1m_size;

        let ws_by_open: HashMap<i64, Bar1m> = self
            .buffer
            .iter()
            .map(|bar| (bar.open_time, *bar))
            .collect();
        let api_by_open: HashMap<i64, Bar1m> = input
            .hist_1m
            .iter()
            .map(|bar| (bar.open_time, *bar))
            .collect();

        let mut window = RingBuffer::new(window_size);
        for idx in 0..window_size {
            let expected_open = window_start_ts + (idx as i64) * window_1m_size;

            if let Some(bar) = ws_by_open.get(&expected_open) {
                window.push(*bar);
                continue;
            }

            if let Some(bar) = api_by_open.get(&expected_open) {
                window.push(*bar);
            }
        }

        self.window_1m_bars = Some(window);
        self.set_aggregate();
        self.buffer.clear();
        self.set_value();
        self.stage = Stage::Ready;
    }

    fn update(&mut self, input: Self::Input) -> Self::Output {
        match self.stage {
            Stage::New => {
                self.stage = Stage::WarmUp;

                self.update_buffer(input.bar_1m);
            }
            Stage::WarmUp => {
                self.update_buffer(input.bar_1m);
            }
            Stage::Ready => {
                self.update_window(input.bar_1m);
                self.set_value();
                return self.value;
            }
        }

        None
    }
}
