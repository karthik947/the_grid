use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum KlineSource {
    Open,
    High,
    Low,
    Close,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Pair(pub String);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Price(pub f64);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Volume(pub f64);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub i64); // ms

mod ring_buffer;

pub use ring_buffer::*;

#[derive(Clone, Copy, Debug)]
pub struct Kline {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub open_time: i64, // in ms
    pub closed: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct AggregatedBar {
    pub closed_agg: Option<Kline>, // aggregate of closed 1m bars in this tf window
    pub open_bar: Option<Kline>,   // current 1m open bar for this tf window
}

pub type Bar1m = Kline;

mod timeframe;

pub use timeframe::Timeframe;

pub mod config;

pub use config::AppConfig;
