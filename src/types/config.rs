use std::collections::HashMap;

use crate::{
    error::{ConfigError, Result},
    types::{KlineSource, Timeframe},
    ui::settings::SettingsForm,
};

pub const DEFAULT_RSI_LENGTH: usize = 14;

const TIMEFRAME_COUNT: usize = 7;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndicatorKey {
    Volatility = 0,
    Rsi = 1,
}

impl IndicatorKey {
    pub const COUNT: usize = 2;

    pub const fn as_index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone)]
pub struct IndexLookup {
    pair_to_id: HashMap<String, u16>,
    // slot_offsets[indicator][timeframe] -> slot within a pair
    slot_offsets: [[Option<usize>; TIMEFRAME_COUNT]; IndicatorKey::COUNT],
    pair_stride: usize,
}

impl IndexLookup {
    pub fn new(
        pairs: &[String],
        volatility_enabled: bool,
        volatility_timeframes: &[Timeframe],
        rsi_enabled: bool,
        rsi_timeframes: &[Timeframe],
    ) -> Self {
        let mut pair_to_id = HashMap::new();
        for (idx, pair) in pairs.iter().enumerate() {
            // Reuse the first id if the same pair appears multiple times.
            pair_to_id.entry(pair.clone()).or_insert(idx as u16);
        }

        let mut slot_offsets = [[None; TIMEFRAME_COUNT]; IndicatorKey::COUNT];
        let mut next_slot = 0usize;

        if volatility_enabled {
            for &tf in volatility_timeframes {
                let tf_idx = timeframe_index(tf);
                if slot_offsets[IndicatorKey::Volatility.as_index()][tf_idx].is_none() {
                    slot_offsets[IndicatorKey::Volatility.as_index()][tf_idx] = Some(next_slot);
                    next_slot += 1;
                }
            }
        }

        if rsi_enabled {
            for &tf in rsi_timeframes {
                let tf_idx = timeframe_index(tf);
                if slot_offsets[IndicatorKey::Rsi.as_index()][tf_idx].is_none() {
                    slot_offsets[IndicatorKey::Rsi.as_index()][tf_idx] = Some(next_slot);
                    next_slot += 1;
                }
            }
        }

        Self {
            pair_to_id,
            slot_offsets,
            pair_stride: next_slot,
        }
    }

    #[inline]
    pub fn index(
        &self,
        pair: &str,
        indicator: IndicatorKey,
        timeframe: Timeframe,
    ) -> Option<usize> {
        let pair_id = *self.pair_to_id.get(pair)? as usize;
        let tf_idx = timeframe_index(timeframe);
        let slot = self.slot_offsets[indicator.as_index()][tf_idx]?;
        Some(pair_id * self.pair_stride + slot)
    }

    pub fn pair_stride(&self) -> usize {
        self.pair_stride
    }

    pub fn pair_count(&self) -> usize {
        self.pair_to_id.len()
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pairs: Vec<String>,
    indicators: IndicatorConfig,
    index_lookup: IndexLookup,
}

impl AppConfig {
    pub fn from_settings(settings: &SettingsForm) -> Self {
        let volatility_timeframes: Vec<Timeframe> = settings
            .volatility_timeframes
            .iter()
            .filter_map(|(tf, cfg)| cfg.enabled.then_some(*tf))
            .collect();

        let rsi_timeframes: Vec<Timeframe> = settings
            .rsi_timeframes
            .iter()
            .filter_map(|(tf, enabled)| (*enabled).then_some(*tf))
            .collect();

        let pairs = settings.pairs();
        let index_lookup = IndexLookup::new(
            &pairs,
            settings.volatility_enabled,
            &volatility_timeframes,
            settings.rsi_enabled,
            &rsi_timeframes,
        );

        Self {
            pairs,
            indicators: IndicatorConfig {
                volatility: VolatilityConfig {
                    enabled: settings.volatility_enabled,
                    timeframes: volatility_timeframes,
                },
                rsi: RsiConfig {
                    enabled: settings.rsi_enabled,
                    length: settings.rsi_length,
                    source: settings.rsi_source,
                    timeframes: rsi_timeframes,
                },
            },
            index_lookup,
        }
    }

    pub fn pairs(&self) -> &[String] {
        &self.pairs
    }

    pub fn indicators(&self) -> &IndicatorConfig {
        &self.indicators
    }

    pub fn index_lookup(&self) -> &IndexLookup {
        &self.index_lookup
    }
}

#[derive(Debug, Clone)]
pub struct IndicatorConfig {
    volatility: VolatilityConfig,
    rsi: RsiConfig,
}

impl IndicatorConfig {
    pub fn volatility(&self) -> &VolatilityConfig {
        &self.volatility
    }

    pub fn rsi(&self) -> &RsiConfig {
        &self.rsi
    }
}

#[derive(Debug, Clone)]
pub struct VolatilityConfig {
    enabled: bool,
    timeframes: Vec<Timeframe>,
}

impl VolatilityConfig {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn timeframes(&self) -> &[Timeframe] {
        &self.timeframes
    }
}

const fn timeframe_index(tf: Timeframe) -> usize {
    match tf {
        Timeframe::M1 => 0,
        Timeframe::M5 => 1,
        Timeframe::M15 => 2,
        Timeframe::M30 => 3,
        Timeframe::H1 => 4,
        Timeframe::H4 => 5,
        Timeframe::D1 => 6,
    }
}

#[derive(Debug, Clone)]
pub struct RsiConfig {
    enabled: bool,
    length: usize,
    source: KlineSource,
    // RSI shares the windowed 1m buffer; indicator is calculated per timeframe listed here.
    timeframes: Vec<Timeframe>,
}

impl RsiConfig {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn source(&self) -> KlineSource {
        self.source
    }

    pub fn timeframes(&self) -> &[Timeframe] {
        &self.timeframes
    }
}
