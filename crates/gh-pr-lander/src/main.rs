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
use std::time::{Duration, Instant};

mod actions;
mod capabilities;
mod command_id;
mod commands;
mod dispatcher;
mod domain_models;
mod keybindings;
mod keymap;
mod log_reader;
mod logger;
mod middleware;
mod reducers;
mod state;
mod store;
mod utils;
mod view_models;
mod views;

use actions::{Action, BootstrapAction, GlobalAction};
use middleware::{
    app_config_middleware::AppConfigMiddleware, bootstrap_middleware::BootstrapMiddleware,
    command_palette_middleware::CommandPaletteMiddleware,
    confirmation_popup_middleware::ConfirmationPopupMiddleware,
    debug_console_middleware::DebugConsoleMiddleware, github_middleware::GitHubMiddleware,
    keyboard_middleware::KeyboardMiddleware, navigation_middleware::NavigationMiddleware,
    pull_request_middleware::PullRequestMiddleware, repository_middleware::RepositoryMiddleware,
    text_input_middleware::TextInputMiddleware,
};
use state::AppState;
use store::Store;

fn main() -> io::Result<()> {
    // Initialize file-based logger (returns log file path for debug console)
    let log_file = logger::init();

    log::info!("Starting GitHub PR Lander");

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize store with middleware
    let mut store = Store::new(AppState::default());

    // Add middleware in order (they execute in this order)
    store.add_middleware(Box::new(BootstrapMiddleware::new()));
    store.add_middleware(Box::new(AppConfigMiddleware::new())); // Load app config early
    store.add_middleware(Box::new(GitHubMiddleware::new())); // GitHub client & API operations
    store.add_middleware(Box::new(KeyboardMiddleware::new()));
    // Translation middlewares - convert generic actions to view-specific actions
    store.add_middleware(Box::new(NavigationMiddleware::new()));
    store.add_middleware(Box::new(TextInputMiddleware::new()));
    // View-specific middlewares
    store.add_middleware(Box::new(CommandPaletteMiddleware::new()));
    store.add_middleware(Box::new(ConfirmationPopupMiddleware::new()));
    store.add_middleware(Box::new(RepositoryMiddleware::new()));
    store.add_middleware(Box::new(PullRequestMiddleware::new())); // Bulk loading coordination
    store.add_middleware(Box::new(DebugConsoleMiddleware::new(log_file))); // Debug console log reader

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

/// Maximum time budget for processing actions before rendering
/// This ensures smooth animations even when many actions are queued
const ACTION_BUDGET: Duration = Duration::from_millis(16); // ~60fps frame budget

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    store: &mut Store,
) -> io::Result<()> {
    // Queue bootstrap to be processed by the main loop (not synchronously)
    // This ensures renders happen during bootstrap, not just after
    store
        .dispatcher()
        .dispatch(Action::Bootstrap(BootstrapAction::Start));

    loop {
        // Process pending actions with a time budget to avoid blocking renders
        let start = Instant::now();

        while let Some(action) = store.dispatcher().pop() {
            store.dispatch(action);

            // Check budget after each action - remaining actions stay in queue
            if start.elapsed() >= ACTION_BUDGET {
                break;
            }
        }

        // Render
        let mut terminal_height = 0u16;
        terminal.draw(|frame| {
            let area = frame.area();
            terminal_height = area.height;
            views::render(store.state(), area, frame);
        })?;

        // Dirty hack to fix the scrolling behaviour of the debug console
        // Update debug console visible height based on terminal size
        // (70% of screen height minus 2 for borders)
        let debug_console_height = ((terminal_height as usize) * 70 / 100).saturating_sub(2);
        if store.state().debug_console.visible_height != debug_console_height {
            store.dispatch(Action::DebugConsole(
                crate::actions::DebugConsoleAction::SetVisibleHeight(debug_console_height),
            ));
        }

        // Update diff viewer viewport height based on terminal size
        // (full height minus 3 for status bar and borders)
        let terminal_width = terminal.size()?.width;
        let diff_viewport_height = terminal_height.saturating_sub(3) as usize;
        if let Some(ref inner) = store.state().diff_viewer.inner {
            if inner.viewport_height != diff_viewport_height {
                store.dispatch(Action::DiffViewer(
                    crate::actions::DiffViewerAction::SetViewport {
                        width: terminal_width,
                        height: diff_viewport_height as u16,
                    },
                ));
            }
        }

        // Check if we should quit
        if !store.state().running {
            break;
        }

        // Handle events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Only process key press events (ignore key release)
                if key.kind == KeyEventKind::Press {
                    store.dispatch(Action::Global(GlobalAction::KeyPressed(key)));
                }
            }
        }
    }

    Ok(())
}
