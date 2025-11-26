use crate::capabilities::{PanelCapabilities, PanelCapabilityProvider};
use crate::state::{ActiveView, AppState};
use ratatui::{layout::Rect, Frame};

pub mod debug_console_view;
pub mod main_view;
pub mod splash_view;

pub trait StatefulView: std::fmt::Debug + Clone {
    type State;
    fn render(self, state: &Self::State, area: Rect, f: &mut Frame);
}

/// View trait - defines the interface that all views must implement
///
/// This allows the application to interact with views polymorphically without
/// knowing which specific view is active.
pub trait View {
    /// Render this view
    fn render(&self, state: &AppState, area: Rect, f: &mut Frame);

    /// Get the capabilities of this view (for keyboard handling)
    fn capabilities(&self, state: &AppState) -> PanelCapabilities;
}

/// Implement View trait for ActiveView enum
///
/// This allows ActiveView to be used polymorphically while maintaining the
/// type safety and zero-cost abstraction of an enum.
impl View for ActiveView {
    fn render(&self, state: &AppState, area: Rect, f: &mut Frame) {
        match self {
            ActiveView::Splash => {
                splash_view::render(&state.splash, &state.theme, area, f);
            }
            ActiveView::Main => {
                main_view::render(state, area, f);
                // Render debug console on top if visible
                debug_console_view::render(&state.debug_console, &state.theme, area, f);
            }
            ActiveView::DebugConsole => {
                main_view::render(state, area, f);
                // Render debug console on top if visible
                debug_console_view::render(&state.debug_console, &state.theme, area, f);
            }
        }
    }

    fn capabilities(&self, state: &AppState) -> PanelCapabilities {
        match self {
            ActiveView::Splash => {
                // Splash screen has no interactive capabilities
                PanelCapabilities::empty()
            }
            ActiveView::Main => {
                // Main view supports vim navigation
                PanelCapabilities::VIM_NAVIGATION_BINDINGS
            }
            ActiveView::DebugConsole => {
                // Debug console has its own capabilities
                state.debug_console.capabilities()
            }
        }
    }
}

/// Render the entire application UI
///
/// This is now just a simple delegation to the active view's render method.
pub fn render(state: &AppState, area: Rect, f: &mut Frame) {
    state.active_view.render(state, area, f);
}
