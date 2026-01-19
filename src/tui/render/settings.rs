use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, Wrap,
    },
};

use crate::tui::settings::{ALL_TIMEFRAMES, MAX_PAIRS, VolatilityTimeframeSetting};

use super::super::{
    app::DashboardApp,
    types::SettingsField,
    util::{
        ACTIVE_ROW_BG, FIELD_ACTIVE, FIELD_INACTIVE, HEADER_COLOR, PAIR_COLOR, centered_rect,
        kline_source_label, pair_count, tf_label, toggle_label,
    },
};

pub fn render_settings(frame: &mut Frame, app: &DashboardApp) {
    let area = centered_rect(92, 92, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default().borders(Borders::ALL).title("Settings");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(26), Constraint::Percentage(74)])
        .split(inner);

    render_presets(frame, columns[0], app);
    render_detail(frame, columns[1], app);

    if app.clone_modal_open() {
        render_clone_modal(frame, app, area);
    }
}

fn render_presets(frame: &mut Frame, area: Rect, app: &DashboardApp) {
    let labels = app.preset_labels();
    let focus = matches!(app.focus_field(), SettingsField::PresetChips);
    let mut items = Vec::new();

    for label in &labels {
        let selected = *label == app.selected_preset_label();
        let active = app.active_preset_label() == Some(label);
        let mut style = Style::default().fg(FIELD_INACTIVE);
        if active {
            style = style.fg(HEADER_COLOR).add_modifier(Modifier::BOLD);
        }
        if selected {
            style = style.fg(Color::White);
        }
        let prefix = if active { "*" } else { " " };
        let line = Line::from(Span::styled(format!("{prefix} {label}"), style));
        items.push(ListItem::new(line));
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "No presets",
            Style::default().fg(FIELD_INACTIVE),
        ))));
    }

    let mut state = ListState::default();
    if let Some(idx) = labels
        .iter()
        .position(|label| label == app.selected_preset_label())
    {
        state.select(Some(idx));
    }

    let highlight_style = if focus {
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().bg(Color::Rgb(30, 30, 30))
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Presets"))
        .highlight_style(highlight_style);
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_detail(frame: &mut Frame, area: Rect, app: &DashboardApp) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(17),
            Constraint::Min(7),
            Constraint::Min(7),
            Constraint::Length(3),
        ])
        .split(area);

    render_top_bar(frame, sections[0], app);
    render_pairs(frame, sections[1], app);
    render_volatility(frame, sections[2], app);
    render_rsi(frame, sections[3], app);
    render_actions(frame, sections[4], app);
}

fn render_top_bar(frame: &mut Frame, area: Rect, app: &DashboardApp) {
    let row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(18),
            Constraint::Min(1),
            Constraint::Length(12),
        ])
        .split(area);

    let selected_is_active = app
        .active_preset_label()
        .is_some_and(|label| label == app.selected_preset_label());
    let active_label = if selected_is_active {
        "Active"
    } else {
        "Set Active"
    };
    let focus_active = matches!(app.focus_field(), SettingsField::ActivatePreset);
    let mut active_style = if selected_is_active {
        Style::default().fg(Color::Black).bg(FIELD_ACTIVE)
    } else {
        Style::default().fg(PAIR_COLOR)
    };
    if focus_active {
        active_style = active_style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
    }
    let active_button = Paragraph::new(active_label)
        .alignment(Alignment::Center)
        .style(active_style)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(active_button, row[0]);

    let focus_clone = matches!(app.focus_field(), SettingsField::ClonePreset);
    let mut clone_style = Style::default().fg(PAIR_COLOR);
    if focus_clone {
        clone_style = clone_style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
    }
    let clone_button = Paragraph::new("Clone")
        .alignment(Alignment::Center)
        .style(clone_style)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(clone_button, row[2]);
}

