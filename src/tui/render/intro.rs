use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::super::util::{FIELD_ACTIVE, FIELD_INACTIVE, HEADER_COLOR, centered_rect};

pub fn render_intro(frame: &mut Frame, area: Rect) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let intro_area = if area.width < 50 || area.height < 14 {
        area
    } else {
        centered_rect(80, 80, area)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(FIELD_INACTIVE))
        .title("Welcome");
    let inner = block.inner(intro_area);
    frame.render_widget(block, intro_area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    render_tile_title(frame, sections[0]);

    let divider = Paragraph::new(Text::from(Line::from(Span::styled(
        "┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈",
        Style::default().fg(Color::DarkGray),
    ))))
    .alignment(Alignment::Center);
    frame.render_widget(divider, sections[1]);

    let body = Paragraph::new(Text::from(body_lines()))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(body, sections[2]);
}

fn render_tile_title(frame: &mut Frame, area: Rect) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let main_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let main_lines = tile_title_lines();
    let max_width = main_lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as u16;
    let height = main_lines.len() as u16;
    let width = max_width.min(area.width);
    let height = height.min(area.height);
    let title_area = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    };
    let lines = main_lines
        .into_iter()
        .map(|line| Line::from(Span::raw(line)))
        .collect::<Vec<_>>();
    let main = Paragraph::new(Text::from(lines))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false })
        .style(main_style);
    frame.render_widget(main, title_area);
}

fn tile_title_lines() -> Vec<String> {
    let the = [
        "██████  █    █  ██████",
        "  ██    █    █  ██    ",
        "  ██    ██████  █████ ",
        "  ██    █    █  ██    ",
        "  ██    █    █  ██████",
    ];

    let grid = [
        "██████  █████   ██  █████ ",
        "██      ██  ██  ██  ██  ██",
        "██  ██  █████   ██  ██  ██",
        "██  ██  ██  ██  ██  ██  ██",
        "██████  ██  ██  ██  █████ ",
    ];

    let grid_width = grid
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);

    let mut raw_lines = Vec::new();
    for line in the {
        raw_lines.push(center_within(line, grid_width));
    }
    raw_lines.push(String::new());
    raw_lines.extend(grid.iter().map(|line| line.to_string()));
    raw_lines.push(String::new());

    raw_lines
}

fn center_within(line: &str, width: usize) -> String {
    let len = line.chars().count();
    if len >= width {
        return line.to_string();
    }
    let pad_left = (width - len) / 2;
    let pad_right = width - len - pad_left;
    let mut out = String::with_capacity(width);
    out.push_str(&" ".repeat(pad_left));
    out.push_str(line);
    out.push_str(&" ".repeat(pad_right));
    out
}

fn body_lines() -> Vec<Line<'static>> {
    let youtube_short = "youtube.com/playlist?list=PLWwN_JDNbVc4-yWSJZ4lKr0os8rAfEaAH";
    let github_short = "github.com/karthik947/the_grid";
    vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "Version 0.2.1",
            Style::default().fg(FIELD_INACTIVE),
        )),
        Line::from(""),
        Line::from(""),
        Line::from("A realtime multi-timeframe multi-indicator crypto dashboard."),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "YouTube Playlist",
            Style::default()
                .fg(FIELD_ACTIVE)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            youtube_short,
            Style::default().fg(FIELD_ACTIVE),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "GitHub Repository",
            Style::default()
                .fg(FIELD_ACTIVE)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            github_short,
            Style::default().fg(FIELD_ACTIVE),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Go to the settings panel and activate a preset to get started.",
            Style::default().fg(FIELD_INACTIVE),
        )),
    ]
}
