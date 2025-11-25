use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{self, Event, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    Terminal,
};
use std::io;

mod actions;
mod dispatcher;
mod middleware;
mod reducer;
mod state;
mod store;
mod view_models;
mod views;

use actions::Action;
use middleware::{keyboard::KeyboardMiddleware, logging::LoggingMiddleware};
use state::AppState;
use store::Store;

fn main() -> io::Result<()> {
    // Initialize logger
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    log::info!("Starting gh-pr-lander");

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize store with middleware
    let mut store = Store::new(AppState::default());

    // Add middleware in order (they execute in this order)
    store.add_middleware(Box::new(LoggingMiddleware::new()));
    store.add_middleware(Box::new(KeyboardMiddleware::new()));

    // Main event loop
    let result = run_app(&mut terminal, &mut store);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {}", err);
    }

    log::info!("Exiting gh-pr-lander");
    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    store: &mut Store,
) -> io::Result<()> {
    loop {
        // Render
        terminal.draw(|frame| {
            let area = frame.area();
            views::render(store.state(), area, frame.buffer_mut());
        })?;

        // Check if we should quit
        if !store.state().running {
            break;
        }

        // Handle events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Only process key press events (ignore key release)
                if key.kind == KeyEventKind::Press {
                    store.dispatch(Action::GlobalKeyPressed(key));
                }
            }
        }
    }

    Ok(())
}
