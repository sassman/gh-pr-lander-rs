use crate::state::AppState;
use ratatui::{buffer::Buffer, layout::Rect};

pub mod main_view;

/// Render the entire application UI
pub fn render(state: &AppState, area: Rect, buf: &mut Buffer) {
    main_view::render(state, area, buf);
}
