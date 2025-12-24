use eframe::App;
use egui::{Color32, Id, Layout, RichText, ScrollArea, TextEdit, TopBottomPanel};
use std::collections::BTreeMap;

use crate::{
    message_bus::{
        EngineMessage, EngineTx, HistoryMessage, HistoryTx, IndicatorThresholds, IndicatorValue,
        UiMessage, UiRx, WsMessage, WsTx,
    },
    types::{AppConfig, KlineSource, Timeframe, config},
    ui::{
        data::{
            DashboardData, DashboardDataBuilder, IndicatorKind, IndicatorState, SizePreset,
            default_indicator_state,
        },
        settings::{
            ALL_TIMEFRAMES, DEFAULT_PRESET_LABEL, MAX_PAIRS, PresetStore, SettingsForm,
            VolatilityTimeframeSetting,
        },
    },
};

use super::ui::render_dashboard;

const TIMEFRAME_TOGGLE_COLOR: Color32 = Color32::from_rgb(33, 150, 243);
const INDICATOR_TOGGLE_COLOR: Color32 = TIMEFRAME_TOGGLE_COLOR;
type IndicatorThresholdMap = BTreeMap<(IndicatorKind, Timeframe), IndicatorThresholds>;
const RSI_OVERSOLD: f32 = 30.0;
const RSI_OVERBOUGHT: f32 = 70.0;

pub struct DashboardApp {
    engine_tx: EngineTx,
    history_tx: HistoryTx,
    ws_tx: WsTx,
    rx: UiRx,
    rt_handle: tokio::runtime::Handle,
    data: DashboardData,
    active_config: Option<AppConfig>,
    indicator_values: Vec<f32>,
    indicator_thresholds: IndicatorThresholdMap,
    indicator_state: IndicatorState,
    size: SizePreset,
    settings_open: bool,
    settings: SettingsForm,
    settings_draft: SettingsForm,
    preset_store: PresetStore,
    active_preset_label: Option<String>,
    selected_preset_label: String,
    new_preset_label: String,
}

impl DashboardApp {
    pub fn new(
        engine_tx: EngineTx,
        history_tx: HistoryTx,
        ws_tx: WsTx,
        rt_handle: tokio::runtime::Handle,
        rx: UiRx,
    ) -> Self {
        let mut preset_store = PresetStore::load();
        let selected_preset_label = DEFAULT_PRESET_LABEL.to_string();
        let settings = preset_store
            .get(&selected_preset_label)
            .map(|preset| preset.settings.clone())
            .unwrap_or_default();
        let data = dashboard_from_settings(&settings);
        let indicator_state = default_indicator_state(&data.indicator_config);
        let settings_draft = settings.clone();
        let indicator_thresholds = indicator_thresholds_from_settings(&settings);

        Self {
            engine_tx,
            history_tx,
            ws_tx,
            rx,
            rt_handle,
            data,
            active_config: None,
            indicator_values: Vec::new(),
            indicator_thresholds,
            indicator_state,
            size: SizePreset::default(),
            settings_open: false,
            settings,
            settings_draft,
            preset_store,
            active_preset_label: None,
            selected_preset_label,
            new_preset_label: String::new(),
        }
    }

    fn select_preset(&mut self, label: String) {
        if let Some(preset) = self.preset_store.get(&label) {
            self.selected_preset_label = label;
            self.settings_draft = preset.settings.clone();
            self.settings_draft.pairs_input =
                uppercase_and_limit_pairs(&self.settings_draft.pairs_input);
        }
    }

    fn commit_settings(&mut self, update_active: bool) {
        let saved_settings = self.settings_draft.clone();
        self.preset_store
            .upsert(self.selected_preset_label.clone(), saved_settings.clone());
        self.preset_store.save();
        if update_active {
            self.settings = saved_settings.clone();
            self.data = dashboard_from_settings(&self.settings);
            self.indicator_state = default_indicator_state(&self.data.indicator_config);
            self.indicator_thresholds = indicator_thresholds_from_settings(&self.settings);
            self.settings_draft = saved_settings;
        }
    }

    fn save_settings(&mut self) {
        let saved_label = self.selected_preset_label.clone();
        let update_active = self
            .active_preset_label
            .as_ref()
            .is_some_and(|active| *active == saved_label);
        self.commit_settings(update_active);
    }

    fn cancel_settings(&mut self) {
        if let Some(preset) = self.preset_store.get(&self.selected_preset_label) {
            self.settings_draft = preset.settings.clone();
        } else {
            self.settings_draft = self.settings.clone();
        }
    }

