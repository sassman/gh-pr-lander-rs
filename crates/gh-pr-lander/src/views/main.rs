use crate::capabilities::PanelCapabilities;
use crate::state::AppState;
use crate::views::View;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
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

    // Create main block
    let block = Block::default()
        .title(" Github PR Lander ")
        .borders(Borders::ALL)
        .border_style(theme.panel_border())
        .title_style(theme.panel_title());

    // Split into repository tabs area and content area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Repository tab bar
            Constraint::Min(0),    // Content area
        ])
        .split(block.inner(area));

    // Render the outer block
    f.render_widget(block, area);

    // Render repository tabs
    let tab_titles = vec!["Repository 1", "Repository 2"];
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::BOTTOM))
        .select(state.main_view.selected_repository)
        .style(theme.panel_background())
        .highlight_style(theme.success().bold());

    f.render_widget(tabs, chunks[0]);

    // Render repository content based on selected repository
    let content = match state.main_view.selected_repository {
        0 => render_repo1_content(theme),
        1 => render_repo2_content(theme),
        _ => vec![Line::from("Invalid repository")],
    };

    let paragraph = Paragraph::new(content)
        .style(theme.panel_background())
        .alignment(Alignment::Center);

    f.render_widget(paragraph, chunks[1]);
}

/// Render content for Repository 1
fn render_repo1_content(theme: &crate::theme::Theme) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(Span::styled(
            "Welcome to Repository 1!",
            theme.success().bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "This is placeholder content for the first repository",
            theme.text_secondary(),
        )),
        Line::from(""),
        Line::from(Span::styled("Controls:", theme.section_header())),
        Line::from(vec![
            Span::styled("  Tab/Shift+Tab  ", theme.key_hint()),
            Span::styled("- Switch repositories", theme.key_description()),
        ]),
        Line::from(vec![
            Span::styled("  `              ", theme.key_hint()),
            Span::styled("- Toggle debug console", theme.key_description()),
        ]),
        Line::from(vec![
            Span::styled("  q or Esc       ", theme.key_hint()),
            Span::styled("- Quit", theme.key_description()),
        ]),
    ]
}

/// Render content for Repository 2
fn render_repo2_content(theme: &crate::theme::Theme) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(Span::styled(
            "Welcome to Repository 2!",
            theme.success().bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "This is placeholder content for the second repository",
            theme.text_secondary(),
        )),
        Line::from(""),
        Line::from("More content coming soon..."),
    ]
}
