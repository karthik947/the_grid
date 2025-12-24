mod indicator;
mod rsi;
mod volatility;

use std::collections::HashMap;

use crate::message_bus::{KlineEvent, KlineHist};
use crate::types::{Kline, Pair, Timeframe};
use rsi::{Rsi, RsiInput};
use volatility::{Volatility, VolatilityInput};

use indicator::Indicator;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndicatorName {
    Rsi,
    Volatility,
}

#[derive(Debug, Clone)]
pub enum IndicatorResult {
    Rsi(<Rsi as Indicator>::Output),
    Volatility(<Volatility as Indicator>::Output),
}

impl IndicatorResult {
    pub fn into_rsi_value(self) -> Option<f32> {
        match self {
            IndicatorResult::Rsi(v) => v,
            _ => None,
        }
    }

    pub fn into_volatility_value(self) -> Option<f32> {
        match self {
            IndicatorResult::Volatility(v) => v,
            _ => None,
        }
    }
}

pub struct IndicatorManager {
    rsi: HashMap<(Pair, Timeframe, IndicatorName), Rsi>,
    vol: HashMap<(Pair, Timeframe, IndicatorName), Volatility>,
}

impl IndicatorManager {
    pub fn new() -> Self {
        Self {
            rsi: HashMap::new(),
            vol: HashMap::new(),
        }
    }

    fn key(
        pair: &Pair,
        timeframe: &Timeframe,
        name: IndicatorName,
    ) -> (Pair, Timeframe, IndicatorName) {
        (pair.clone(), *timeframe, name)
    }

    pub fn update_khist(&mut self, khist: KlineHist) {
        let pair = khist.pair.clone();
        let timeframe = khist.indicator_tf;
        let indicator = khist.indicator;
        let key = Self::key(&pair, &timeframe, indicator);
        match khist.indicator {
            IndicatorName::Rsi => {
                let mut entry = self.rsi.get_mut(&key).unwrap();
                entry.update_khist(khist);
            }
            IndicatorName::Volatility => {
                let mut entry = self.vol.get_mut(&key).unwrap();
                entry.update_khist(khist);
            }
        }
    }

    pub fn update(
        &mut self,
        pair: &Pair,
        timeframe: &Timeframe,
        indicator: IndicatorName,
        bar_1m: &Kline,
    ) -> IndicatorResult {
        match indicator {
            IndicatorName::Rsi => {
                let key = Self::key(pair, timeframe, indicator);
                let entry = self
                    .rsi
                    .entry(key)
                    .or_insert_with(|| Rsi::new(14, timeframe, pair));
                IndicatorResult::Rsi(entry.update(RsiInput { bar_1m: *bar_1m }))
            }
            IndicatorName::Volatility => {
                let key = Self::key(pair, timeframe, indicator);
                let entry = self
                    .vol
                    .entry(key)
                    .or_insert_with(|| Volatility::new(timeframe, pair));
                IndicatorResult::Volatility(entry.update(VolatilityInput { bar_1m: *bar_1m }))
            }
        }
    }
}
