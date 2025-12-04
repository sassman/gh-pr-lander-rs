//! Bootstrap Middleware
//!
//! Manages application startup sequence:
//! - Starts tick thread for animations on BootstrapStart
//! - Dispatches LoadRecentRepositories to trigger repository loading
//! - Listens for LoadRecentRepositoriesDone to dispatch BootstrapEnd
//! - Stops tick thread on BootstrapEnd

use crate::actions::{Action, BootstrapAction, GlobalAction};
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;
use crate::views::PullRequestView;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Bootstrap middleware - manages application startup and tick generation
pub struct BootstrapMiddleware {
    tick_thread_started: Arc<Mutex<bool>>,
}

impl BootstrapMiddleware {
    pub fn new() -> Self {
        Self {
            tick_thread_started: Arc::new(Mutex::new(false)),
        }
    }
}

impl Default for BootstrapMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for BootstrapMiddleware {
    fn handle(&mut self, action: &Action, _state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            Action::Bootstrap(BootstrapAction::Start) => {
                log::info!("BootstrapMiddleware: Bootstrap starting");

                // Start tick thread if not already started
                let mut started = self.tick_thread_started.lock().unwrap();
                if !*started {
                    *started = true;

                    let dispatcher_clone = dispatcher.clone();
                    let should_continue = self.tick_thread_started.clone();

                    // Spawn tick generation thread
                    thread::spawn(move || {
                        let tick_rate = Duration::from_millis(200);
                        let mut last_tick = Instant::now();

                        loop {
                            if !*should_continue.lock().unwrap() {
                                log::debug!("Bootstrap: Tick thread terminating");
                                break;
                            }
                            // Wait for next tick
                            let now = Instant::now();
                            let elapsed = now.duration_since(last_tick);

                            if elapsed >= tick_rate {
                                dispatcher_clone.dispatch(Action::Global(GlobalAction::Tick));
                                last_tick = now;
                            } else {
                                // Sleep for the remaining time
                                thread::sleep(tick_rate - elapsed);
                            }
                        }
                    });

                    log::debug!("Bootstrap: Tick thread started");
                }

                // NOTE: Repository loading is now triggered by ClientReady from github_middleware
                // after the async client initialization completes

                // Pass through
                true
            }

            Action::Bootstrap(BootstrapAction::LoadRecentRepositoriesDone) => {
                log::info!("BootstrapMiddleware: Repository loading done, ending bootstrap");
                dispatcher.dispatch(Action::Bootstrap(BootstrapAction::End));
                dispatcher.dispatch(Action::Global(GlobalAction::ReplaceView(Box::new(
                    PullRequestView::new(),
                ))));
                true
            }

            Action::Bootstrap(BootstrapAction::End) => {
                // Stop the tick thread
                let mut started = self.tick_thread_started.lock().unwrap();
                *started = false;
                log::info!("BootstrapMiddleware: Bootstrap ended, stopping tick thread");

                // Pass through
                true
            }

            _ => {
                // All other actions pass through
                true
            }
        }
    }
}
