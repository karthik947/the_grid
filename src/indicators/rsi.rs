use log::info;
use std::collections::HashMap;

use crate::types::Kline;
use crate::{
    message_bus::KlineHist,
    time::now_millis,
    types::{Bar1m, Pair, RingBuffer, Timeframe},
};

use super::indicator::Indicator;

#[derive(Debug, Clone)]
pub struct Rsi {
    stage: Stage,
    pair: Pair,
    tf: Timeframe,
    period: usize,
    buffer: RingBuffer<Bar1m>,
    window_1m_bars: Option<RingBuffer<Bar1m>>,
    aggr_closed_bars: Option<BarAggregation>, // aggregation of closed 1m bars of the current bar
    previous_bar: Option<PreviousBar>,
    value: Option<f32>,
}

#[derive(Debug, Clone)]
struct PreviousBar {
    close: f64,
    avg_gain: f64,
    avg_loss: f64,
}

#[derive(Debug, Clone)]
struct BarAggregation {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

#[derive(Debug, Clone)]
pub enum Stage {
    New,
    WarmUp,
    Ready,
}

impl Rsi {
    pub fn new(period: usize, tf: &Timeframe, pair: &Pair) -> Self {
        Self {
            stage: Stage::New,
            pair: pair.clone(),
            period,
            tf: *tf,
            buffer: RingBuffer::new(10),
            window_1m_bars: None,
            aggr_closed_bars: None,
            previous_bar: None,
            value: None,
        }
    }
    fn update_window(&mut self, bar: Bar1m) {
        //The window_1m_bars is already set to RingBugger in set_window_1m_bars_from_history
        //So assume that, and don't set again here.
        let now_ms = now_millis();
        let current_tf_open = self.tf.nearest_ms(now_ms);
        let mut update_prev_from: Option<Bar1m> = None;

        {
            let Some(window) = self.window_1m_bars.as_mut() else {
                return;
            };

            let last = window.back().copied();
            match last {
                None => {
                    window.push(bar);
                    return;
                }
                Some(last) if last.open_time == bar.open_time => {
                    window.replace_last(bar);
                    return;
                }
                Some(last) if last.open_time > bar.open_time => {
                    return;
                }
                Some(last) => {
                    if window
                        .front()
                        .map(|first| first.open_time < current_tf_open)
                        .unwrap_or(false)
                    {
                        // capture last before trimming to update previous_bar later
                        update_prev_from = Some(last);
                        window.retain_by_open_time(|ts| ts >= current_tf_open);
                    }
                    window.push(bar);
                    self.set_aggregate();
                }
            }
        }

        if let Some(last_before_trim) = update_prev_from {
            self.update_previous_bar_from_last(last_before_trim);
        }
    }
    fn update_previous_bar_from_last(&mut self, last: Bar1m) {
        let Some(prev) = self.previous_bar.as_ref() else {
            return;
        };

        let diff = last.close - prev.close;
        let gain = if diff > 0.0 { diff } else { 0.0 };
        let loss = if diff < 0.0 { -diff } else { 0.0 };
        let period = self.period as f64;
        let avg_gain = (prev.avg_gain * (period - 1.0) + gain) / period;
        let avg_loss = (prev.avg_loss * (period - 1.0) + loss) / period;

        self.previous_bar = Some(PreviousBar {
            close: last.close,
            avg_gain,
            avg_loss,
        });
    }
    fn set_value(&mut self) {
        let Some(window) = self.window_1m_bars.as_ref() else {
            return;
        };
        let Some(last) = window.back() else {
            return;
        };
        let Some(prev) = self.previous_bar.as_ref() else {
            return;
        };

        let diff = last.close - prev.close;
        let gain = if diff > 0.0 { diff } else { 0.0 };
        let loss = if diff < 0.0 { -diff } else { 0.0 };

        let period = self.period as f64;
        let avg_gain = (prev.avg_gain * (period - 1.0) + gain) / period;
        let avg_loss = (prev.avg_loss * (period - 1.0) + loss) / period;

        let value = if avg_loss == 0.0 {
            100.0
        } else {
            let rs = avg_gain / avg_loss;
            100.0 - (100.0 / (1.0 + rs))
        };

        self.value = Some(value as f32);
    }
    fn set_previous_bar_from_history(&mut self, input: &KlineHist) {
        let now_ms = now_millis();
        let tf = input.indicator_tf;
        let prev_open = tf.nearest_ms(now_ms) - tf.window_millis();

        let closes: Vec<f64> = input
            .hist_tf
            .iter()
            .filter(|bar| bar.open_time <= prev_open)
            .map(|bar| bar.close)
            .collect();

        if closes.len() < self.period + 1 {
            self.previous_bar = None;
            return;
        }

        let mut gains = 0.0;
        let mut losses = 0.0;

        for i in 1..=self.period {
            let diff = closes[i] - closes[i - 1];
            if diff >= 0.0 {
                gains += diff;
            } else {
                losses -= diff;
            }
        }

        let mut avg_gain = gains / self.period as f64;
        let mut avg_loss = losses / self.period as f64;

        for i in (self.period + 1)..closes.len() {
            let diff = closes[i] - closes[i - 1];
            let gain = if diff > 0.0 { diff } else { 0.0 };
            let loss = if diff < 0.0 { -diff } else { 0.0 };

            avg_gain = (avg_gain * (self.period as f64 - 1.0) + gain) / self.period as f64;
            avg_loss = (avg_loss * (self.period as f64 - 1.0) + loss) / self.period as f64;
        }

        if let Some(close) = closes.last().copied() {
            self.previous_bar = Some(PreviousBar {
                close,
                avg_gain,
                avg_loss,
            });
        } else {
            self.previous_bar = None;
        }
    }
    fn set_aggregate(&mut self) {
        if self.tf == Timeframe::M1 {
            self.aggr_closed_bars = None;
            return;
        }

        let Some(window) = self.window_1m_bars.as_ref() else {
            self.aggr_closed_bars = None;
            return;
        };

        let mut bars = window.iter_without_last();

        let Some(first_bar) = bars.next() else {
            self.aggr_closed_bars = None;
            return;
        };

        let mut open = first_bar.open;
        let mut close = first_bar.close;
        let mut high = first_bar.high;
        let mut low = first_bar.low;
        let mut volume = first_bar.volume;

        for bar in bars {
            if bar.high > high {
                high = bar.high;
            }
            if bar.low < low {
                low = bar.low;
            }
            close = bar.close;
            volume += bar.volume;
        }

        self.aggr_closed_bars = Some(BarAggregation {
            open,
            high,
            low,
            close,
            volume,
        });
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
    fn set_window_1m_bars_from_history(&mut self, input: &KlineHist) {
        let now_ms = now_millis();
        let tf = input.indicator_tf;
        let window_size = tf.window_minutes();
        let window_1m_size = Timeframe::M1.window_millis();
        let current_1m_start = Timeframe::M1.nearest_ms(now_ms);
        let window_start_ts = tf.nearest_ms(now_ms);
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
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RsiInput {
    pub bar_1m: Bar1m,
}

impl Indicator for Rsi {
    type Input = RsiInput;
    type Output = Option<f32>;

    fn update_khist(&mut self, input: KlineHist) {
        self.set_window_1m_bars_from_history(&input);
        self.buffer.clear();
        self.set_aggregate();
        self.set_previous_bar_from_history(&input);
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
