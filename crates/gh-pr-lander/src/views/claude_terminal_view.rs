//! Claude Terminal View
//!
//! Renders the embedded terminal as a popup panel overlay.

use crate::actions::{Action, AvailableAction};
use crate::capabilities::PanelCapabilities;
use crate::command_id::CommandId;
use crate::state::AppState;
use crate::views::{View, ViewId};
use gh_pr_fix_with_claude::{TerminalColor, TerminalScreen};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear},
    Frame,
};

/// Claude terminal view - renders PTY output as a popup overlay
#[derive(Debug, Clone)]
pub struct ClaudeTerminalView;

impl ClaudeTerminalView {
    pub fn new() -> Self {
        Self
    }
}

impl View for ClaudeTerminalView {
    fn view_id(&self) -> ViewId {
        ViewId::ClaudeTerminal
    }

    fn render(&self, state: &AppState, area: Rect, f: &mut Frame) {
        let theme = &state.theme;

        // Dim overlay for modal effect
        let overlay = Block::default().style(
            Style::default()
                .bg(Color::Black)
                .add_modifier(Modifier::DIM),
        );
        f.render_widget(overlay, area);

        // Calculate popup area (80% width, 80% height)
        let popup_width = (area.width * 80 / 100).max(40);
        let popup_height = (area.height * 80 / 100).max(10);
        let popup_x = (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect {
            x: area.x + popup_x,
            y: area.y + popup_y,
            width: popup_width,
            height: popup_height,
        };

        // Clear popup area
        f.render_widget(Clear, popup_area);
        f.render_widget(
            Block::default().style(Style::default().bg(theme.bg_panel)),
            popup_area,
        );

        // Build title
        let title = match &state.claude_terminal.session_name {
            Some(name) => format!(" Claude: {} ", name),
            None => " Claude Terminal ".to_string(),
        };

        // Footer hint
        let footer = Line::from(vec![
            Span::styled(" Esc ", Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD)),
            Span::styled("return to PR list ", Style::default().fg(theme.text_muted)),
        ]);

        // Render bordered block
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            )
            .title_bottom(footer)
            .border_style(Style::default().fg(theme.accent_primary))
            .style(Style::default().bg(theme.bg_panel));

        let inner = block.inner(popup_area);
        f.render_widget(block, popup_area);

        // Render terminal content
        if let Some(ref screen) = state.claude_terminal.screen {
            render_terminal_cells(f, screen, inner);
        } else {
            // Show waiting message
            let msg = Line::from(Span::styled(
                "Connecting...",
                Style::default().fg(theme.text_muted),
            ));
            let center_y = inner.y + inner.height / 2;
            let center_x = inner.x + (inner.width.saturating_sub(13)) / 2;
            f.render_widget(
                ratatui::widgets::Paragraph::new(msg),
                Rect::new(center_x, center_y, 13, 1),
            );
        }
    }

    fn capabilities(&self, _state: &AppState) -> PanelCapabilities {
        PanelCapabilities::RAW_INPUT
    }

    fn clone_box(&self) -> Box<dyn View> {
        Box::new(self.clone())
    }

    fn accepts_action(&self, action: &Action) -> bool {
        matches!(
            action,
            Action::ClaudeTerminal(_) | Action::Global(_)
        )
    }

    fn available_actions(&self, _state: &AppState) -> Vec<AvailableAction> {
        vec![AvailableAction::primary(CommandId::GlobalClose, "Detach")]
    }
}

/// Render terminal cells into the ratatui buffer directly
fn render_terminal_cells(f: &mut Frame, screen: &TerminalScreen, area: Rect) {
    let buf = f.buffer_mut();
    let visible_rows = area.height as usize;
    let visible_cols = area.width as usize;

    for (row_idx, row) in screen.rows.iter().enumerate() {
        if row_idx >= visible_rows {
            break;
        }
        let y = area.y + row_idx as u16;

        for (col_idx, cell) in row.iter().enumerate() {
            if col_idx >= visible_cols {
                break;
            }
            let x = area.x + col_idx as u16;

            let (fg, bg) = if cell.inverse {
                (to_ratatui_color(cell.bg), to_ratatui_color(cell.fg))
            } else {
                (to_ratatui_color(cell.fg), to_ratatui_color(cell.bg))
            };

            let mut style = Style::default().fg(fg).bg(bg);
            if cell.bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            if cell.underline {
                style = style.add_modifier(Modifier::UNDERLINED);
            }

            let buf_cell = &mut buf[(x, y)];
            buf_cell.set_char(cell.ch);
            buf_cell.set_style(style);
        }
    }
}

/// Convert our TerminalColor to a ratatui Color
fn to_ratatui_color(color: TerminalColor) -> Color {
    match color {
        TerminalColor::Default => Color::Reset,
        TerminalColor::Indexed(idx) => Color::Indexed(idx),
        TerminalColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
