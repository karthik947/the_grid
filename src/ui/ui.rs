use std::collections::BTreeMap;

use egui::{Align, Color32, FontId, Layout, RichText, ScrollArea, StrokeKind, Ui};
use egui_extras::{Column, TableBuilder};

use crate::{
    message_bus::{IndicatorThresholds, IndicatorValue},
    types::{
        Timeframe,
        config::{IndexLookup, IndicatorKey},
    },
    ui::data::{
        DashboardData, IndicatorConfig, IndicatorKind, IndicatorState, SizePreset, UiMetrics,
    },
};

#[derive(Clone, Debug)]
pub struct LayoutInfo {
    pub pair_width: f32,
    pub value_width: f32,
    pub row_width: f32,
    pub columns: usize,
    pub rows_per_column: usize,
}

const CELL_BG: Color32 = Color32::from_rgb(26, 26, 26);
const CELL_BORDER: Color32 = Color32::from_gray(70);
const TIMEFRAME_COLOR: Color32 = Color32::from_rgb(90, 200, 255);
const PAIR_TEXT_COLOR: Color32 = Color32::from_gray(170);

pub fn render_dashboard(
    ui: &mut Ui,
    ctx: &egui::Context,
    data: &DashboardData,
    indicator_state: &IndicatorState,
    size: SizePreset,
    index_lookup: &IndexLookup,
    indicator_values: &[f32],
    indicator_thresholds: &BTreeMap<(IndicatorKind, Timeframe), IndicatorThresholds>,
) {
    let metrics: UiMetrics = size.into();
    let active = active_indicators(&data.indicator_config, indicator_state);
    let layout = compute_layout(ctx, data, &active, &metrics, ui.available_width());

    // Render header once so it stays visible while the body scrolls.
    ui.horizontal_wrapped(|ui| {
        ui.set_width(ui.available_width());
        render_header_columns(ui, &active, &layout, &metrics);
    });

    ScrollArea::vertical()
        .id_salt("dashboard_scroll")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.set_width(ui.available_width());
                render_body_columns(
                    ui,
                    data,
                    &active,
                    &layout,
                    &metrics,
                    index_lookup,
                    indicator_values,
                    indicator_thresholds,
                );
            });
        });
}

fn active_indicators<'a>(
    config: &'a [IndicatorConfig],
    state: &IndicatorState,
) -> Vec<&'a IndicatorConfig> {
    config
        .iter()
        .filter(|cfg| *state.get(&cfg.kind).unwrap_or(&cfg.enabled))
        .collect()
}

fn compute_layout(
    ctx: &egui::Context,
    data: &DashboardData,
    active: &[&IndicatorConfig],
    metrics: &UiMetrics,
    available_width: f32,
) -> LayoutInfo {
    let font_id = FontId::proportional(metrics.font_size);
    let pair_width = ctx.fonts_mut(|fonts| {
        data.pairs
            .iter()
            .map(|row| {
                let width = fonts
                    .layout_no_wrap(row.pair.clone(), font_id.clone(), Color32::WHITE)
                    .rect
                    .width();
                width + (metrics.cell_padding * 2.0)
            })
            .fold(70.0, f32::max)
    });

    let sample_value = "+12.3%";
    let value_width = ctx.fonts_mut(|fonts| {
        fonts
            .layout_no_wrap(sample_value.to_string(), font_id.clone(), Color32::WHITE)
            .rect
            .width()
            + (metrics.cell_padding * 2.0)
    });

    let indicator_width: f32 = active
        .iter()
        .map(|cfg| cfg.timeframes.len() as f32 * value_width)
        .sum();

    let group_gaps = if active.len() > 1 {
        metrics.group_gap * (active.len() as f32 - 1.0)
    } else {
        0.0
    };

    let row_width = pair_width + indicator_width + group_gaps;
    let usable_width = available_width.max(row_width);
    let columns = (usable_width / row_width).floor().max(1.0) as usize;
    let rows_per_column = if columns == 0 {
        data.pairs.len()
    } else {
        (data.pairs.len() + columns - 1) / columns
    };

    LayoutInfo {
        pair_width,
        value_width,
        row_width,
        columns: columns.max(1),
        rows_per_column: rows_per_column.max(1),
    }
}

