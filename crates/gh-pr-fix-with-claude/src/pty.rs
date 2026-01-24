//! PTY spawn helper
//!
//! Opens a pseudo-terminal and spawns `tmux attach-session` inside it.

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};

/// Handle to an open PTY with reader/writer/master
pub struct EmbeddedPty {
    pub reader: Box<dyn Read + Send>,
    pub writer: Box<dyn Write + Send>,
    pub master: Box<dyn portable_pty::MasterPty + Send>,
}

/// Open a PTY and spawn `tmux attach-session -t <session_name>` inside it.
pub fn open_tmux_pty(session_name: &str, cols: u16, rows: u16) -> Result<EmbeddedPty, String> {
    let pty_system = native_pty_system();

    let size = PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = pty_system
        .openpty(size)
        .map_err(|e| format!("Failed to open PTY: {}", e))?;

    let mut cmd = CommandBuilder::new("tmux");
    cmd.args(["attach-session", "-t", session_name]);

    pair.slave
        .spawn_command(cmd)
        .map_err(|e| format!("Failed to spawn tmux: {}", e))?;

    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("Failed to clone PTY reader: {}", e))?;

    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("Failed to take PTY writer: {}", e))?;

    Ok(EmbeddedPty {
        reader,
        writer,
        master: pair.master,
    })
}