    fn activate_preset(&mut self, label: String) {
        if let Some(preset) = self.preset_store.get(&label) {
            self.active_preset_label = Some(label.clone());
            self.settings = preset.settings.clone();
            self.settings_draft = self.settings.clone();
            self.data = dashboard_from_settings(&self.settings);
            self.indicator_state = default_indicator_state(&self.data.indicator_config);
            self.indicator_thresholds = indicator_thresholds_from_settings(&self.settings);
            self.selected_preset_label = label;
        }
    }

    fn create_preset_from_draft(&mut self) {
        let label = self.new_preset_label.trim();
        if label.is_empty() {
            return;
        }

        self.selected_preset_label = label.to_string();
        self.commit_settings(false);
        self.new_preset_label.clear();
    }
}

impl App for DashboardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut received_updates = false;
        while let Ok(UiMessage::IndicatorResults(batch)) = self.rx.try_recv() {
            received_updates = true;
            for (idx, val) in batch {
                if idx >= self.indicator_values.len() {
                    self.indicator_values.resize(idx + 1, 0.0);
                }
                match val {
                    IndicatorValue::Volatility(v) | IndicatorValue::Rsi(v) => {
                        self.indicator_values[idx] = v;
                    }
                };
            }
        }

        if received_updates {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(std::time::Duration::from_secs(2));
        }

        TopBottomPanel::top("grid_top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new("THE GRID").size(20.0).strong());
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    let settings = ui
                        .add(
                            egui::Button::new("⚙")
                                .min_size(egui::vec2(32.0, 32.0))
                                .frame(false),
                        )
                        .on_hover_text("Open settings");
                    if settings.clicked() && !self.settings_open {
                        if let Some(active) = &self.active_preset_label {
                            self.selected_preset_label = active.clone();
                        }
                        if let Some(preset) = self.preset_store.get(&self.selected_preset_label) {
                            self.settings_draft = preset.settings.clone();
                        } else {
                            self.settings_draft = self.settings.clone();
                        }
                        self.new_preset_label.clear();
                        self.settings_draft.pairs_input =
                            uppercase_and_limit_pairs(&self.settings_draft.pairs_input);
                        self.settings_open = true;
                    }

                    ui.spacing_mut().item_spacing.x = 6.0;
                    for preset in [
                        SizePreset::Xl,
                        SizePreset::Lg,
                        SizePreset::Md,
                        SizePreset::Sm,
                        SizePreset::Xs,
                    ] {
                        let selected = self.size == preset;
                        let label = preset.label();
                        if ui.selectable_label(selected, label).clicked() {
                            self.size = preset;
                        }
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(config) = &self.active_config {
                render_dashboard(
                    ui,
                    ctx,
                    &self.data,
                    &self.indicator_state,
                    self.size,
                    config.index_lookup(),
                    &self.indicator_values,
                    &self.indicator_thresholds,
                );
            } else {
                render_dashboard(
                    ui,
                    ctx,
                    &self.data,
                    &self.indicator_state,
                    self.size,
                    // fallback empty lookup/value; table will render zeros
                    &config::IndexLookup::new(&[], false, &[], false, &[]),
                    &self.indicator_values,
                    &self.indicator_thresholds,
                );
            }
        });

        if self.settings_open {
            render_settings(ctx, self);
        }
    }
}

