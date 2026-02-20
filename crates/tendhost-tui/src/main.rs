//! tendhost TUI
//!
//! Terminal user interface for monitoring and controlling tendhost daemon

use std::io;
use std::time::Duration;

use clap::Parser;
use color_eyre::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod action;
mod app;
mod config;
mod event;
mod ui;

use app::App;
use event::EventHandler;

/// tendhost Terminal UI
#[derive(Parser, Debug)]
#[command(name = "tendhost-tui", version, about)]
struct Args {
    /// Server address
    #[arg(short, long, default_value = "http://localhost:8080")]
    server: String,

    /// Tick rate in milliseconds
    #[arg(long, default_value = "250")]
    tick_rate: u64,

    /// Enable debug logging to file
    #[arg(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize error handling
    color_eyre::install()?;

    // Parse arguments
    let args = Args::parse();

    // Initialize logging
    if args.debug {
        let file = std::fs::File::create("tendhost-tui.log")?;
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_writer(file))
            .init();
    }

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let tick_rate = Duration::from_millis(args.tick_rate);
    let mut app = App::new(&args.server);
    let result = run_app(&mut terminal, &mut app, tick_rate).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Handle any errors
    if let Err(err) = result {
        eprintln!("Error: {err:?}");
    }

    Ok(())
}

/// Run the application main loop
async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    tick_rate: Duration,
) -> Result<()> {
    // Create event handler
    let mut events = EventHandler::new(tick_rate);
    events.start();

    // Connect to daemon
    app.connect().await?;

    // Connection check interval
    let mut check_connection_interval = tokio::time::interval(Duration::from_secs(10));

    // Main loop
    loop {
        // Draw UI
        terminal.draw(|frame| ui::render(frame, app))?;

        // Handle events with timeout to prevent blocking
        tokio::select! {
            // Handle terminal events
            event = events.next() => {
                if let Some(event) = event {
                    let action = match event {
                        event::Event::Key(key) => event::key_to_action(key, app.search_active),
                        event::Event::Resize(_, _) => action::Action::Render,
                        event::Event::Tick => action::Action::Tick,
                    };
                    app.handle_action(action).await?;
                }
            }
            // Check connection periodically
            _ = check_connection_interval.tick() => {
                app.check_connection();
            }
        }

        // Process WebSocket events
        app.process_ws_events().await?;

        if app.should_quit() {
            break;
        }
    }

    Ok(())
}
