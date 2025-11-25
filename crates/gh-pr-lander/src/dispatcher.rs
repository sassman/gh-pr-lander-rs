use crate::actions::Action;
use std::sync::{Arc, Mutex};

/// Dispatcher for sending actions through the middleware chain
#[derive(Clone)]
pub struct Dispatcher {
    sender: Arc<Mutex<Vec<Action>>>,
}

impl Dispatcher {
    pub fn new() -> Self {
        Self {
            sender: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Dispatch an action to be processed
    pub fn dispatch(&self, action: Action) {
        if let Ok(mut queue) = self.sender.lock() {
            queue.push(action);
        }
    }

    /// Drain all pending actions
    pub fn drain(&self) -> Vec<Action> {
        if let Ok(mut queue) = self.sender.lock() {
            queue.drain(..).collect()
        } else {
            Vec::new()
        }
    }
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new()
    }
}
