//! PTY spawn helper
//!
//! Opens a pseudo-terminal and spawns a multiplexer attach inside it.

use crate::config::Multiplexer;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{Read, Write};

/// Handle to an open PTY with reader/writer/master
pub struct EmbeddedPty {
    pub reader: Box<dyn Read + Send>,
    pub writer: Box<dyn Write + Send>,
    pub master: Box<dyn portable_pty::MasterPty + Send>,
}

/// Open a PTY and spawn a multiplexer attach inside it.
pub fn open_multiplexer_pty(
    multiplexer: &Multiplexer,
    session_name: &str,
    cols: u16,
    rows: u16,
) -> Result<EmbeddedPty, String> {
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

    let cmd = match multiplexer {
        Multiplexer::Tmux => {
            let mut cmd = CommandBuilder::new("tmux");
            cmd.args(["attach-session", "-t", session_name]);
            cmd
        }
        Multiplexer::Zellij => {
            let mut cmd = CommandBuilder::new("zellij");
            cmd.args(["attach", session_name]);
            cmd
        }
    };

    let mux_name = match multiplexer {
        Multiplexer::Tmux => "tmux",
        Multiplexer::Zellij => "zellij",
    };

    pair.slave
        .spawn_command(cmd)
        .map_err(|e| format!("Failed to spawn {mux_name}: {}", e))?;

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
