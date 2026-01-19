use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::super::{
    app::DashboardApp,
    types::LayoutField,
    util::{FIELD_ACTIVE, FIELD_INACTIVE},
};

pub fn render_layout(frame: &mut Frame, app: &DashboardApp) {
    let dashboard_area = dashboard_inner_rect(frame.area());
    let area = bottom_right_rect(54, 15, dashboard_area);
    frame.render_widget(Clear, area);

    let block = Block::default().borders(Borders::ALL).title("Layout");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(inner);

    render_number_field(
        frame,
        sections[1],
        "Column spacing (cols) [0-10]",
        app.settings_draft().layout_column_spacing,
        app.layout_focus_field() == LayoutField::ColumnSpacing,
    );
    render_number_field(
        frame,
        sections[3],
        "Tables [1-4]",
        app.settings_draft().layout_table_count,
        app.layout_focus_field() == LayoutField::TableCount,
    );
    render_number_field(
        frame,
        sections[5],
        "Table spacing (cols) [0-10]",
        app.settings_draft().layout_table_spacing,
        app.layout_focus_field() == LayoutField::TableSpacing,
    );
}

fn render_number_field(frame: &mut Frame, area: Rect, label: &str, value: u16, focused: bool) {
    let style = field_style(true, focused);
    let input = Paragraph::new(value.to_string())
        .alignment(Alignment::Center)
        .style(style)
        .block(Block::default().borders(Borders::ALL).title(label));
    frame.render_widget(input, area);
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

fn dashboard_inner_rect(area: Rect) -> Rect {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(area);
    inset_rect(chunks[1], 1)
}

fn bottom_right_rect(width: u16, height: u16, area: Rect) -> Rect {
    let clamped_width = width.min(area.width);
    let clamped_height = height.min(area.height);
    let x = area.x + area.width.saturating_sub(clamped_width);
    let y = area.y + area.height.saturating_sub(clamped_height);
    Rect {
        x,
        y,
        width: clamped_width,
        height: clamped_height,
    }
}

fn inset_rect(area: Rect, margin: u16) -> Rect {
    let x = area.x.saturating_add(margin);
    let y = area.y.saturating_add(margin);
    let width = area.width.saturating_sub(margin.saturating_mul(2));
    let height = area.height.saturating_sub(margin.saturating_mul(2));
    Rect {
        x,
        y,
        width,
        height,
    }
}
