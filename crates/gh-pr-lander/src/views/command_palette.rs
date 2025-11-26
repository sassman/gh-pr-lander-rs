use crate::capabilities::PanelCapabilities;
use crate::commands::{filter_commands, get_all_commands};
use crate::state::AppState;
use crate::views::View;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

/// Command palette view - searchable command launcher
#[derive(Debug, Clone)]
pub struct CommandPaletteView;

impl CommandPaletteView {
    pub fn new() -> Self {
        Self
    }
}

impl View for CommandPaletteView {
    fn view_id(&self) -> crate::views::ViewId {
        crate::views::ViewId::CommandPalette
    }

    fn render(&self, state: &AppState, area: Rect, f: &mut Frame) {
        render(state, area, f);
    }

    fn capabilities(&self, _state: &AppState) -> PanelCapabilities {
        // Command palette only supports arrow key navigation (not vim keys)
        // This allows j/k/h/l to be typed into the search field
        PanelCapabilities::empty()
    }

    fn clone_box(&self) -> Box<dyn View> {
        Box::new(self.clone())
    }
}

/// Render the command palette as a centered floating panel
fn render(state: &AppState, area: Rect, f: &mut Frame) {
    let theme = &state.theme;

    // Get all commands and filter by query
    let all_commands = get_all_commands();
    let filtered_commands = filter_commands(&all_commands, &state.command_palette.query);

    // Calculate centered area (70% width, 60% height)
    let popup_width = (area.width * 70 / 100).min(100);
    let popup_height = (area.height * 60 / 100).min(30);
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect {
        x: area.x + popup_x,
        y: area.y + popup_y,
        width: popup_width,
        height: popup_height,
    };

    // Clear the area behind the popup
    f.render_widget(Clear, popup_area);

    // Render background
    f.render_widget(
        Block::default().style(theme.panel_background()),
        popup_area,
    );

    // Render border and title with command count
    let title = format!(" Command Palette ({} commands) ", filtered_commands.len());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(theme.panel_title().add_modifier(Modifier::BOLD))
        .border_style(theme.panel_border().add_modifier(Modifier::BOLD))
        .style(theme.panel_background());

    f.render_widget(block, popup_area);

    // Calculate inner area with margins
    let inner = popup_area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });

    // Split into input area, results area, details area, and footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input box
            Constraint::Min(5),    // Results list
            Constraint::Length(2), // Details area
            Constraint::Length(1), // Footer
        ])
        .split(inner);

    // Render input box
    let input_text = if state.command_palette.query.is_empty() {
        Line::from(vec![Span::styled(
            "Type to search commands...",
            theme.muted().italic(),
        )])
    } else {
        Line::from(vec![Span::styled(&state.command_palette.query, theme.text())])
    };

    let input_paragraph = Paragraph::new(input_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.panel_border())
                .style(theme.panel_background()),
        )
        .style(theme.text());

    f.render_widget(input_paragraph, chunks[0]);

    // Render results list
    if filtered_commands.is_empty() {
        let no_results = Paragraph::new("No matching commands")
            .style(theme.muted())
            .alignment(Alignment::Center);
        f.render_widget(no_results, chunks[1]);
    } else {
        // Calculate max category width for better table layout
        let max_category_width = filtered_commands
            .iter()
            .map(|cmd| cmd.category.len())
            .max()
            .unwrap_or(10)
            as u16
            + 2; // Add padding

        // Build table rows
        let rows: Vec<Row> = filtered_commands
            .iter()
            .enumerate()
            .map(|(idx, cmd)| {
                let is_selected = idx == state.command_palette.selected_index;

                let indicator = if is_selected { "> " } else { "  " };

                let indicator_style = if is_selected {
                    theme.success().bold()
                } else {
                    theme.muted()
                };

                let title_style = if is_selected {
                    theme.success().bold()
                } else {
                    theme.text()
                };

                let category_style = if is_selected {
                    theme.text_secondary().bold()
                } else {
                    theme.muted()
                };

                let bg_color = if is_selected {
                    theme.panel_background() // Could use a highlight background if available
                } else {
                    theme.panel_background()
                };

                Row::new(vec![
                    Cell::from(indicator).style(indicator_style),
                    Cell::from(cmd.title.clone()).style(title_style),
                    Cell::from(cmd.category.clone()).style(category_style),
                ])
                .style(bg_color)
            })
            .collect();

        let table = Table::new(
            rows,
            vec![
                Constraint::Length(2),          // Indicator
                Constraint::Percentage(70),     // Title
                Constraint::Length(max_category_width), // Category
            ],
        )
        .style(theme.panel_background());

        f.render_widget(table, chunks[1]);
    }

    // Render details area with selected command description
    if let Some(cmd) = filtered_commands.get(state.command_palette.selected_index) {
        let details_line = Line::from(vec![Span::styled(
            &cmd.description,
            theme.text_secondary(),
        )]);

        let details_paragraph = Paragraph::new(details_line)
            .wrap(Wrap { trim: false })
            .style(theme.panel_background());

        f.render_widget(details_paragraph, chunks[2]);
    }

    // Render footer with keyboard hints
    let footer_line = Line::from(vec![
        Span::styled("Enter", theme.key_hint().bold()),
        Span::styled(" execute  ", theme.key_description()),
        Span::styled("↑/↓", theme.key_hint().bold()),
        Span::styled(" navigate  ", theme.key_description()),
        Span::styled("Esc", theme.key_hint().bold()),
        Span::styled(" close", theme.key_description()),
    ]);

    let footer = Paragraph::new(footer_line)
        .style(theme.muted())
        .alignment(Alignment::Center);

    f.render_widget(footer, chunks[3]);
}