fn render_header_columns(
    ui: &mut Ui,
    active: &[&IndicatorConfig],
    layout: &LayoutInfo,
    metrics: &UiMetrics,
) {
    let mut column_idx = 0usize;

    while column_idx < layout.columns {
        ui.vertical(|ui| {
            ui.push_id(format!("header_{column_idx}"), |ui| {
                render_table_header(ui, active, layout, metrics);
            });
        });
        column_idx += 1;
        if column_idx < layout.columns {
            ui.add_space(metrics.column_gap);
        }
    }
}

fn render_body_columns(
    ui: &mut Ui,
    data: &DashboardData,
    active: &[&IndicatorConfig],
    layout: &LayoutInfo,
    metrics: &UiMetrics,
    index_lookup: &IndexLookup,
    indicator_values: &[f32],
    indicator_thresholds: &BTreeMap<(IndicatorKind, Timeframe), IndicatorThresholds>,
) {
    let mut start = 0usize;
    let total = data.pairs.len();
    let mut column_idx = 0usize;

    while start < total {
        let end = (start + layout.rows_per_column).min(total);
        let slice = &data.pairs[start..end];

        ui.vertical(|ui| {
            ui.push_id(column_idx, |ui| {
                render_table_body(
                    ui,
                    slice,
                    active,
                    layout,
                    metrics,
                    index_lookup,
                    indicator_values,
                    indicator_thresholds,
                );
            });
        });

        start = end;
        column_idx += 1;
        if start < total {
            ui.add_space(metrics.column_gap);
        }
    }
}

fn base_table<'a>(
    ui: &'a mut Ui,
    active: &[&IndicatorConfig],
    layout: &LayoutInfo,
    metrics: &UiMetrics,
) -> TableBuilder<'a> {
    let mut table = TableBuilder::new(ui)
        .cell_layout(Layout::left_to_right(Align::Center))
        .min_scrolled_height(0.0)
        .max_scroll_height(f32::INFINITY)
        .vscroll(false)
        .column(Column::exact(layout.pair_width));

    for (idx, cfg) in active.iter().enumerate() {
        for _ in &cfg.timeframes {
            table = table.column(Column::exact(layout.value_width));
        }
        if idx + 1 < active.len() {
            table = table.column(Column::exact(metrics.group_gap));
        }
    }

    table
}

fn render_table_header(
    ui: &mut Ui,
    active: &[&IndicatorConfig],
    layout: &LayoutInfo,
    metrics: &UiMetrics,
) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

        let mut table = base_table(ui, active, layout, metrics);
        table = table.id_salt("dashboard_header_table");

        table
            .header(metrics.header_height, |mut header| {
                header.col(|ui| {
                    ui.with_layout(
                        Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            ui.strong(
                                RichText::new("PAIR")
                                    .size(metrics.font_size)
                                    .color(PAIR_TEXT_COLOR),
                            );
                        },
                    );
                });

                for (group_idx, cfg) in active.iter().enumerate() {
                    for timeframe in &cfg.timeframes {
                        header.col(|ui| {
                            let rect = ui.max_rect();
                            ui.allocate_ui_with_layout(
                                rect.size(),
                                Layout::bottom_up(Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(cfg.kind.label().to_lowercase())
                                            .size(metrics.header_indicator_font_size)
                                            .color(PAIR_TEXT_COLOR),
                                    );
                                    ui.label(
                                        RichText::new(tf_label(*timeframe))
                                            .size(metrics.header_timeframe_font_size)
                                            .strong()
                                            .color(TIMEFRAME_COLOR),
                                    );
                                },
                            );
                        });
                    }
                    if group_idx + 1 < active.len() {
                        header.col(|_| {});
                    }
                }
            })
            .body(|mut body| {
                // Empty body; header-only table.
                let _ = &mut body;
            });
    });
}