fn render_pairs(frame: &mut Frame, area: Rect, app: &DashboardApp) {
    let focus = matches!(app.focus_field(), SettingsField::PairsInput);
    let pairs = &app.settings_draft().pairs_input;
    let filled = !pairs.trim().is_empty();
    let input_style = field_style(filled, focus);
    let display = if filled {
        pairs.to_string()
    } else {
        "Type pairs (comma separated)".to_string()
    };

    let block = Block::default().borders(Borders::ALL).title("Pairs");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let input = Paragraph::new(Text::from(display))
        .style(input_style)
        .wrap(Wrap { trim: false });
    frame.render_widget(input, layout[0]);

    let count = pair_count(pairs);
    let count_line = Line::from(Span::styled(
        format!("{count} / {MAX_PAIRS} pairs"),
        Style::default().fg(FIELD_INACTIVE),
    ));
    frame.render_widget(Paragraph::new(count_line), layout[1]);
}

fn render_volatility(frame: &mut Frame, area: Rect, app: &DashboardApp) {
    let focus = app.focus_field();
    let block = Block::default().borders(Borders::ALL).title("Volatility");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    frame.render_widget(Paragraph::new(""), layout[0]);

    let mut toggle_style = if app.settings_draft().volatility_enabled {
        Style::default().fg(FIELD_ACTIVE)
    } else {
        Style::default().fg(FIELD_INACTIVE)
    };
    if matches!(focus, SettingsField::VolatilityEnabled) {
        toggle_style = toggle_style.add_modifier(Modifier::UNDERLINED);
    }
    let header_line = Line::from(vec![
        Span::raw("Enabled "),
        Span::styled(toggle_label(app.settings_draft().volatility_enabled), toggle_style),
    ]);
    frame.render_widget(Paragraph::new(header_line), layout[1]);
    frame.render_widget(Paragraph::new(""), layout[2]);

    let widths = [10, 12, 8];
    let spacing = 0;
    let table_width = widths.iter().sum::<u16>();
    let separator_style = Style::default().fg(Color::DarkGray);

    let header = Row::new(vec![
        Cell::from(cell_text(
            "Timeframe",
            widths[0],
            Style::default().fg(HEADER_COLOR).add_modifier(Modifier::BOLD),
            separator_style,
        )),
        Cell::from(cell_text(
            "Threshold",
            widths[1],
            Style::default().fg(HEADER_COLOR).add_modifier(Modifier::BOLD),
            separator_style,
        )),
        Cell::from(cell_text(
            "Active",
            widths[2],
            Style::default().fg(HEADER_COLOR).add_modifier(Modifier::BOLD),
            separator_style,
        )),
    ])
    .height(2);

    let rows = ALL_TIMEFRAMES.iter().map(|tf| {
        let entry = app
            .settings_draft()
            .volatility_timeframes
            .get(tf)
            .cloned()
            .unwrap_or(VolatilityTimeframeSetting {
                enabled: false,
                threshold: 0.0,
            });
        let is_focus = matches!(focus, SettingsField::VolatilityTf(active) if active == *tf);
        let mut value_style = if entry.enabled {
            Style::default().fg(FIELD_ACTIVE)
        } else {
            Style::default().fg(FIELD_INACTIVE)
        };
        if is_focus {
            value_style = value_style.add_modifier(Modifier::UNDERLINED);
        }
        let mut tf_style = if entry.enabled {
            Style::default().fg(FIELD_ACTIVE)
        } else {
            Style::default().fg(FIELD_INACTIVE)
        };
        if is_focus {
            tf_style = tf_style.add_modifier(Modifier::UNDERLINED);
        }
        if entry.enabled {
            value_style = value_style.bg(ACTIVE_ROW_BG);
            tf_style = tf_style.bg(ACTIVE_ROW_BG);
        }

        Row::new(vec![
            Cell::from(cell_text(
                tf_label(*tf),
                widths[0],
                tf_style,
                separator_style,
            )),
            Cell::from(cell_text(
                format!("{:.1}", entry.threshold),
                widths[1],
                value_style,
                separator_style,
            )),
            Cell::from(cell_text(
                if entry.enabled { "Yes" } else { "No" },
                widths[2],
                value_style,
                separator_style,
            )),
        ])
        .height(2)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(widths[0]),
            Constraint::Length(widths[1]),
            Constraint::Length(widths[2]),
        ],
    )
    .header(header)
    .column_spacing(spacing);

    let table_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(table_width), Constraint::Min(0)])
        .split(layout[3])[0];
    frame.render_widget(table, table_area);
}

