use std::{
    io::{self, Stdout},
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, Event, KeyEvent, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{
    error::{GlobalError, Result},
    message_bus::{EngineTx, HistoryTx, UiRx, WsTx},
};

use super::{app::DashboardApp, render};

pub fn run_tui(
    engine_tx: EngineTx,
    history_tx: HistoryTx,
    ws_tx: WsTx,
    rt_handle: tokio::runtime::Handle,
    rx: UiRx,
) -> Result<()> {
    let mut terminal = init_terminal().map_err(|e| GlobalError::Other(e.to_string()))?;
    let _guard = TerminalGuard::new()?;
    let mut app = DashboardApp::new(engine_tx, history_tx, ws_tx, rt_handle, rx);
    let tick_rate = Duration::from_millis(200);
    let mut last_tick = Instant::now();

    while !app.should_quit() {
        app.poll_updates();
        terminal
            .draw(|f| render::draw(f, &mut app))
            .map_err(|e| GlobalError::Other(e.to_string()))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_millis(0));
        if event::poll(timeout).map_err(|e| GlobalError::Other(e.to_string()))? {
            if let Event::Key(key) = event::read().map_err(|e| GlobalError::Other(e.to_string()))? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key);
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }

    terminal
        .show_cursor()
        .map_err(|e| GlobalError::Other(e.to_string()))?;
    Ok(())
}

fn init_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(io::stdout()))
}

struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}