fn render_settings(ctx: &egui::Context, app: &mut DashboardApp) {
    let mut settings_open = app.settings_open;
    let center = ctx.input(|i| i.content_rect().center());
    egui::Window::new("Settings")
        .id(Id::new("dashboard_settings"))
        .collapsible(false)
        .open(&mut settings_open)
        .resizable(true)
        .default_pos(center - egui::vec2(320.0, 360.0))
        .default_size(egui::vec2(640.0, 720.0))
        .min_width(540.0)
        .min_height(640.0)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .max_height(ui.available_height() - 60.0)
                    .show(ui, |ui| {
                        ui.heading("Presets");
                        ui.add_space(6.0);
                        let active_text = if let Some(label) = app.active_preset_label.as_ref() {
                            format!("{label} is currently active.")
                        } else {
                            "No preset is currently active.".to_string()
                        };
                        ui.label(active_text);
                        ui.add_space(6.0);
                        render_preset_controls(ui, app);

                        ui.separator();
                        ui.add_space(6.0);

                        ui.heading("Pairs");
                        ui.add_space(6.0);
                        ui.label("Comma separated pairs");
                        ui.add_space(6.0);
                        if pair_count(&app.settings_draft.pairs_input) > MAX_PAIRS {
                            app.settings_draft.pairs_input =
                                uppercase_and_limit_pairs(&app.settings_draft.pairs_input);
                        }
                        let pairs_response = ui.add(
                            TextEdit::multiline(&mut app.settings_draft.pairs_input)
                                .desired_rows(8)
                                .desired_width(f32::INFINITY),
                        );
                        if pairs_response.changed() {
                            app.settings_draft.pairs_input =
                                uppercase_and_limit_pairs(&app.settings_draft.pairs_input);
                        }
                        let pair_count = pair_count(&app.settings_draft.pairs_input);
                        ui.add_space(4.0);
                        ui.label(format!("{pair_count} / {MAX_PAIRS} pairs"));
                        ui.add_space(6.0);

                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.heading("Volatility");
                            ui.add_space(8.0);
                            indicator_toggle(ui, &mut app.settings_draft.volatility_enabled);
                        });
                        ui.add_space(8.0);
                        ui.add_enabled_ui(app.settings_draft.volatility_enabled, |ui| {
                            ui.label("Timeframes");
                            ui.add_space(6.0);
                            render_volatility_timeframes(
                                ui,
                                &mut app.settings_draft.volatility_timeframes,
                            );
                        });

                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.heading("RSI");
                            ui.add_space(8.0);
                            indicator_toggle(ui, &mut app.settings_draft.rsi_enabled);
                        });
                        ui.add_space(8.0);
                        ui.add_enabled_ui(app.settings_draft.rsi_enabled, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("RSI length");
                                ui.add(
                                    egui::DragValue::new(&mut app.settings_draft.rsi_length)
                                        .range(1..=250usize),
                                );
                            });
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.label("Source");
                                egui::ComboBox::from_id_salt("rsi_source_combo")
                                    .selected_text(kline_source_label(
                                        app.settings_draft.rsi_source,
                                    ))
                                    .show_ui(ui, |ui| {
                                        for source in [
                                            KlineSource::Open,
                                            KlineSource::High,
                                            KlineSource::Low,
                                            KlineSource::Close,
                                        ] {
                                            ui.selectable_value(
                                                &mut app.settings_draft.rsi_source,
                                                source,
                                                kline_source_label(source),
                                            );
                                        }
                                    });
                            });
                            ui.add_space(8.0);
                            ui.label("Timeframes");
                            ui.add_space(6.0);
                            render_rsi_timeframes(ui, &mut app.settings_draft.rsi_timeframes);
                        });
                    });

                ui.separator();
                ui.add_space(8.0);
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    let save = ui.add_sized((104.0, 34.0), egui::Button::new("Save"));
                    let cancel = ui.add_sized((104.0, 34.0), egui::Button::new("Cancel"));
                    if save.clicked() {
                        app.save_settings();
                    }
                    if cancel.clicked() {
                        app.cancel_settings();
                    }
                });
            });
        });
    app.settings_open = settings_open;
}

fn render_preset_controls(ui: &mut egui::Ui, app: &mut DashboardApp) {
    let mut selected_label = app.selected_preset_label.clone();
    let labels = app.preset_store.labels();
    let combo_width = 190.0;

    ui.horizontal(|ui| {
        ui.scope(|ui| {
            let original_height = ui.spacing().interact_size.y;
            ui.spacing_mut().interact_size.y = 32.0;

            egui::ComboBox::from_id_salt("preset_selector")
                .selected_text(selected_label.clone())
                .width(combo_width)
                .show_ui(ui, |ui| {
                    for label in &labels {
                        ui.selectable_value(&mut selected_label, label.clone(), label);
                    }
                });

            let play = ui
                .add_enabled(
                    !selected_label.is_empty(),
                    egui::Button::new("▶").min_size(egui::vec2(32.0, 32.0)),
                )
                .on_hover_text("Activate selected preset");
            if play.clicked() {
                app.activate_preset(selected_label.clone());
                let config = AppConfig::from_settings(&app.settings);
                let cfg_for_history = config.clone();
                let cfg_for_ws = config.clone();
                let tx1 = app.engine_tx.clone();
                let tx2 = app.history_tx.clone();
                let tx3 = app.ws_tx.clone();
                let rt_handle = app.rt_handle.clone();

                let pair_stride = config.index_lookup().pair_stride();
                let total_slots = config.index_lookup().pair_count() * pair_stride;
                app.indicator_values = vec![0.0; total_slots];
                app.active_config = Some(config.clone());

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

            ui.spacing_mut().interact_size.y = original_height;
        });
    });

    if selected_label != app.selected_preset_label {
        app.select_preset(selected_label);
    }

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label("New preset");
        ui.add(TextEdit::singleline(&mut app.new_preset_label).hint_text("Name for preset"));
        let create = ui.add_enabled(
            !app.new_preset_label.trim().is_empty(),
            egui::Button::new("Create from current"),
        );
        if create.clicked() {
            app.create_preset_from_draft();
        }
    });
}

