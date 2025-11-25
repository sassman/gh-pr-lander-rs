use crate::state::AppState;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

/// Render the main view
pub fn render(_state: &AppState, area: Rect, buf: &mut Buffer) {
    let block = Block::default()
        .title(" gh-pr-lander ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Welcome to gh-pr-lander",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("A clean, minimal PR landing tool"),
        Line::from(""),
        Line::from("Controls:"),
        Line::from("  j/k or ↓/↑  - Navigate"),
        Line::from("  h/l or ←/→  - Navigate left/right"),
        Line::from("  q or Esc    - Close/Quit"),
        Line::from("  Ctrl+C      - Force quit"),
    ];

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);

    paragraph.render(area, buf);
}
