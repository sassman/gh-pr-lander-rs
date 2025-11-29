use crate::capabilities::PanelCapabilities;
use crate::domain_models::LoadingState;
use crate::state::AppState;
use crate::theme::Theme;
use crate::view_models::{EmptyPrTableViewModel, PrTableViewModel};
use crate::views::View;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::Line,
    widgets::{Block, Cell, Paragraph, Row, Table, Widget},
    Frame,
};

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
        render(state, area, f);
    }

    fn capabilities(&self, _state: &AppState) -> PanelCapabilities {
        // Main view supports vim navigation
        PanelCapabilities::VIM_NAVIGATION_BINDINGS
    }

    fn clone_box(&self) -> Box<dyn View> {
        Box::new(self.clone())
    }
}

/// Render the main view
fn render(state: &AppState, area: Rect, f: &mut Frame) {
    let theme = &state.theme;

    // Split into repository tabs area and content area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Repository tab bar (single row)
            Constraint::Min(0),    // Content area with QuadrantOutside border
        ])
        .split(area);

    // Generate tab titles and loading states from repositories
    let tab_data: Vec<(String, bool)> = state
        .main_view
        .repositories
        .iter()
        .enumerate()
        .map(|(idx, repo)| {
            let title = format!("{}/{}@{}", repo.org, repo.repo, repo.branch);
            let is_loading = state.main_view.repo_data.get(&idx).is_none_or(|data| {
                matches!(
                    data.loading_state,
                    LoadingState::Idle | LoadingState::Loading
                )
            });
            (title, is_loading)
        })
        .collect();
    let tabs_widget = ModernTabs::new(tab_data, state.main_view.selected_repository, theme);
    f.render_widget(tabs_widget, chunks[0]);

    // Render PR table for the selected repository
    render_pr_table(state, chunks[1], f);
}

/// Render the PR table for the currently selected repository
fn render_pr_table(state: &AppState, area: Rect, f: &mut Frame) {
    let theme = &state.theme;
    let repo_idx = state.main_view.selected_repository;

    // Get current repository and its data
    let repo = state.main_view.repositories.get(repo_idx);
    let repo_data = state.main_view.repo_data.get(&repo_idx);

    // Handle empty state
    if repo.is_none() {
        let vm = EmptyPrTableViewModel::no_repos();
        render_empty_state(&vm, area, f, theme);
        return;
    }

    let repo = repo.unwrap();

    // Check loading state
    match repo_data.map(|rd| &rd.loading_state) {
        None | Some(LoadingState::Idle) => {
            let vm = EmptyPrTableViewModel::loading();
            render_empty_state(&vm, area, f, theme);
            return;
        }
        Some(LoadingState::Loading) => {
            let vm = EmptyPrTableViewModel::loading();
            render_empty_state(&vm, area, f, theme);
            return;
        }
        Some(LoadingState::Error(err)) => {
            let vm = EmptyPrTableViewModel::error(err);
            render_empty_state(&vm, area, f, theme);
            return;
        }
        Some(LoadingState::Loaded) => {
            // Continue to render the table
        }
    }

    let repo_data = repo_data.unwrap();

    // Check if there are any PRs
    if repo_data.prs.is_empty() {
        let vm = EmptyPrTableViewModel::no_prs();
        render_empty_state(&vm, area, f, theme);
        return;
    }

    // Build view model
    let vm = PrTableViewModel::from_repo_data(repo_data, repo, theme);

    // Build block with header
    let status_line = Line::from(vm.header.status_text.clone())
        .style(ratatui::style::Style::default().fg(vm.header.status_color))
        .right_aligned();

    let block = Block::bordered()
        .border_type(ratatui::widgets::BorderType::QuadrantOutside)
        .border_style(ratatui::style::Style::default().fg(theme.accent_primary))
        .title(vm.header.title.clone())
        .title(status_line);

    // Build header row
    let header_style = ratatui::style::Style::default()
        .fg(theme.accent_primary)
        .add_modifier(Modifier::BOLD);

    let header_cells = ["#PR", "Title", "Author", "Comments", "Status"]
        .iter()
        .map(|h| Cell::from(*h).style(header_style));

    let header = Row::new(header_cells).height(1);

    // Build rows from view model
    let rows: Vec<Row> = vm
        .rows
        .iter()
        .map(|row_vm| {
            let style = ratatui::style::Style::default()
                .fg(row_vm.fg_color)
                .bg(row_vm.bg_color);

            Row::new(vec![
                Cell::from(row_vm.pr_number.clone()),
                Cell::from(row_vm.title.clone()),
                Cell::from(row_vm.author.clone()),
                Cell::from(row_vm.comments.clone()),
                Cell::from(row_vm.status_text.clone())
                    .style(ratatui::style::Style::default().fg(row_vm.status_color)),
            ])
            .style(style)
            .height(1)
        })
        .collect();

    let widths = [
        Constraint::Length(6),      // #PR
        Constraint::Percentage(50), // Title
        Constraint::Percentage(15), // Author
        Constraint::Length(10),     // Comments
        Constraint::Percentage(15), // Status
    ];

    // Selected row style
    let selected_style = ratatui::style::Style::default()
        .bg(theme.selected_bg)
        .fg(theme.active_fg)
        .add_modifier(Modifier::BOLD);

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(selected_style)
        .highlight_symbol("> ");

    // Create a table state for highlighting
    let mut table_state = ratatui::widgets::TableState::default();
    table_state.select(Some(vm.selected_index));

    f.render_stateful_widget(table, area, &mut table_state);
}

