use std::collections::BTreeMap;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
};

use crate::{
    message_bus::{IndicatorThresholds, IndicatorValue},
    tui::{
        data::{
            DashboardData, DashboardDataBuilder, IndicatorConfig, IndicatorKind, IndicatorState,
        },
        settings::{ALL_TIMEFRAMES, MAX_PAIRS, SettingsForm, VolatilityTimeframeSetting},
    },
    types::{KlineSource, Timeframe, config},
};

pub const POSITIVE_TEXT: Color = Color::Rgb(64, 199, 122);
pub const NEGATIVE_TEXT: Color = Color::Rgb(230, 82, 82);
pub const POSITIVE_BG: Color = Color::Rgb(33, 178, 125);
pub const NEGATIVE_BG: Color = Color::Rgb(186, 64, 117);
pub const HEADER_COLOR: Color = Color::Rgb(90, 200, 255);
pub const PAIR_COLOR: Color = Color::Rgb(200, 200, 200);
pub const INDICATOR_GROUP_BG: Color = Color::Rgb(90, 200, 255);
pub const FIELD_ACTIVE: Color = Color::Rgb(100, 220, 150);
pub const FIELD_INACTIVE: Color = Color::Rgb(120, 120, 120);
pub const ACTIVE_ROW_BG: Color = Color::Rgb(20, 55, 40);

pub fn active_indicators<'a>(
    config: &'a [IndicatorConfig],
    state: &IndicatorState,
) -> Vec<&'a IndicatorConfig> {
    config
        .iter()
        .filter(|cfg| *state.get(&cfg.kind).unwrap_or(&cfg.enabled))
        .collect()
}

pub fn lookup_value<'a>(
    pair: &str,
    kind: IndicatorKind,
    timeframe: Timeframe,
    index_lookup: &config::IndexLookup,
    indicator_values: &[f32],
    indicator_labels: &'a [String],
) -> (f32, Option<&'a str>) {
    let key = match kind {
        IndicatorKind::Volatility => config::IndicatorKey::Volatility,
        IndicatorKind::Rsi => config::IndicatorKey::Rsi,
    };

    if let Some(idx) = index_lookup.index(pair, key, timeframe) {
        let value = indicator_values.get(idx).copied().unwrap_or(0.0);
        let display = indicator_labels.get(idx).map(|s| s.as_str());
        (value, display)
    } else {
        (0.0, None)
    }
}

pub fn value_style(value: &IndicatorValue, thresholds: Option<&IndicatorThresholds>) -> Style {
    match value {
        IndicatorValue::Volatility(v) => {
            let mut background = None;
            if let Some(IndicatorThresholds::Volatility { threshold }) = thresholds {
                if *threshold > 0.0 && v.abs() >= *threshold {
                    background = Some(if *v >= 0.0 { POSITIVE_BG } else { NEGATIVE_BG });
                }
            }

            let mut style = Style::default();
            if let Some(bg) = background {
                style = style.bg(bg).fg(Color::Black);
            } else if *v >= 0.0 {
                style = style.fg(POSITIVE_TEXT);
            } else {
                style = style.fg(NEGATIVE_TEXT);
            }
            style
        }
        IndicatorValue::Rsi(v) => {
            let mut background = None;
            if let Some(IndicatorThresholds::Rsi {
                oversold,
                overbought,
            }) = thresholds
            {
                if v <= oversold {
                    background = Some(NEGATIVE_BG);
                } else if v >= overbought {
                    background = Some(POSITIVE_BG);
                }
            }

            let mut style = Style::default();
            if let Some(bg) = background {
                style = style.bg(bg).fg(Color::Black);
            } else if *v >= 50.0 {
                style = style.fg(POSITIVE_TEXT);
            } else {
                style = style.fg(NEGATIVE_TEXT);
            }
            style
        }
    }
}

pub fn toggle_label(enabled: bool) -> &'static str {
    if enabled { "[ON]" } else { "[OFF]" }
}

pub fn pair_count(input: &str) -> usize {
    input
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .count()
}

pub fn uppercase_and_limit_pairs(input: &str) -> String {
    let uppercased = input.to_ascii_uppercase();
    if pair_count(&uppercased) <= MAX_PAIRS {
        return uppercased;
    }
    clamp_pairs_input(&uppercased)
}

pub fn clamp_pairs_input(input: &str) -> String {
    let mut limited = String::new();
    for (idx, pair) in input
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .take(MAX_PAIRS)
        .enumerate()
    {
        if idx > 0 {
            limited.push_str(", ");
        }
        limited.push_str(pair);
    }
    limited
}

pub fn tf_label(tf: Timeframe) -> String {
    tf.to_string().to_owned()
}

pub fn kline_source_label(source: KlineSource) -> &'static str {
    match source {
        KlineSource::Open => "Open",
        KlineSource::High => "High",
        KlineSource::Low => "Low",
        KlineSource::Close => "Close",
    }
}

pub fn indicator_thresholds_from_settings(
    settings: &SettingsForm,
) -> BTreeMap<(IndicatorKind, Timeframe), IndicatorThresholds> {
    let mut thresholds = BTreeMap::new();

    for (&tf, cfg) in &settings.volatility_timeframes {
        if cfg.enabled {
            thresholds.insert(
                (IndicatorKind::Volatility, tf),
                IndicatorThresholds::Volatility {
                    threshold: cfg.threshold.abs(),
                },
            );
        }
    }

    for (&tf, enabled) in &settings.rsi_timeframes {
        if *enabled {
            thresholds.insert(
                (IndicatorKind::Rsi, tf),
                IndicatorThresholds::Rsi {
                    oversold: 30.0,
                    overbought: 70.0,
                },
            );
        }
    }

    thresholds
}

pub fn dashboard_from_settings(settings: &SettingsForm) -> DashboardData {
    DashboardDataBuilder::new()
        .indicator_config(settings.indicator_config())
        .pairs(settings.pairs())
        .build()
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
