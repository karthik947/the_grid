use std::collections::BTreeMap;

use ratatui::{layout::Rect, widgets::TableState};

use crate::{
    message_bus::{
        EngineTx, HistoryTx, IndicatorThresholds, IndicatorValue, UiMessage, UiRx, WsTx,
    },
    tui::{
        data::{
            DashboardData, IndicatorConfig, IndicatorKind, IndicatorState, PairRow,
            default_indicator_state,
        },
        settings::{ALL_TIMEFRAMES, DEFAULT_PRESET_LABEL, PresetStore, SettingsForm},
    },
    types::{AppConfig, Timeframe, config},
};

use super::{
    types::{LayoutField, SettingsField, ViewMode},
    util::{
        active_indicators, dashboard_from_settings, indicator_thresholds_from_settings,
        uppercase_and_limit_pairs,
    },
};

pub struct DashboardApp {
    engine_tx: EngineTx,
    history_tx: HistoryTx,
    ws_tx: WsTx,
    rx: UiRx,
    rt_handle: tokio::runtime::Handle,
    data: DashboardData,
    active_config: Option<AppConfig>,
    default_lookup: config::IndexLookup,
    indicator_values: Vec<f32>,
    indicator_labels: Vec<String>,
    indicator_thresholds: BTreeMap<(IndicatorKind, Timeframe), IndicatorThresholds>,
    indicator_state: IndicatorState,
    settings: SettingsForm,
    settings_draft: SettingsForm,
    preset_store: PresetStore,
    active_preset_label: Option<String>,
    selected_preset_label: String,
    clone_modal_open: bool,
    clone_name: String,
    saved_focus_idx: Option<usize>,
    should_quit: bool,
    view: ViewMode,
    focus_idx: usize,
    table_state: TableState,
}

impl DashboardApp {
    pub fn new(
        engine_tx: EngineTx,
        history_tx: HistoryTx,
        ws_tx: WsTx,
        rt_handle: tokio::runtime::Handle,
        rx: UiRx,
    ) -> Self {
        let selected_preset_label = DEFAULT_PRESET_LABEL.to_string();
        let mut preset_store = PresetStore::load();
        let settings = preset_store
            .get(&selected_preset_label)
            .map(|preset| preset.settings.clone())
            .unwrap_or_default();
        let data = dashboard_from_settings(&settings);
        let indicator_state = default_indicator_state(&data.indicator_config);
        let indicator_thresholds = indicator_thresholds_from_settings(&settings);
        let default_lookup = config::IndexLookup::new(&[], false, &[], false, &[]);

        Self {
            engine_tx,
            history_tx,
            ws_tx,
            rx,
            rt_handle,
            data,
            active_config: None,
            default_lookup,
            indicator_values: Vec::new(),
            indicator_labels: Vec::new(),
            indicator_thresholds,
            indicator_state,
            settings: settings.clone(),
            settings_draft: settings,
            preset_store,
            active_preset_label: None,
            selected_preset_label,
            clone_modal_open: false,
            clone_name: String::new(),
            saved_focus_idx: None,
            should_quit: false,
            view: ViewMode::Dashboard,
            focus_idx: 0,
            table_state: TableState::default(),
        }
    }

    pub fn poll_updates(&mut self) {
        while let Ok(UiMessage::IndicatorResults(batch)) = self.rx.try_recv() {
            for (idx, val) in batch {
                if idx >= self.indicator_values.len() {
                    self.indicator_values.resize(idx + 1, 0.0);
                    self.indicator_labels
                        .resize(idx + 1, IndicatorValue::Volatility(0.0).display());
                }
                let display = val.display();
                let numeric = match val {
                    IndicatorValue::Volatility(v) | IndicatorValue::Rsi(v) => v,
                };
                self.indicator_values[idx] = numeric;
                self.indicator_labels[idx] = display;
            }
        }
    }

    pub fn on_tick(&mut self) {}

    pub fn mark_quit(&mut self) {
        self.should_quit = true;
    }


    pub fn set_view(&mut self, view: ViewMode) {
        self.view = view;
    }

    pub fn settings_focus_order(&self) -> Vec<SettingsField> {
        if self.clone_modal_open {
            return vec![
                SettingsField::CloneName,
                SettingsField::CloneConfirm,
                SettingsField::CloneCancel,
            ];
        }

        let mut fields = vec![
            SettingsField::PresetChips,
            SettingsField::ActivatePreset,
            SettingsField::ClonePreset,
        ];
        fields.push(SettingsField::PairsInput);
        fields.push(SettingsField::VolatilityEnabled);
        for tf in ALL_TIMEFRAMES {
            fields.push(SettingsField::VolatilityTf(tf));
        }
        fields.push(SettingsField::RsiEnabled);
        fields.push(SettingsField::RsiLength);
        fields.push(SettingsField::RsiSource);
        for tf in ALL_TIMEFRAMES {
            fields.push(SettingsField::RsiTf(tf));
        }
        fields.push(SettingsField::Save);
        fields.push(SettingsField::Cancel);
        fields
    }

    pub fn settings_focus_field(&self) -> SettingsField {
        let fields = self.settings_focus_order();
        fields
            .get(self.focus_idx % fields.len())
            .copied()
            .unwrap_or(SettingsField::PresetChips)
    }

    pub fn layout_focus_order(&self) -> Vec<LayoutField> {
        vec![
            LayoutField::ColumnSpacing,
            LayoutField::TableCount,
            LayoutField::TableSpacing,
        ]
    }

