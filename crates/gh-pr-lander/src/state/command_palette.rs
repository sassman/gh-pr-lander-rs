//! Command Palette State

/// Command palette state.
///
/// Only domain state lives here. The visible window (`offset`) is derived in
/// the view model each frame from `selected_index` plus the actual rendered
/// area, so the view never needs to broadcast its layout back into state.
#[derive(Debug, Clone, Default)]
pub struct CommandPaletteState {
    pub query: String,
    pub selected_index: usize,
}
