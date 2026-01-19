use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    message_bus::{EngineMessage, HistoryMessage, IndicatorValue, WsMessage},
    tui::{
        layout,
        settings::{ALL_TIMEFRAMES, SettingsForm, VolatilityTimeframeSetting},
    },
    types::{AppConfig, KlineSource, Timeframe},
};

use super::{
    app::DashboardApp,
    types::{LayoutField, SettingsField, ViewMode},
    util::{
        dashboard_from_settings, indicator_thresholds_from_settings, uppercase_and_limit_pairs,
    },
};

impl DashboardApp {
    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.view() {
            ViewMode::Dashboard => self.handle_dashboard_key(key),
            ViewMode::Settings => self.handle_settings_key(key),
            ViewMode::Layout => self.handle_layout_key(key),
        }
    }

    fn handle_dashboard_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.mark_quit(),
            KeyCode::Char('s') => self.open_settings(),
            KeyCode::Char('l') => self.open_layout(),
            KeyCode::Up => self.move_selection_up(),
            KeyCode::Down => self.move_selection_down(),
            _ => {}
        }
    }

    fn handle_settings_key(&mut self, key: KeyEvent) {
        if self.clone_modal_open() {
            self.handle_clone_modal_key(key);
            return;
        }

        match key.code {
            KeyCode::Esc => self.set_view(ViewMode::Dashboard),
            KeyCode::Tab => self.focus_next(),
            KeyCode::BackTab => self.focus_prev(),
            _ => {}
        }

        match self.focus_field() {
            SettingsField::PresetChips => match key.code {
                KeyCode::Left => self.cycle_preset(-1),
                KeyCode::Right => self.cycle_preset(1),
                KeyCode::Enter => self.activate_selected(),
                _ => {}
            },
            SettingsField::ActivatePreset => {
                if matches!(key.code, KeyCode::Enter) {
                    self.activate_selected();
                }
            }
            SettingsField::ClonePreset => {
                if matches!(key.code, KeyCode::Enter) {
                    self.open_clone_modal();
                }
            }
            SettingsField::PairsInput => match key.code {
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.settings_draft_mut().pairs_input.push(c);
                    let limited = uppercase_and_limit_pairs(&self.settings_draft().pairs_input);
                    self.settings_draft_mut().pairs_input = limited;
                }
                KeyCode::Backspace => {
                    self.settings_draft_mut().pairs_input.pop();
                }
                _ => {}
            },
            SettingsField::VolatilityEnabled => {
                if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                    self.settings_draft_mut().volatility_enabled =
                        !self.settings_draft().volatility_enabled
                }
            }
            SettingsField::VolatilityTf(tf) => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    let entry = self
                        .settings_draft_mut()
                        .volatility_timeframes
                        .entry(tf)
                        .or_insert(VolatilityTimeframeSetting {
                            enabled: false,
                            threshold: 0.0,
                        });
                    entry.enabled = !entry.enabled;
                }
                KeyCode::Left => self.adjust_threshold(tf, -0.5),
                KeyCode::Right => self.adjust_threshold(tf, 0.5),
                KeyCode::Up => self.adjust_threshold(tf, 5.0),
                KeyCode::Down => self.adjust_threshold(tf, -5.0),
                _ => {}
            },
            SettingsField::RsiEnabled => {
                if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                    self.settings_draft_mut().rsi_enabled = !self.settings_draft().rsi_enabled
                }
            }
            SettingsField::RsiLength => match key.code {
                KeyCode::Left | KeyCode::Down => {
                    if self.settings_draft().rsi_length > 1 {
                        self.settings_draft_mut().rsi_length -= 1;
                    }
                }
                KeyCode::Right | KeyCode::Up => {
                    self.settings_draft_mut().rsi_length =
                        (self.settings_draft().rsi_length + 1).min(250);
                }
                _ => {}
            },
            SettingsField::RsiSource => {
                if matches!(key.code, KeyCode::Left | KeyCode::Right | KeyCode::Enter) {
                    self.cycle_source();
                }
            }
            SettingsField::RsiTf(tf) => {
                if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                    let entry = self
                        .settings_draft_mut()
                        .rsi_timeframes
                        .entry(tf)
                        .or_insert(false);
                    *entry = !*entry;
                }
            }
            SettingsField::Save => {
                if matches!(key.code, KeyCode::Enter) {
                    self.save_settings();
                    self.set_view(ViewMode::Dashboard);
                }
            }
            SettingsField::Cancel => {
                if matches!(key.code, KeyCode::Enter) {
                    self.cancel_settings();
                    self.set_view(ViewMode::Dashboard);
                }
            }
            _ => {}
        }
    }

    fn handle_layout_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.set_view(ViewMode::Dashboard),
            KeyCode::Tab => self.focus_next(),
            KeyCode::BackTab => self.focus_prev(),
            _ => {}
        }

        match self.layout_focus_field() {
            LayoutField::ColumnSpacing => match key.code {
                KeyCode::Left | KeyCode::Down => {
                    layout::adjust_column_spacing(self.settings_draft_mut(), -1);
                    self.sync_layout_settings();
                }
                KeyCode::Right | KeyCode::Up => {
                    layout::adjust_column_spacing(self.settings_draft_mut(), 1);
                    self.sync_layout_settings();
                }
                _ => {}
            },
            LayoutField::TableCount => match key.code {
                KeyCode::Left | KeyCode::Down => {
                    layout::adjust_table_count(self.settings_draft_mut(), -1);
                    self.sync_layout_settings();
                }
                KeyCode::Right | KeyCode::Up => {
                    layout::adjust_table_count(self.settings_draft_mut(), 1);
                    self.sync_layout_settings();
                }
                _ => {}
            },
            LayoutField::TableSpacing => match key.code {
                KeyCode::Left | KeyCode::Down => {
                    layout::adjust_table_spacing(self.settings_draft_mut(), -1);
                    self.sync_layout_settings();
                }
                KeyCode::Right | KeyCode::Up => {
                    layout::adjust_table_spacing(self.settings_draft_mut(), 1);
                    self.sync_layout_settings();
                }
                _ => {}
            },
        }
    }

    fn open_settings(&mut self) {
        if let Some(active) = self.active_preset_label().cloned() {
            self.set_selected_preset(active);
        }
        let draft = self
            .preset_store()
            .get(self.selected_preset_label())
            .map(|p| p.settings.clone())
            .unwrap_or_else(|| self.settings().clone());
        self.set_settings_draft(uppercase_and_limit_pairs(&draft.pairs_input), draft);
        self.clone_name_mut().clear();
        self.set_clone_modal_open(false);
        self.set_saved_focus_idx(None);
        self.set_view(ViewMode::Settings);
        self.reset_focus();
    }

    fn open_layout(&mut self) {
        if let Some(active) = self.active_preset_label().cloned() {
            self.set_selected_preset(active);
        }
        let draft = self
            .preset_store()
            .get(self.selected_preset_label())
            .map(|p| p.settings.clone())
            .unwrap_or_else(|| self.settings().clone());
        self.set_settings_draft(uppercase_and_limit_pairs(&draft.pairs_input), draft);
        self.set_clone_modal_open(false);
        self.set_saved_focus_idx(None);
        self.set_view(ViewMode::Layout);
        self.reset_focus();
    }

    fn sync_layout_settings(&mut self) {
        let updated = self.settings_draft().clone();
        self.set_settings(updated.clone());

        let active_matches = self
            .active_preset_label()
            .is_some_and(|active| active == self.selected_preset_label());
        if active_matches {
            let label = self.selected_preset_label().to_string();
            let store = self.preset_store_mut();
            store.upsert(label, updated);
            store.save();
        }
    }

    fn move_selection_up(&mut self) {
        let total = self.pairs().len();
        if total == 0 {
            return;
        }
        let selected = self.table_state().selected().unwrap_or(0);
        let next = if selected == 0 {
            total - 1
        } else {
            selected - 1
        };
        self.table_state().select(Some(next));
    }

    fn move_selection_down(&mut self) {
        let total = self.pairs().len();
        if total == 0 {
            return;
        }
        let selected = self.table_state().selected().unwrap_or(0);
        let next = if selected + 1 >= total {
            0
        } else {
            selected + 1
        };
        self.table_state().select(Some(next));
    }

    fn focus_next(&mut self) {
        let len = match self.view() {
            ViewMode::Settings => self.settings_focus_order().len(),
            ViewMode::Layout => self.layout_focus_order().len(),
            ViewMode::Dashboard => 1,
        };
        self.set_focus_idx((self.focus_idx() + 1) % len);
    }

    fn focus_prev(&mut self) {
        let len = match self.view() {
            ViewMode::Settings => self.settings_focus_order().len(),
            ViewMode::Layout => self.layout_focus_order().len(),
            ViewMode::Dashboard => 1,
        };
        self.set_focus_idx((self.focus_idx() + len - 1) % len);
    }

    fn handle_clone_modal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.close_clone_modal(false);
                return;
            }
            KeyCode::Tab => self.focus_next(),
            KeyCode::BackTab => self.focus_prev(),
            _ => {}
        }

        match self.focus_field() {
            SettingsField::CloneName => match key.code {
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.clone_name_mut().push(c);
                }
                KeyCode::Backspace => {
                    self.clone_name_mut().pop();
                }
                _ => {}
            },
            SettingsField::CloneConfirm => {
                if matches!(key.code, KeyCode::Enter) {
                    self.confirm_clone_preset();
                }
            }
            SettingsField::CloneCancel => {
                if matches!(key.code, KeyCode::Enter) {
                    self.close_clone_modal(false);
                }
            }
            _ => {}
        }
    }

    fn open_clone_modal(&mut self) {
        self.set_saved_focus_idx(Some(self.focus_idx()));
        self.set_focus_idx(0);
        self.clone_name_mut().clear();
        self.set_clone_modal_open(true);
    }

    fn close_clone_modal(&mut self, reset_name: bool) {
        self.set_clone_modal_open(false);
        if reset_name {
            self.clone_name_mut().clear();
        }
        if let Some(idx) = self.saved_focus_idx() {
            self.set_focus_idx(idx);
        }
        self.set_saved_focus_idx(None);
    }

    fn cycle_preset(&mut self, delta: isize) {
        let mut labels = self.preset_labels();
        labels.sort();
        if labels.is_empty() {
            return;
        }
        let current_idx = labels
            .iter()
            .position(|l| l == self.selected_preset_label())
            .unwrap_or(0);
        let next_idx = ((current_idx as isize + delta).rem_euclid(labels.len() as isize)) as usize;
        let next_label = labels[next_idx].clone();
        self.select_preset(next_label);
    }

    fn adjust_threshold(&mut self, tf: Timeframe, delta: f32) {
        let entry = self
            .settings_draft_mut()
            .volatility_timeframes
            .entry(tf)
            .or_insert(VolatilityTimeframeSetting {
                enabled: false,
                threshold: 0.0,
            });
        entry.threshold = (entry.threshold + delta).abs();
    }

    fn cycle_source(&mut self) {
        let next = match self.settings_draft().rsi_source {
            KlineSource::Open => KlineSource::High,
            KlineSource::High => KlineSource::Low,
            KlineSource::Low => KlineSource::Close,
            KlineSource::Close => KlineSource::Open,
        };
        self.settings_draft_mut().rsi_source = next;
    }

    fn select_preset(&mut self, label: String) {
        if let Some(preset) = self.preset_store().get(&label).cloned() {
            self.set_selected_preset(label.clone());
            let mut draft = preset.settings;
            draft.pairs_input = uppercase_and_limit_pairs(&draft.pairs_input);
            self.set_settings_draft(draft.pairs_input.clone(), draft);
        }
    }

    fn commit_settings(&mut self, update_active: bool) {
        let saved_settings = self.settings_draft().clone();
        let label = self.selected_preset_label().to_string();
        {
            let store = self.preset_store_mut();
            store.upsert(label, saved_settings.clone());
            store.save();
        }
        if update_active {
            self.set_settings(saved_settings.clone());
            self.refresh_from_settings(saved_settings);
        }
    }

    fn save_settings(&mut self) {
        let saved_label = self.selected_preset_label().to_string();
        let update_active = self
            .active_preset_label()
            .is_some_and(|active| *active == saved_label);
        self.commit_settings(update_active);
    }

    fn cancel_settings(&mut self) {
        if let Some(preset) = self.preset_store().get(self.selected_preset_label()) {
            self.set_settings_draft(preset.settings.pairs_input.clone(), preset.settings.clone());
        } else {
            self.set_settings_draft(self.settings().pairs_input.clone(), self.settings().clone());
        }
    }

    fn activate_selected(&mut self) {
        let label = self.selected_preset_label().to_string();
        if let Some(preset) = self.preset_store().get(&label).cloned() {
            self.set_active_preset(Some(label.clone()));
            self.set_settings(preset.settings.clone());
            self.set_settings_draft(preset.settings.pairs_input.clone(), preset.settings.clone());
            self.refresh_from_settings(self.settings().clone());
            self.set_selected_preset(label.clone());
            let config = AppConfig::from_settings(self.settings());
            let cfg_for_history = config.clone();
            let cfg_for_ws = config.clone();
            let tx1 = self.engine_tx().clone();
            let tx2 = self.history_tx().clone();
            let tx3 = self.ws_tx().clone();
            let rt_handle = self.runtime_handle();

            let pair_stride = config.index_lookup().pair_stride();
            let total_slots = config.index_lookup().pair_count() * pair_stride;
            self.set_indicator_buffers(
                vec![0.0; total_slots],
                vec![IndicatorValue::Volatility(0.0).display(); total_slots],
            );
            self.set_active_config(Some(config.clone()));

            rt_handle.spawn(async move {
                let (engine_res, history_res, ws_res) = tokio::join!(
                    tx1.send(EngineMessage::Config(config)),
                    tx2.send(HistoryMessage::Config(cfg_for_history)),
                    tx3.send(WsMessage::Config(cfg_for_ws)),
                );
                if let Err(err) = engine_res {
                    log::error!("failed to send config to engine: {err}");
                }
                if let Err(err) = history_res {
                    log::error!("failed to send config to history: {err}");
                }
                if let Err(err) = ws_res {
                    log::error!("failed to send config to ws: {err}");
                }
            });
        }
    }

    fn confirm_clone_preset(&mut self) {
        let label = self.clone_name().trim().to_string();
        if label.is_empty() {
            return;
        }

        let mut settings = self.settings_draft().clone();
        settings.pairs_input = uppercase_and_limit_pairs(&settings.pairs_input);
        self.set_selected_preset(label.clone());
        self.set_settings_draft(settings.pairs_input.clone(), settings.clone());
        {
            let store = self.preset_store_mut();
            store.upsert(label, settings);
            store.save();
        }
        self.close_clone_modal(true);
    }

    fn refresh_from_settings(&mut self, settings: SettingsForm) {
        self.set_data(dashboard_from_settings(&settings));
        self.set_indicator_state(crate::tui::data::default_indicator_state(
            &self.data().indicator_config,
        ));
        self.set_indicator_thresholds(indicator_thresholds_from_settings(&settings));
        self.set_settings_draft(settings.pairs_input.clone(), settings);
    }
}
