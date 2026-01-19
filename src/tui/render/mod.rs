mod dashboard;
mod intro;
mod layout;
mod settings;

use ratatui::layout::{Constraint, Direction, Layout};

use super::{app::DashboardApp, types::ViewMode};

pub fn draw(frame: &mut ratatui::Frame, app: &mut DashboardApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(frame.area());

    dashboard::render_header(frame, chunks[0], app);
    dashboard::render_dashboard(frame, chunks[1], app);
    dashboard::render_footer(frame, chunks[2]);

    if app.view() == ViewMode::Settings {
        settings::render_settings(frame, app);
    } else if app.view() == ViewMode::Layout {
        layout::render_layout(frame, app);
    }
}
