use std::collections::BTreeMap;

use confy;
use log::error;
use serde::{Deserialize, Serialize};

use crate::{
    types::{KlineSource, Timeframe, config::DEFAULT_RSI_LENGTH},
    ui::data::{IndicatorConfig, IndicatorKind},
};

pub const DEFAULT_PRESET_LABEL: &str = "Default";
pub const PRESET_CONFIG_NAME: &str = "dashboard_presets";
pub const MAX_PAIRS: usize = 200;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VolatilityTimeframeSetting {
    pub enabled: bool,
    pub threshold: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettingsForm {
    pub pairs_input: String,
    pub volatility_enabled: bool,
    pub volatility_timeframes: BTreeMap<Timeframe, VolatilityTimeframeSetting>,
    pub rsi_enabled: bool,
    pub rsi_length: usize,
    pub rsi_source: KlineSource,
    pub rsi_timeframes: BTreeMap<Timeframe, bool>,
}

impl Default for SettingsForm {
    fn default() -> Self {
        let volatility_timeframes = default_volatility_timeframes();
        let rsi_timeframes = default_timeframe_toggles(&[Timeframe::M5, Timeframe::M15]);
        Self {
            pairs_input: "BTCUSDT,ETHUSDT".to_string(),
            volatility_enabled: true,
            volatility_timeframes,
            rsi_enabled: true,
            rsi_length: DEFAULT_RSI_LENGTH,
            rsi_source: KlineSource::Close,
            rsi_timeframes,
        }
    }
}

impl SettingsForm {
    pub fn indicator_config(&self) -> Vec<IndicatorConfig> {
        let volatility_timeframes: Vec<Timeframe> = self
            .volatility_timeframes
            .iter()
            .filter_map(|(tf, setting)| setting.enabled.then_some(*tf))
            .collect();
        let volatility_thresholds: BTreeMap<Timeframe, f32> = self
            .volatility_timeframes
            .iter()
            .filter_map(|(tf, setting)| setting.enabled.then_some((*tf, setting.threshold.abs())))
            .collect();

        let rsi_timeframes: Vec<Timeframe> = self
            .rsi_timeframes
            .iter()
            .filter_map(|(tf, enabled)| (*enabled).then_some(*tf))
            .collect();

        vec![
            IndicatorConfig {
                kind: IndicatorKind::Volatility,
                enabled: self.volatility_enabled,
                timeframes: volatility_timeframes,
                thresholds: volatility_thresholds,
            },
            IndicatorConfig {
                kind: IndicatorKind::Rsi,
                enabled: self.rsi_enabled,
                timeframes: rsi_timeframes,
                thresholds: BTreeMap::new(),
            },
        ]
    }

    pub fn pairs(&self) -> Vec<String> {
        self.pairs_input
            .to_ascii_uppercase()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .take(MAX_PAIRS)
            .map(|s| s.to_string())
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredPreset {
    pub settings: SettingsForm,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresetStore {
    presets: BTreeMap<String, StoredPreset>,
}

impl Default for PresetStore {
    fn default() -> Self {
        let mut presets = BTreeMap::new();
        presets.insert(
            DEFAULT_PRESET_LABEL.to_string(),
            StoredPreset {
                settings: SettingsForm::default(),
            },
        );
        Self { presets }
    }
}

impl PresetStore {
    pub fn load() -> Self {
        match confy::load::<PresetStore>("testmods", PRESET_CONFIG_NAME) {
            Ok(store) => store.normalize(),
            Err(err) => {
                error!("failed to load presets: {err}");
                PresetStore::default()
            }
        }
    }

    pub fn save(&self) {
        if let Err(err) = confy::store("testmods", PRESET_CONFIG_NAME, self) {
            error!("failed to save presets: {err}");
        }
    }

    fn normalize(mut self) -> Self {
        if self.presets.is_empty() {
            return PresetStore::default();
        }

        self.presets
            .entry(DEFAULT_PRESET_LABEL.to_string())
            .or_insert(StoredPreset {
                settings: SettingsForm::default(),
            });

        self
    }

    pub fn upsert(&mut self, label: String, settings: SettingsForm) {
        let entry = self.presets.entry(label.clone()).or_insert(StoredPreset {
            settings: settings.clone(),
        });
        entry.settings = settings;
    }

    pub fn labels(&self) -> Vec<String> {
        self.presets.keys().cloned().collect()
    }

    pub fn get(&self, label: &str) -> Option<&StoredPreset> {
        self.presets.get(label)
    }
}

pub fn default_volatility_timeframes() -> BTreeMap<Timeframe, VolatilityTimeframeSetting> {
    ALL_TIMEFRAMES
        .iter()
        .map(|&tf| {
            let enabled = matches!(
                tf,
                Timeframe::M15 | Timeframe::H1 | Timeframe::H4 | Timeframe::D1
            );
            (
                tf,
                VolatilityTimeframeSetting {
                    enabled,
                    threshold: 0.0,
                },
            )
        })
        .collect()
}

pub fn default_timeframe_toggles(enabled: &[Timeframe]) -> BTreeMap<Timeframe, bool> {
    ALL_TIMEFRAMES
        .iter()
        .map(|&tf| (tf, enabled.contains(&tf)))
        .collect()
}

pub const ALL_TIMEFRAMES: [Timeframe; 7] = [
    Timeframe::M1,
    Timeframe::M5,
    Timeframe::M15,
    Timeframe::M30,
    Timeframe::H1,
    Timeframe::H4,
    Timeframe::D1,
];
