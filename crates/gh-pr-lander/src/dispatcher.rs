use crate::actions::Action;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Dispatcher for sending actions through the middleware chain
#[derive(Clone)]
pub struct Dispatcher {
    queue: Arc<Mutex<VecDeque<Action>>>,
}

impl Dispatcher {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Dispatch an action to be processed
    pub fn dispatch(&self, action: Action) {
        if let Ok(mut queue) = self.queue.lock() {
            queue.push_back(action);
        }
    }

    /// Pop a single action from the queue (FIFO) - O(1)
    pub fn pop(&self) -> Option<Action> {
        if let Ok(mut queue) = self.queue.lock() {
            queue.pop_front()
        } else {
            None
        }
    }
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new()
    }
}
