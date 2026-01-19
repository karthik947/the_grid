use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
};

use crate::tui::data::{IndicatorConfig, IndicatorKind, PairRow};

use super::{intro::render_intro, super::app::DashboardApp};
use super::super::util::{
    HEADER_COLOR, INDICATOR_GROUP_BG, PAIR_COLOR, lookup_value, tf_label, value_style,
};

pub fn render_header(_frame: &mut Frame, _area: Rect, _app: &DashboardApp) {}

pub fn render_dashboard(frame: &mut Frame, area: Rect, app: &mut DashboardApp) {
    let active = app
        .active_indicators()
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    let settings = app.settings();
    let column_spacing = settings.layout_column_spacing;
    let table_spacing = settings.layout_table_spacing;
    let mut widths: Vec<Constraint> = Vec::new();
    widths.push(Constraint::Length(app.pair_width(&area)));
    for cfg in active.iter() {
        for _ in &cfg.timeframes {
            widths.push(Constraint::Length(app.value_width()));
        }
    }
    let column_widths: Vec<u16> = widths
        .iter()
        .map(|constraint| match *constraint {
            Constraint::Length(value) => value,
            _ => 0,
        })
        .collect();

    let title = if app.active_preset_label().is_none() {
        "The grid"
    } else {
        "the_grid"
    };
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if inner_area.height < 3 || inner_area.width == 0 {
        return;
    }

    if app.active_preset_label().is_none() {
        render_intro(frame, inner_area);
        return;
    }

    let layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .split(inner_area);
    let table_area = layout[2];

    let column_count = widths.len() as u16;
    let table_width = widths
        .iter()
        .map(|constraint| match *constraint {
            Constraint::Length(value) => value,
            _ => 0,
        })
        .sum::<u16>()
        .saturating_add(column_spacing.saturating_mul(column_count.saturating_sub(1)));
    let requested_tables = settings.layout_table_count.max(1);
    let max_tables = if table_width == 0 {
        1
    } else {
        let total = table_width.saturating_add(table_spacing);
        (inner_area.width.saturating_add(table_spacing) / total).max(1)
    };
    let table_count = requested_tables.min(max_tables);

    let mut table_constraints = Vec::with_capacity(table_count as usize + 1);
    for _ in 0..table_count {
        table_constraints.push(Constraint::Length(table_width));
    }
    table_constraints.push(Constraint::Min(0));

    let table_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(table_constraints)
        .spacing(table_spacing)
        .split(inner_area);

    if app.table_state().selected().is_none() && !app.pairs().is_empty() {
        app.table_state().select(Some(0));
    }

    let visible_rows = table_area.height.saturating_sub(1) as usize;
    let visible_pairs = (visible_rows / 2).max(1);
    let total_visible_pairs = visible_pairs.saturating_mul(table_count as usize).max(1);
    app.ensure_selection_visible(total_visible_pairs);

    let (selected_pair, offset_pair) = {
        let state = app.table_state();
        (state.selected(), state.offset())
    };

    for idx in 0..table_count as usize {
        let column_area = table_columns[idx];
        let inner = column_area;
        let column_layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);
        let group_area = column_layout[0];
        let separator_area = column_layout[1];
        let body_area = column_layout[2];

        render_indicator_groups(frame, group_area, &active, &widths, column_spacing);
        render_header_separator(frame, separator_area, &widths, column_spacing);

        let start_pair = offset_pair.saturating_add(idx.saturating_mul(visible_pairs));
        let rows = dashboard_rows_range(app, &active, &column_widths, start_pair, visible_pairs);

        let table = Table::new(rows, widths.clone())
            .header(timeframe_header(&active))
            .column_spacing(column_spacing)
            .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        let mut table_state = TableState::default();
        if let Some(selected) = selected_pair {
            if selected >= start_pair && selected < start_pair + visible_pairs {
                let local = selected - start_pair;
                table_state.select(Some(local.saturating_mul(2).saturating_add(1)));
            }
        }

        frame.render_stateful_widget(table, body_area, &mut table_state);
    }
}

fn timeframe_header(active: &[IndicatorConfig]) -> Row<'static> {
    let mut cells = Vec::new();
    let pair_text = Text::from(Span::styled(
        "PAIR",
        Style::default().fg(PAIR_COLOR).add_modifier(Modifier::BOLD),
    ))
    .centered();
    cells.push(Cell::from(pair_text));

    for (idx, cfg) in active.iter().enumerate() {
        for tf in &cfg.timeframes {
            let tf_text = Text::from(Span::styled(
                tf_label(*tf),
                Style::default()
                    .fg(HEADER_COLOR)
                    .add_modifier(Modifier::BOLD),
            ))
            .centered();
            cells.push(Cell::from(tf_text));
        }
    }

    Row::new(cells).height(1)
}

