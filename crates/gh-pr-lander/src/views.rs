use crate::capabilities::PanelCapabilities;
use crate::state::AppState;
use ratatui::{layout::Rect, Frame};

// New view modules (concrete view types)
pub mod debug_console;
pub mod main;
pub mod splash;

// Re-export concrete view types for convenience
pub use debug_console::DebugConsoleView;
pub use main::MainView;
pub use splash::SplashView;

/// View identifier - allows comparing which view is active
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewId {
    Splash,
    Main,
    DebugConsole,
}

/// View trait - defines the interface that all views must implement
///
/// This allows the application to interact with views polymorphically through
/// trait objects (Box<dyn View>).
///
/// IMPORTANT: This trait must be object-safe to be used as a trait object.
/// That means:
/// - No generic methods
/// - No Self: Sized bounds
/// - All methods must use &self (not consume self)
/// - Must be Send for thread safety (actions are sent between threads)
pub trait View: std::fmt::Debug + Send {
    /// Get the unique identifier for this view type
    fn view_id(&self) -> ViewId;

    /// Render this view
    fn render(&self, state: &AppState, area: Rect, f: &mut Frame);

    /// Get the capabilities of this view (for keyboard handling)
    fn capabilities(&self, state: &AppState) -> PanelCapabilities;

    /// Check if this view is a floating view (renders on top of other views)
    /// Default implementation returns false (non-floating)
    fn is_floating(&self) -> bool {
        false
    }

    /// Clone this view into a Box
    /// This is needed because Clone requires Sized, so we provide a manual clone method
    fn clone_box(&self) -> Box<dyn View>;
}

/// Implement Clone for Box<dyn View>
impl Clone for Box<dyn View> {
    fn clone(&self) -> Box<dyn View> {
        self.clone_box()
    }
}

/// Render the entire application UI
///
/// Optimized rendering strategy:
/// - If the top view is non-floating, only render that view (it covers everything)
/// - If the top view is floating, render the base view below it, then the floating view on top
///   (floating views use `Clear` widget to preserve unrendered areas)
pub fn render(state: &AppState, area: Rect, f: &mut Frame) {
    let stack_len = state.view_stack.len();

    if stack_len == 0 {
        return; // Should never happen, but guard against it
    }

    // Get the top-most view
    let top_view = &state.view_stack[stack_len - 1];

    if top_view.is_floating() {
        // Floating view on top - render the view below it first (if any)
        if stack_len > 1 {
            let base_view = &state.view_stack[stack_len - 2];
            base_view.render(state, area, f);
        }
        // Then render the floating view on top
        top_view.render(state, area, f);
    } else {
        // Non-floating view - only render the top view (it covers everything)
        top_view.render(state, area, f);
    }
}
