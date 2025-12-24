use std::collections::BTreeMap;

use crate::types::Timeframe;

/// Indicators supported on the dashboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IndicatorKind {
    Volatility,
    Rsi,
}

impl IndicatorKind {
    pub const fn label(&self) -> &'static str {
        match self {
            IndicatorKind::Volatility => "VOLATILITY",
            IndicatorKind::Rsi => "RSI",
        }
    }
}

/// Size presets that drive spacing and typography.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SizePreset {
    Xs,
    Sm,
    Md,
    Lg,
    Xl,
}

impl SizePreset {
    pub const fn label(&self) -> &'static str {
        match self {
            SizePreset::Xs => "XS",
            SizePreset::Sm => "SM",
            SizePreset::Md => "MD",
            SizePreset::Lg => "LG",
            SizePreset::Xl => "XL",
        }
    }
}

impl Default for SizePreset {
    fn default() -> Self {
        SizePreset::Xs
    }
}

/// UI metrics derived from the active size preset.
#[derive(Clone, Copy, Debug)]
pub struct UiMetrics {
    pub font_size: f32,
    pub header_timeframe_font_size: f32,
    pub header_indicator_font_size: f32,
    pub row_height: f32,
    pub cell_padding: f32,
    pub group_gap: f32,
    pub column_gap: f32,
    pub header_height: f32,
}

impl From<SizePreset> for UiMetrics {
    fn from(value: SizePreset) -> Self {
        match value {
            SizePreset::Xs => Self {
                font_size: 12.0,
                header_timeframe_font_size: 14.0,
                header_indicator_font_size: 8.0,
                row_height: 22.0,
                cell_padding: 6.0,
                group_gap: 12.0,
                column_gap: 16.0,
                header_height: 36.0,
            },
            SizePreset::Sm => Self {
                font_size: 13.0,
                header_timeframe_font_size: 15.0,
                header_indicator_font_size: 10.5,
                row_height: 24.0,
                cell_padding: 7.0,
                group_gap: 14.0,
                column_gap: 18.0,
                header_height: 38.0,
            },
            SizePreset::Md => Self {
                font_size: 14.0,
                header_timeframe_font_size: 16.0,
                header_indicator_font_size: 11.0,
                row_height: 26.0,
                cell_padding: 8.0,
                group_gap: 16.0,
                column_gap: 20.0,
                header_height: 40.0,
            },
            SizePreset::Lg => Self {
                font_size: 15.0,
                header_timeframe_font_size: 17.0,
                header_indicator_font_size: 11.5,
                row_height: 28.0,
                cell_padding: 9.0,
                group_gap: 18.0,
                column_gap: 22.0,
                header_height: 42.0,
            },
            SizePreset::Xl => Self {
                font_size: 16.0,
                header_timeframe_font_size: 18.0,
                header_indicator_font_size: 12.0,
                row_height: 30.0,
                cell_padding: 10.0,
                group_gap: 20.0,
                column_gap: 24.0,
                header_height: 44.0,
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct IndicatorConfig {
    pub kind: IndicatorKind,
    pub enabled: bool,
    pub timeframes: Vec<Timeframe>,
    pub thresholds: BTreeMap<Timeframe, f32>,
}

#[derive(Clone, Debug)]
pub struct PairRow {
    pub pair: String,
}

#[derive(Clone, Debug)]
pub struct DashboardData {
    pub pairs: Vec<PairRow>,
    pub indicator_config: Vec<IndicatorConfig>,
}

pub type IndicatorState = BTreeMap<IndicatorKind, bool>;

pub fn default_indicator_state(config: &[IndicatorConfig]) -> IndicatorState {
    config.iter().map(|cfg| (cfg.kind, cfg.enabled)).collect()
}

fn default_indicator_config() -> Vec<IndicatorConfig> {
    vec![
        IndicatorConfig {
            kind: IndicatorKind::Volatility,
            enabled: true,
            timeframes: vec![Timeframe::M15, Timeframe::H1, Timeframe::H4, Timeframe::D1],
            thresholds: thresholds_for(&[
                Timeframe::M15,
                Timeframe::H1,
                Timeframe::H4,
                Timeframe::D1,
            ]),
        },
        IndicatorConfig {
            kind: IndicatorKind::Rsi,
            enabled: true,
            timeframes: vec![Timeframe::M15, Timeframe::H1, Timeframe::H4, Timeframe::D1],
            thresholds: BTreeMap::new(),
        },
    ]
}

/// Generates deterministic dummy data for the dashboard.
pub struct DashboardDataBuilder {
    pair_count: usize,
    pairs: Option<Vec<String>>,
    indicator_config: Vec<IndicatorConfig>,
    seed: u64,
}

impl DashboardDataBuilder {
    pub fn new() -> Self {
        Self {
            pair_count: 200,
            indicator_config: default_indicator_config(),
            pairs: None,
            seed: 0x5EED_DBAC,
        }
    }

    pub fn pair_count(mut self, pair_count: usize) -> Self {
        self.pair_count = pair_count;
        self
    }

    pub fn indicator(
        mut self,
        kind: IndicatorKind,
        enabled: bool,
        timeframes: Vec<Timeframe>,
    ) -> Self {
        let thresholds = thresholds_for(&timeframes);
        if let Some(cfg) = self
            .indicator_config
            .iter_mut()
            .find(|cfg| cfg.kind == kind)
        {
            cfg.enabled = enabled;
            cfg.timeframes = timeframes;
            cfg.thresholds = thresholds;
        } else {
            self.indicator_config.push(IndicatorConfig {
                kind,
                enabled,
                timeframes,
                thresholds,
            });
        }
        self
    }

    pub fn indicator_config(mut self, indicator_config: Vec<IndicatorConfig>) -> Self {
        self.indicator_config = indicator_config;
        self
    }

    pub fn pairs(mut self, pairs: Vec<String>) -> Self {
        self.pairs = Some(pairs);
        self
    }

    pub fn build(mut self) -> DashboardData {
        let mut rng = Lcg::new(self.seed);
        let indicator_config = self.indicator_config.clone();
        let pair_names = if let Some(pairs) = self.pairs.take() {
            pairs
        } else {
            (0..self.pair_count)
                .map(|_| generate_pair(&mut rng))
                .collect::<Vec<_>>()
        };

        let mut pairs = Vec::with_capacity(pair_names.len());

        for pair in pair_names {
            pairs.push(PairRow { pair });
        }

        DashboardData {
            pairs,
            indicator_config,
        }
    }
}

fn generate_pair(rng: &mut Lcg) -> String {
    let mut letters = String::with_capacity(6);
    for _ in 0..6 {
        letters.push(rng.next_char());
    }
    letters.push_str("USDT");
    letters
}

fn thresholds_for(timeframes: &[Timeframe]) -> BTreeMap<Timeframe, f32> {
    timeframes.iter().copied().map(|tf| (tf, 0.0_f32)).collect()
}

#[derive(Clone)]
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        // Simple LCG; good enough for deterministic dummy data.
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }

    fn next_char(&mut self) -> char {
        let idx = (self.next() % 26) as u8;
        (b'A' + idx) as char
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        let raw = (self.next() >> 32) as f32 / u32::MAX as f32;
        min + (max - min) * raw
    }
}
