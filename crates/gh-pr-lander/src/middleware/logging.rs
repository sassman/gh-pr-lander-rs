use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;

/// LoggingMiddleware - logs all actions passing through
pub struct LoggingMiddleware;

impl LoggingMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Middleware for LoggingMiddleware {
    fn handle(&mut self, action: &Action, _state: &AppState, _dispatcher: &Dispatcher) -> bool {
        log::debug!("Action: {:?}", action);
        true // Always pass action through
    }
}
