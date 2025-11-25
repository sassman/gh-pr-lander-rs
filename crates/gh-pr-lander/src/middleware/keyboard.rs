use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// KeyboardMiddleware - converts raw keyboard events to semantic actions
pub struct KeyboardMiddleware;

impl KeyboardMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Middleware for KeyboardMiddleware {
    fn handle(&mut self, action: &Action, _state: &AppState, dispatcher: &Dispatcher) -> bool {
        if let Action::GlobalKeyPressed(key) = action {
            handle_key_event(key, dispatcher);
            // Consume the raw key event (don't pass to reducer)
            return false;
        }

        // Pass all other actions through
        true
    }
}

/// Handle a key event and dispatch semantic actions
fn handle_key_event(key: &KeyEvent, dispatcher: &Dispatcher) {
    match key.code {
        // Global close/quit
        KeyCode::Char('q') if key.modifiers == KeyModifiers::NONE => {
            dispatcher.dispatch(Action::GlobalClose);
        }
        KeyCode::Esc => {
            dispatcher.dispatch(Action::GlobalClose);
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            dispatcher.dispatch(Action::GlobalQuit);
        }

        // Vim navigation - down/next
        KeyCode::Char('j') if key.modifiers == KeyModifiers::NONE => {
            dispatcher.dispatch(Action::NavNext);
        }
        KeyCode::Down => {
            dispatcher.dispatch(Action::NavNext);
        }

        // Vim navigation - up/previous
        KeyCode::Char('k') if key.modifiers == KeyModifiers::NONE => {
            dispatcher.dispatch(Action::NavPrevious);
        }
        KeyCode::Up => {
            dispatcher.dispatch(Action::NavPrevious);
        }

        // Vim navigation - left
        KeyCode::Char('h') if key.modifiers == KeyModifiers::NONE => {
            dispatcher.dispatch(Action::NavLeft);
        }
        KeyCode::Left => {
            dispatcher.dispatch(Action::NavLeft);
        }

        // Vim navigation - right
        KeyCode::Char('l') if key.modifiers == KeyModifiers::NONE => {
            dispatcher.dispatch(Action::NavRight);
        }
        KeyCode::Right => {
            dispatcher.dispatch(Action::NavRight);
        }

        // Unhandled keys
        _ => {
            log::trace!("Unhandled key: {:?}", key);
        }
    }
}