fn render_rsi(frame: &mut Frame, area: Rect, app: &DashboardApp) {
    let focus = app.focus_field();
    let block = Block::default().borders(Borders::ALL).title("RSI");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    frame.render_widget(Paragraph::new(""), layout[0]);

    let mut toggle_style = if app.settings_draft().rsi_enabled {
        Style::default().fg(FIELD_ACTIVE)
    } else {
        Style::default().fg(FIELD_INACTIVE)
    };
    if matches!(focus, SettingsField::RsiEnabled) {
        toggle_style = toggle_style.add_modifier(Modifier::UNDERLINED);
    }
    let header_line = Line::from(vec![
        Span::raw("Enabled "),
        Span::styled(toggle_label(app.settings_draft().rsi_enabled), toggle_style),
    ]);
    frame.render_widget(Paragraph::new(header_line), layout[1]);
    frame.render_widget(Paragraph::new(""), layout[2]);

    let fields = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(14), Constraint::Length(20), Constraint::Min(1)])
        .split(layout[3]);

    let length_focus = matches!(focus, SettingsField::RsiLength);
    let length_style = field_style(true, length_focus);
    let length_input = Paragraph::new(app.settings_draft().rsi_length.to_string())
        .alignment(Alignment::Center)
        .style(length_style)
        .block(Block::default().borders(Borders::ALL).title("Length"));
    frame.render_widget(length_input, fields[0]);

    let source_focus = matches!(focus, SettingsField::RsiSource);
    let source_style = field_style(true, source_focus);
    let source_label = format!("{} v", kline_source_label(app.settings_draft().rsi_source));
    let source_input = Paragraph::new(source_label)
        .alignment(Alignment::Center)
        .style(source_style)
        .block(Block::default().borders(Borders::ALL).title("Source"));
    frame.render_widget(source_input, fields[1]);
    frame.render_widget(Paragraph::new(""), layout[4]);

    let widths = [10, 8];
    let spacing = 0;
    let table_width = widths.iter().sum::<u16>();
    let separator_style = Style::default().fg(Color::DarkGray);

    let header = Row::new(vec![
        Cell::from(cell_text(
            "Timeframe",
            widths[0],
            Style::default().fg(HEADER_COLOR).add_modifier(Modifier::BOLD),
            separator_style,
        )),
        Cell::from(cell_text(
            "Active",
            widths[1],
            Style::default().fg(HEADER_COLOR).add_modifier(Modifier::BOLD),
            separator_style,
        )),
    ])
    .height(2);

    let rows = ALL_TIMEFRAMES.iter().map(|tf| {
        let enabled = *app.settings_draft().rsi_timeframes.get(tf).unwrap_or(&false);
        let is_focus = matches!(focus, SettingsField::RsiTf(active) if active == *tf);
        let mut value_style = if enabled {
            Style::default().fg(FIELD_ACTIVE)
        } else {
            Style::default().fg(FIELD_INACTIVE)
        };
        if is_focus {
            value_style = value_style.add_modifier(Modifier::UNDERLINED);
        }
        let mut tf_style = if enabled {
            Style::default().fg(FIELD_ACTIVE)
        } else {
            Style::default().fg(FIELD_INACTIVE)
        };
        if is_focus {
            tf_style = tf_style.add_modifier(Modifier::UNDERLINED);
        }
        if enabled {
            value_style = value_style.bg(ACTIVE_ROW_BG);
            tf_style = tf_style.bg(ACTIVE_ROW_BG);
        }

        Row::new(vec![
            Cell::from(cell_text(
                tf_label(*tf),
                widths[0],
                tf_style,
                separator_style,
            )),
            Cell::from(cell_text(
                if enabled { "Yes" } else { "No" },
                widths[1],
                value_style,
                separator_style,
            )),
        ])
        .height(2)
    });

    let table = Table::new(
        rows,
        [Constraint::Length(widths[0]), Constraint::Length(widths[1])],
    )
    .header(header)
    .column_spacing(spacing);

    let table_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(table_width), Constraint::Min(0)])
        .split(layout[5])[0];
    frame.render_widget(table, table_area);
}