fn pair_count(input: &str) -> usize {
    input
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .count()
}

fn uppercase_and_limit_pairs(input: &str) -> String {
    let uppercased = input.to_ascii_uppercase();
    if pair_count(&uppercased) <= MAX_PAIRS {
        return uppercased;
    }
    clamp_pairs_input(&uppercased)
}

fn clamp_pairs_input(input: &str) -> String {
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

fn tf_label(tf: Timeframe) -> String {
    tf.to_string().to_owned()
}

fn kline_source_label(source: KlineSource) -> &'static str {
    match source {
        KlineSource::Open => "Open",
        KlineSource::High => "High",
        KlineSource::Low => "Low",
        KlineSource::Close => "Close",
    }
}

fn toggle_switch_colored(ui: &mut egui::Ui, value: &mut bool, label: &str, color: Color32) {
    ui.scope(|ui| {
        let mut style: egui::Style = ui.style().as_ref().clone();
        style.visuals.widgets.inactive.bg_fill = color.linear_multiply(0.35);
        style.visuals.widgets.hovered.bg_fill = color.linear_multiply(0.45);
        style.visuals.widgets.active.bg_fill = color;
        style.visuals.widgets.inactive.fg_stroke.color = Color32::WHITE;
        style.visuals.widgets.hovered.fg_stroke.color = Color32::WHITE;
        style.visuals.widgets.active.fg_stroke.color = Color32::WHITE;
        ui.set_style(style);
        ui.horizontal(|ui| {
            ui.toggle_value(value, label);
        });
    });
}

fn indicator_toggle(ui: &mut egui::Ui, value: &mut bool) -> egui::Response {
    let desired_size = egui::vec2(38.0, 20.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let radius = rect.height() * 0.5;
        let visuals = ui.visuals().widgets.clone();
        let stroke = visuals.active.bg_stroke;
        let fill_on = INDICATOR_TOGGLE_COLOR;
        let fill_off = visuals.inactive.bg_fill;
        let bg_fill = if *value { fill_on } else { fill_off };
        ui.painter()
            .rect(rect, radius, bg_fill, stroke, egui::StrokeKind::Inside);

        let center_x = if *value {
            rect.right() - radius
        } else {
            rect.left() + radius
        };
        let center = egui::pos2(center_x, rect.center().y);
        ui.painter()
            .circle(center, radius * 0.68, Color32::WHITE, stroke);
    }

    response
}

fn render_volatility_timeframes(
    ui: &mut egui::Ui,
    timeframes: &mut BTreeMap<Timeframe, VolatilityTimeframeSetting>,
) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(14.0, 14.0);
        for tf in ALL_TIMEFRAMES {
            let entry = timeframes.entry(tf).or_insert(VolatilityTimeframeSetting {
                enabled: false,
                threshold: 0.0,
            });
            entry.threshold = entry.threshold.abs();

            ui.vertical(|ui| {
                toggle_switch_colored(
                    ui,
                    &mut entry.enabled,
                    tf_label(tf).as_str(),
                    TIMEFRAME_TOGGLE_COLOR,
                );
                ui.add_space(6.0);
                ui.add_enabled_ui(entry.enabled, |ui| {
                    ui.label("Threshold");
                    ui.add(
                        egui::DragValue::new(&mut entry.threshold)
                            .speed(0.25)
                            .range(0.0..=10_000.0),
                    );
                });
                ui.add_space(4.0);
            });
        }
    });
}

fn render_rsi_timeframes(ui: &mut egui::Ui, timeframes: &mut BTreeMap<Timeframe, bool>) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(14.0, 12.0);
        for tf in ALL_TIMEFRAMES {
            let entry = timeframes.entry(tf).or_insert(false);
            toggle_switch_colored(ui, entry, tf_label(tf).as_str(), TIMEFRAME_TOGGLE_COLOR);
        }
    });
}

fn indicator_thresholds_from_settings(settings: &SettingsForm) -> IndicatorThresholdMap {
    let mut thresholds = IndicatorThresholdMap::new();

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
                    oversold: RSI_OVERSOLD,
                    overbought: RSI_OVERBOUGHT,
                },
            );
        }
    }

    thresholds
}

fn dashboard_from_settings(settings: &SettingsForm) -> DashboardData {
    DashboardDataBuilder::new()
        .indicator_config(settings.indicator_config())
        .pairs(settings.pairs())
        .build()
}