/// Render empty/loading state
fn render_empty_state(vm: &EmptyPrTableViewModel, area: Rect, f: &mut Frame, theme: &Theme) {
    let block = Block::bordered()
        .border_type(ratatui::widgets::BorderType::QuadrantOutside)
        .border_style(ratatui::style::Style::default().fg(theme.accent_primary));

    let paragraph = Paragraph::new(vm.message.clone())
        .block(block)
        .style(theme.muted())
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

/// Modern background-color style tabs widget
/// Uses background colors instead of borders - active tab has prominent color,
/// inactive tabs are subtle. Content frame matches selected tab's color.
struct ModernTabs<'a> {
    /// Tab data: (title, is_loading)
    tabs: Vec<(String, bool)>,
    selected: usize,
    theme: &'a Theme,
}

/// Hourglass icon for loading state
const HOURGLASS_ICON: &str = "⏳";

impl<'a> ModernTabs<'a> {
    fn new(tabs: Vec<(String, bool)>, selected: usize, theme: &'a Theme) -> Self {
        Self {
            tabs,
            selected,
            theme,
        }
    }
}

impl Widget for ModernTabs<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 1 || area.width < 10 {
            return;
        }

        // Modern style: background colors only, no borders on tabs
        // Selected:   ████████████ (prominent color)
        // Unselected: ▒▒▒▒▒▒▒▒▒▒▒▒ (subtle)

        let mut x = area.x + 1; // Start with a small margin

        // Colors
        let active_bg = self.theme.accent_primary;
        let active_fg = self.theme.bg_primary;
        let inactive_bg = self.theme.bg_tertiary;
        let inactive_fg = self.theme.text_muted;

        // Render each tab (just 1 row of content with background)
        for (i, (title, is_loading)) in self.tabs.iter().enumerate() {
            let is_selected = i == self.selected;

            // Build the display title with optional loading icon
            let display_title = if *is_loading {
                format!("{} {}", HOURGLASS_ICON, title)
            } else {
                title.clone()
            };

            let tab_width = display_title.chars().count() as u16 + 4; // 2 chars padding on each side

            if x + tab_width > area.x + area.width {
                break; // Don't overflow
            }

            // Style based on selection
            let style = if is_selected {
                ratatui::style::Style::default()
                    .fg(active_fg)
                    .bg(active_bg)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                ratatui::style::Style::default()
                    .fg(inactive_fg)
                    .bg(inactive_bg)
            };

            // Render tab background and text
            let padded_title = format!("  {}  ", display_title);
            buf.set_string(x, area.y, &padded_title, style);

            x += tab_width + 1; // Gap between tabs
        }

        // Render "add repository" hint tab at the end
        let hint_text = " p → a ";
        let hint_width = hint_text.len() as u16;
        if x + hint_width <= area.x + area.width {
            let hint_style = ratatui::style::Style::default()
                .fg(self.theme.text_muted)
                .add_modifier(ratatui::style::Modifier::DIM);
            buf.set_string(x, area.y, hint_text, hint_style);
        }
    }
}
