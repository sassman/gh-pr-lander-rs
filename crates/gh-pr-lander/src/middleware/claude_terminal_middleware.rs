//! Claude Terminal Middleware
//!
//! Manages the PTY lifecycle for the embedded terminal panel:
//! - Spawns multiplexer attach in a PTY on Open
//! - Runs a reader thread that parses output via vt100 and dispatches screen updates
//! - Forwards key input to the PTY writer
//! - Handles PTY resize (both PTY and vt100 parser)

use crate::actions::{Action, ClaudeTerminalAction};
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::claude_terminal::{popup_inner_size, PtyWriter};
use crate::state::AppState;
use gh_pr_fix_with_claude::{key_event_to_bytes, open_multiplexer_pty, TerminalScreen};
use portable_pty::PtySize;
use std::io::Read;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Packs (cols, rows) into a single u32 for atomic sharing
fn pack_size(cols: u16, rows: u16) -> u32 {
    (cols as u32) << 16 | rows as u32
}

/// Unpacks a u32 into (cols, rows)
fn unpack_size(packed: u32) -> (u16, u16) {
    ((packed >> 16) as u16, packed as u16)
}

pub struct ClaudeTerminalMiddleware {
    /// Master PTY handle for resize operations
    master: Option<Box<dyn portable_pty::MasterPty + Send>>,
    /// Shared size signal for the reader thread's vt100 parser
    shared_size: Option<Arc<AtomicU32>>,
}

impl ClaudeTerminalMiddleware {
    pub fn new() -> Self {
        Self {
            master: None,
            shared_size: None,
        }
    }

    fn handle_open(&mut self, session_name: &str, state: &AppState, dispatcher: &Dispatcher) {
        // Compute correct initial size from terminal area
        let (term_w, term_h) = state.claude_terminal.terminal_area;
        let (cols, rows) = if term_w > 0 && term_h > 0 {
            popup_inner_size(term_w, term_h)
        } else {
            (120, 40) // Fallback if terminal area not yet known
        };

        match open_multiplexer_pty(
            &state.app_config.fix_with_claude_code.multiplexer,
            session_name,
            cols,
            rows,
        ) {
            Ok(pty) => {
                // Store master for resize
                self.master = Some(pty.master);

                // Create shared size signal for reader thread
                let shared_size = Arc::new(AtomicU32::new(pack_size(cols, rows)));
                self.shared_size = Some(shared_size.clone());

                // Create shared writer
                let writer: PtyWriter = Arc::new(Mutex::new(pty.writer));
                dispatcher.dispatch(Action::ClaudeTerminal(ClaudeTerminalAction::SetWriter(
                    writer,
                )));

                // Spawn reader thread with shared size for parser resize
                spawn_reader(pty.reader, cols, rows, shared_size, dispatcher.clone());
            }
            Err(err) => {
                log::error!("Failed to open PTY for terminal: {}", err);
                dispatcher.dispatch(Action::ClaudeTerminal(ClaudeTerminalAction::PtyExited));
            }
        }
    }

    fn handle_key_input(&self, key: &ratatui::crossterm::event::KeyEvent, state: &AppState) {
        if let Some(ref writer) = state.claude_terminal.pty_writer {
            let bytes = key_event_to_bytes(key);
            if !bytes.is_empty() {
                if let Ok(mut w) = writer.lock() {
                    use std::io::Write;
                    let _ = w.write_all(&bytes);
                    let _ = w.flush();
                }
            }
        }
    }

    fn handle_resize(&mut self, cols: u16, rows: u16) {
        // Resize the PTY
        if let Some(ref master) = self.master {
            let size = PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            };
            if let Err(e) = master.resize(size) {
                log::warn!("Failed to resize PTY: {}", e);
            }
        }

        // Signal the reader thread to resize the vt100 parser
        if let Some(ref shared_size) = self.shared_size {
            shared_size.store(pack_size(cols, rows), Ordering::Relaxed);
        }
    }
}

impl Middleware for ClaudeTerminalMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        if let Action::ClaudeTerminal(sub) = action {
            match sub {
                ClaudeTerminalAction::Open { session_name } => {
                    self.handle_open(session_name, state, dispatcher);
                    true // Pass through to reducer (pushes view)
                }
                ClaudeTerminalAction::KeyInput(key) => {
                    self.handle_key_input(key, state);
                    false // Consumed - no reducer action needed
                }
                ClaudeTerminalAction::Resize { cols, rows } => {
                    self.handle_resize(*cols, *rows);
                    true // Pass through to reducer (updates last_size)
                }
                ClaudeTerminalAction::PtyExited => {
                    self.master = None;
                    self.shared_size = None;
                    true // Pass through to reducer (pops view)
                }
                // ScreenUpdated and SetWriter pass through to reducer
                _ => true,
            }
        } else {
            true // Pass through all other actions
        }
    }
}

/// Spawn a background thread that reads PTY output, parses via vt100, and dispatches screen updates.
/// The shared_size atomic allows the middleware to signal parser resize.
fn spawn_reader(
    mut reader: Box<dyn Read + Send>,
    cols: u16,
    rows: u16,
    shared_size: Arc<AtomicU32>,
    dispatcher: Dispatcher,
) {
    std::thread::spawn(move || {
        let mut parser = vt100::Parser::new(rows, cols, 0);
        let mut current_size = pack_size(cols, rows);
        let mut buf = [0u8; 4096];
        let mut last_dispatch = Instant::now();

        loop {
            // Check if parser needs resize
            let new_size = shared_size.load(Ordering::Relaxed);
            if new_size != current_size {
                let (new_cols, new_rows) = unpack_size(new_size);
                parser.set_size(new_rows, new_cols);
                current_size = new_size;
            }

            match reader.read(&mut buf) {
                Ok(0) | Err(_) => {
                    dispatcher.dispatch(Action::ClaudeTerminal(ClaudeTerminalAction::PtyExited));
                    break;
                }
                Ok(n) => {
                    parser.process(&buf[..n]);
                    // Throttle: dispatch at most every 16ms (~60fps)
                    if last_dispatch.elapsed() >= Duration::from_millis(16) {
                        let screen = TerminalScreen::from_vt100(parser.screen());
                        dispatcher.dispatch(Action::ClaudeTerminal(
                            ClaudeTerminalAction::ScreenUpdated(screen),
                        ));
                        last_dispatch = Instant::now();
                    }
                }
            }
        }
    });
}
