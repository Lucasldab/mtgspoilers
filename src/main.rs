mod api;
mod app;
mod config;
mod db;
mod fetcher;
mod filter;
mod models;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tracing::{info, error};

use crate::app::App;
use crate::db::Database;
use crate::fetcher::Fetcher;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging
    tracing_subscriber::fmt::init();

    info!("Starting MTG Spoiler TUI");

    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize database
    let db = Database::new("sqlite:mtg_spoilers.db").await?;

    // Create on-demand fetcher (no background task)
    let fetcher_db = Database::new("sqlite:mtg_spoilers.db").await?;
    let fetcher = Fetcher::new(fetcher_db).await?;

    // Create app — fetch_once() runs inside App::new on startup
    let mut app = App::new(db, fetcher).await?;

    // Main loop
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        error!("Error: {:?}", err);
        println!("Error: {:?}", err);
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    let mut last_tick = tokio::time::Instant::now();
    let tick_rate = tokio::time::Duration::from_millis(250);

    loop {
        // Draw UI
        terminal.draw(|f| ui::draw(f, app))?;

        // Handle input with timeout
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| tokio::time::Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                if key.kind == crossterm::event::KeyEventKind::Press {
                    app.on_key(key.code);
                }
            }
        }

        // Background tick for async updates
        if last_tick.elapsed() >= tick_rate {
            app.tick().await?;
            last_tick = tokio::time::Instant::now();
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