fn render_indicator_groups(
    frame: &mut Frame,
    area: Rect,
    active: &[IndicatorConfig],
    widths: &[Constraint],
    column_spacing: u16,
) {
    if area.height == 0 || widths.is_empty() {
        return;
    }

    let column_rects = Layout::horizontal(widths.to_vec())
        .flex(Flex::Start)
        .spacing(column_spacing)
        .split(Rect::new(area.x, area.y, area.width, area.height));

    if column_rects.is_empty() {
        return;
    }

    let mut column_idx = 1;
    for (idx, cfg) in active.iter().enumerate() {
        if cfg.timeframes.is_empty() {
            continue;
        }

        let start_idx = column_idx;
        column_idx += cfg.timeframes.len();
        let end_idx = column_idx.saturating_sub(1);

        if end_idx >= column_rects.len() {
            break;
        }

        let start_rect = column_rects[start_idx];
        let end_rect = column_rects[end_idx];
        let group_width = end_rect.x + end_rect.width - start_rect.x;
        if group_width == 0 {
            continue;
        }

        let group_rect = Rect {
            x: start_rect.x,
            y: area.y,
            width: group_width,
            height: area.height,
        };

        let border = if idx + 1 == active.len() {
            Borders::NONE
        } else {
            Borders::RIGHT
        };
        let indicator_header = Paragraph::new(cfg.kind.label())
            .block(
                Block::default()
                    .borders(border)
                    .border_style(Style::default().fg(Color::Black)),
            )
            .style(
                Style::default()
                    .bg(INDICATOR_GROUP_BG)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        frame.render_widget(indicator_header, group_rect);
    }
}

fn render_header_separator(
    frame: &mut Frame,
    area: Rect,
    widths: &[Constraint],
    column_spacing: u16,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let total_width: u16 = widths
        .iter()
        .map(|constraint| match *constraint {
            Constraint::Length(value) => value,
            _ => 0,
        })
        .sum::<u16>()
        .saturating_add(column_spacing.saturating_mul(widths.len().saturating_sub(1) as u16));
    let line_width = total_width.min(area.width).max(1) as usize;
    let line_style = Style::default().fg(Color::DarkGray);
    let text = "-".repeat(line_width);
    let separator = Paragraph::new(Text::from(Span::styled(text, line_style)));
    frame.render_widget(separator, area);
}

fn dashboard_rows_range(
    app: &DashboardApp,
    active: &[IndicatorConfig],
    column_widths: &[u16],
    start_idx: usize,
    max_pairs: usize,
) -> Vec<Row<'static>> {
    let pair_count = app.pairs().len();
    let mut rows = Vec::with_capacity(pair_count.saturating_mul(2));
    if pair_count == 0 || start_idx >= pair_count || max_pairs == 0 {
        return rows;
    }
    let end_idx = (start_idx + max_pairs).min(pair_count);
    rows.push(separator_row(column_widths));
    for (idx, row) in app.pairs()[start_idx..end_idx].iter().enumerate() {
        rows.push(dashboard_row(app, row, active));
        if idx + 1 < end_idx - start_idx {
            rows.push(separator_row(column_widths));
        }
    }
    rows
}

fn dashboard_row(app: &DashboardApp, pair: &PairRow, active: &[IndicatorConfig]) -> Row<'static> {
    let mut cells = Vec::new();
    cells.push(Cell::from(Span::styled(
        pair.pair.clone(),
        Style::default().fg(PAIR_COLOR),
    )));

    for (idx, cfg) in active.iter().enumerate() {
        for tf in &cfg.timeframes {
            let (value, display) = lookup_value(
                &pair.pair,
                cfg.kind,
                *tf,
                app.active_index_lookup(),
                app.indicator_values(),
                app.indicator_labels(),
            );
            let threshold = app.indicator_thresholds().get(&(cfg.kind, *tf));
            let indicator_value = match cfg.kind {
                IndicatorKind::Volatility => crate::message_bus::IndicatorValue::Volatility(value),
                IndicatorKind::Rsi => crate::message_bus::IndicatorValue::Rsi(value),
            };
            let style = value_style(&indicator_value, threshold);
            let text = display
                .map(|d| d.to_string())
                .unwrap_or_else(|| indicator_value.display());
            let cell_text = Text::from(Span::styled(text, style)).centered();
            cells.push(Cell::from(cell_text));
        }
    }

    Row::new(cells)
}

fn separator_row(column_widths: &[u16]) -> Row<'static> {
    let line_style = Style::default().fg(Color::DarkGray);
    let cells = column_widths
        .iter()
        .map(|width| {
            let len = (*width).max(1) as usize;
            let text = "-".repeat(len);
            Cell::from(Span::styled(text, line_style))
        })
        .collect::<Vec<_>>();
    Row::new(cells)
}

pub fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new("↑/↓ to scroll rows • s: settings • l: layout • q: quit")
        .wrap(Wrap { trim: true });
    frame.render_widget(footer, area);
}
