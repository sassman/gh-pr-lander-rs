use crate::capabilities::{PanelCapabilities, PanelCapabilityProvider};
use crate::state::AppState;
use crate::views::debug_console_view;
use crate::views::View;
use ratatui::{layout::Rect, Frame};

/// Debug console view - shows logs overlaid on main view
#[derive(Debug, Clone)]
pub struct DebugConsoleView;

impl DebugConsoleView {
    pub fn new() -> Self {
        Self
    }
}

impl View for DebugConsoleView {
    fn view_id(&self) -> crate::views::ViewId {
        crate::views::ViewId::DebugConsole
    }

    fn render(&self, state: &AppState, area: Rect, f: &mut Frame) {
        // Render the main view in the background
        // main_view::render(state, area, f);

        // Render debug console on top
        debug_console_view::render(&state.debug_console, &state.theme, area, f);
    }

    fn capabilities(&self, state: &AppState) -> PanelCapabilities {
        // Debug console has its own capabilities
        state.debug_console.capabilities()
    }

    fn clone_box(&self) -> Box<dyn View> {
        Box::new(self.clone())
    }
}
