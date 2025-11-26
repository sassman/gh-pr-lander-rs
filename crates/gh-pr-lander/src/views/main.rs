use crate::capabilities::PanelCapabilities;
use crate::state::AppState;
use crate::views::View;
use crate::views::{debug_console_view, main_view};
use ratatui::{layout::Rect, Frame};

/// Main application view
#[derive(Debug, Clone)]
pub struct MainView;

impl MainView {
    pub fn new() -> Self {
        Self
    }
}

impl View for MainView {
    fn view_id(&self) -> crate::views::ViewId {
        crate::views::ViewId::Main
    }

    fn render(&self, state: &AppState, area: Rect, f: &mut Frame) {
        // Render the main view content
        main_view::render(state, area, f);

        // Render debug console on top if visible
        debug_console_view::render(&state.debug_console, &state.theme, area, f);
    }

    fn capabilities(&self, _state: &AppState) -> PanelCapabilities {
        // Main view supports vim navigation
        PanelCapabilities::VIM_NAVIGATION_BINDINGS
    }

    fn clone_box(&self) -> Box<dyn View> {
        Box::new(self.clone())
    }
}