fn render_actions(frame: &mut Frame, area: Rect, app: &DashboardApp) {
    let row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(12),
            Constraint::Length(12),
        ])
        .split(area);

    let save_focus = matches!(app.focus_field(), SettingsField::Save);
    let cancel_focus = matches!(app.focus_field(), SettingsField::Cancel);

    let save_style = button_style(save_focus);
    let cancel_style = button_style(cancel_focus);

    let save = Paragraph::new("Save")
        .alignment(Alignment::Center)
        .style(save_style)
        .block(Block::default().borders(Borders::ALL));
    let cancel = Paragraph::new("Cancel")
        .alignment(Alignment::Center)
        .style(cancel_style)
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(save, row[1]);
    frame.render_widget(cancel, row[2]);
}

fn render_clone_modal(frame: &mut Frame, app: &DashboardApp, area: Rect) {
    let popup = centered_rect(60, 30, area);
    frame.render_widget(Clear, popup);

    let block = Block::default().borders(Borders::ALL).title("Clone preset");
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(inner);

    frame.render_widget(Paragraph::new("New preset name:"), layout[0]);

    let name_focus = matches!(app.focus_field(), SettingsField::CloneName);
    let filled = !app.clone_name().trim().is_empty();
    let display = if filled {
        app.clone_name().to_string()
    } else {
        "Enter name".to_string()
    };
    let input = Paragraph::new(display)
        .style(field_style(filled, name_focus))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(input, layout[1]);

    let buttons = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(10), Constraint::Length(12), Constraint::Min(1)])
        .split(layout[2]);

    let ok_focus = matches!(app.focus_field(), SettingsField::CloneConfirm);
    let cancel_focus = matches!(app.focus_field(), SettingsField::CloneCancel);

    let ok = Paragraph::new("OK")
        .alignment(Alignment::Center)
        .style(button_style(ok_focus))
        .block(Block::default().borders(Borders::ALL));
    let cancel = Paragraph::new("Cancel")
        .alignment(Alignment::Center)
        .style(button_style(cancel_focus))
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(ok, buttons[0]);
    frame.render_widget(cancel, buttons[1]);
}

fn field_style(filled: bool, focused: bool) -> Style {
    let mut style = if filled {
        Style::default().fg(FIELD_ACTIVE)
    } else {
        Style::default().fg(FIELD_INACTIVE)
    };
    if focused {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    style
}

fn button_style(focused: bool) -> Style {
    let mut style = Style::default().fg(PAIR_COLOR);
    if focused {
        style = style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
    }
    style
}

fn cell_text(
    value: impl Into<String>,
    width: u16,
    value_style: Style,
    separator_style: Style,
) -> Text<'static> {
    let label = value.into();
    let width_usize = width as usize;
    let mut text = label;
    if text.len() > width_usize {
        text.truncate(width_usize);
    }
    let padding = width_usize.saturating_sub(text.len());
    let left = padding / 2;
    let right = padding - left;
    let centered = format!("{}{}{}", " ".repeat(left), text, " ".repeat(right));
    Text::from(vec![
        Line::from(Span::styled(centered, value_style)),
        Line::from(Span::styled("-".repeat(width_usize), separator_style)),
    ])
}
