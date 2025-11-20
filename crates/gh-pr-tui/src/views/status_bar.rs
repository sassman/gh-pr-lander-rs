use ratatui::{prelude::*, widgets::*};

use crate::App;
use crate::state::TaskStatusType;

/// Render the status bar showing background task progress
pub fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    if let Some(ref status) = app.store.state().task.status {
        let (icon, color) = match status.status_type {
            TaskStatusType::Running => ("⏳", app.store.state().theme.status_warning),
            TaskStatusType::Success => ("✓", app.store.state().theme.status_success),
            TaskStatusType::Error => ("✗", app.store.state().theme.status_error),
            TaskStatusType::Warning => ("⚠", app.store.state().theme.status_warning),
        };

        let status_text = format!(" {} {}", icon, status.message);
        let status_span = Span::styled(
            status_text,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        );

        let paragraph = Paragraph::new(Line::from(status_span))
            .style(Style::default().bg(app.store.state().repos.colors.buffer_bg));
        f.render_widget(paragraph, area);
    }
}
