//! Debug Console screen actions
//!
//! Actions specific to the debug console overlay.

use crate::logger::OwnedLogRecord;

/// Actions for the Debug Console screen
#[derive(Debug, Clone)]
pub enum DebugConsoleAction {
    // Navigation (translated from NavigationAction)
    /// Scroll to next log entry
    NavigateNext,
    /// Scroll to previous log entry
    NavigatePrevious,
    /// Scroll to top (oldest logs)
    NavigateToTop,
    /// Scroll to bottom (newest logs)
    NavigateToBottom,

    // Specific actions
    /// Clear all logs
    Clear,
    /// New log record added
    LogAdded(OwnedLogRecord),
    /// Dump logs to file
    DumpLogs,
    /// Update visible height (for proper scroll bounds)
    SetVisibleHeight(usize),
}