fn render_table_body(
    ui: &mut Ui,
    rows: &[crate::ui::data::PairRow],
    active: &[&IndicatorConfig],
    layout: &LayoutInfo,
    metrics: &UiMetrics,
    index_lookup: &IndexLookup,
    indicator_values: &[f32],
    indicator_thresholds: &BTreeMap<(IndicatorKind, Timeframe), IndicatorThresholds>,
) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

        let mut table = base_table(ui, active, layout, metrics);
        table = table.id_salt("dashboard_body_table");

        table.body(|mut body| {
            for row in rows {
                body.row(metrics.row_height, |mut row_ui| {
                    row_ui.col(|ui| {
                        paint_cell_fill_and_border(ui, CELL_BG);
                        ui.with_layout(
                            Layout::centered_and_justified(egui::Direction::TopDown),
                            |ui| {
                                ui.label(
                                    RichText::new(row.pair.clone())
                                        .size(metrics.font_size)
                                        .color(PAIR_TEXT_COLOR),
                                );
                            },
                        );
                    });

                    for (group_idx, cfg) in active.iter().enumerate() {
                        for timeframe in &cfg.timeframes {
                            let value = lookup_value(
                                &row.pair,
                                cfg.kind,
                                *timeframe,
                                index_lookup,
                                indicator_values,
                            );
                            let threshold = indicator_thresholds.get(&(cfg.kind, *timeframe));
                            row_ui.col(|ui| {
                                render_value(ui, cfg.kind, *timeframe, value, threshold, metrics);
                            });
                        }

                        if group_idx + 1 < active.len() {
                            row_ui.col(|_| {});
                        }
                    }
                });
            }
        });
    });
}

fn lookup_value(
    pair: &str,
    kind: crate::ui::data::IndicatorKind,
    timeframe: Timeframe,
    index_lookup: &IndexLookup,
    indicator_values: &[f32],
) -> f32 {
    let key = match kind {
        crate::ui::data::IndicatorKind::Volatility => IndicatorKey::Volatility,
        crate::ui::data::IndicatorKind::Rsi => IndicatorKey::Rsi,
    };

    if let Some(idx) = index_lookup.index(pair, key, timeframe) {
        indicator_values.get(idx).copied().unwrap_or(0.0)
    } else {
        0.0
    }
}

fn render_value(
    ui: &mut Ui,
    kind: IndicatorKind,
    timeframe: Timeframe,
    value: f32,
    thresholds: Option<&IndicatorThresholds>,
    metrics: &UiMetrics,
) {
    let indicator_value = match kind {
        IndicatorKind::Volatility => IndicatorValue::Volatility(value),
        IndicatorKind::Rsi => IndicatorValue::Rsi(value),
    };
    let colors = indicator_value.colors(thresholds);

    let fill = colors.background.unwrap_or(CELL_BG);
    paint_cell_fill_and_border(ui, fill);

    ui.with_layout(
        Layout::centered_and_justified(egui::Direction::TopDown),
        |ui| {
            let rich = RichText::new(indicator_value.display())
                .color(colors.text)
                .size(metrics.font_size);
            ui.label(rich);
        },
    );
}

fn tf_label(tf: Timeframe) -> String {
    tf.to_string().to_owned()
}

fn paint_cell_fill_and_border(ui: &mut Ui, fill: Color32) {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, fill);
    paint_cell_border(ui);
}

fn paint_cell_border(ui: &mut Ui) {
    let rect = ui.max_rect();
    ui.painter().rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, CELL_BORDER),
        StrokeKind::Middle,
    );
}