    pub fn layout_focus_field(&self) -> LayoutField {
        let fields = self.layout_focus_order();
        fields
            .get(self.focus_idx % fields.len())
            .copied()
            .unwrap_or(LayoutField::ColumnSpacing)
    }

    pub fn focus_idx(&self) -> usize {
        self.focus_idx
    }

    pub fn set_focus_idx(&mut self, idx: usize) {
        self.focus_idx = idx;
    }

    pub fn reset_focus(&mut self) {
        self.focus_idx = 0;
    }

    pub fn ensure_selection_visible(&mut self, visible_rows: usize) {
        if visible_rows == 0 {
            return;
        }
        if let Some(selected) = self.table_state.selected() {
            let offset = self.table_state.offset();
            if selected < offset {
                *self.table_state.offset_mut() = selected;
            } else if selected >= offset + visible_rows {
                *self.table_state.offset_mut() = selected + 1 - visible_rows;
            }
        }
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn view(&self) -> ViewMode {
        self.view
    }

    pub fn focus_field(&self) -> SettingsField {
        self.settings_focus_field()
    }

    pub fn data(&self) -> &DashboardData {
        &self.data
    }

    pub fn indicator_state(&self) -> &IndicatorState {
        &self.indicator_state
    }

    pub fn indicator_thresholds(
        &self,
    ) -> &BTreeMap<(IndicatorKind, Timeframe), IndicatorThresholds> {
        &self.indicator_thresholds
    }

    pub fn set_indicator_thresholds(
        &mut self,
        thresholds: BTreeMap<(IndicatorKind, Timeframe), IndicatorThresholds>,
    ) {
        self.indicator_thresholds = thresholds;
    }

    pub fn indicator_values(&self) -> &[f32] {
        &self.indicator_values
    }

    pub fn set_indicator_buffers(&mut self, values: Vec<f32>, labels: Vec<String>) {
        self.indicator_values = values;
        self.indicator_labels = labels;
    }

    pub fn indicator_labels(&self) -> &[String] {
        &self.indicator_labels
    }

    pub fn table_state(&mut self) -> &mut TableState {
        &mut self.table_state
    }

    pub fn active_index_lookup(&self) -> &config::IndexLookup {
        self.active_config
            .as_ref()
            .map(|c| c.index_lookup())
            .unwrap_or(&self.default_lookup)
    }

    pub fn engine_tx(&self) -> &EngineTx {
        &self.engine_tx
    }

    pub fn history_tx(&self) -> &HistoryTx {
        &self.history_tx
    }

    pub fn ws_tx(&self) -> &WsTx {
        &self.ws_tx
    }

    pub fn runtime_handle(&self) -> tokio::runtime::Handle {
        self.rt_handle.clone()
    }

    pub fn active_indicators(&self) -> Vec<&IndicatorConfig> {
        active_indicators(&self.data.indicator_config, &self.indicator_state)
    }

    pub fn preset_labels(&self) -> Vec<String> {
        self.preset_store.labels()
    }

    pub fn selected_preset_label(&self) -> &str {
        &self.selected_preset_label
    }

    pub fn set_selected_preset(&mut self, label: String) {
        self.selected_preset_label = label;
    }

    pub fn active_preset_label(&self) -> Option<&String> {
        self.active_preset_label.as_ref()
    }

    pub fn set_active_preset(&mut self, label: Option<String>) {
        self.active_preset_label = label;
    }

    pub fn preset_store(&self) -> &PresetStore {
        &self.preset_store
    }

    pub fn preset_store_mut(&mut self) -> &mut PresetStore {
        &mut self.preset_store
    }

    pub fn settings_draft(&self) -> &SettingsForm {
        &self.settings_draft
    }

    pub fn settings_draft_mut(&mut self) -> &mut SettingsForm {
        &mut self.settings_draft
    }

    pub fn set_settings_draft(&mut self, pairs_input: String, mut draft: SettingsForm) {
        draft.pairs_input = pairs_input;
        self.settings_draft = draft;
    }

    pub fn settings(&self) -> &SettingsForm {
        &self.settings
    }

    pub fn set_settings(&mut self, settings: SettingsForm) {
        self.settings = settings;
    }

    pub fn clone_modal_open(&self) -> bool {
        self.clone_modal_open
    }

    pub fn set_clone_modal_open(&mut self, open: bool) {
        self.clone_modal_open = open;
    }

    pub fn clone_name(&self) -> &str {
        &self.clone_name
    }

    pub fn clone_name_mut(&mut self) -> &mut String {
        &mut self.clone_name
    }

    pub fn saved_focus_idx(&self) -> Option<usize> {
        self.saved_focus_idx
    }

    pub fn set_saved_focus_idx(&mut self, idx: Option<usize>) {
        self.saved_focus_idx = idx;
    }

    pub fn pairs(&self) -> &[PairRow] {
        &self.data.pairs
    }

    pub fn set_active_config(&mut self, cfg: Option<AppConfig>) {
        self.active_config = cfg;
    }

    pub fn set_data(&mut self, data: DashboardData) {
        self.data = data;
    }

    pub fn set_indicator_state(&mut self, state: IndicatorState) {
        self.indicator_state = state;
    }

    pub fn pair_width(&self, area: &Rect) -> u16 {
        let max_pair = self
            .data
            .pairs
            .iter()
            .map(|p| p.pair.len())
            .max()
            .unwrap_or(4);
        let padding = 2;
        let desired = (max_pair + padding) as u16;
        desired.min(area.width.saturating_sub(10)).max(8)
    }

    pub fn value_width(&self) -> u16 {
        6
    }
}
