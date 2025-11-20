use ratatui::{
    layout::{Margin, Rect},
    prelude::*,
    widgets::*,
};

use crate::App;
use crate::state::{AddRepoField, AddRepoForm, LoadingState};
use crate::theme::Theme;

/// Render the repository tabs showing all tracked repositories
pub fn render_repository_tabs(f: &mut Frame, area: Rect, app: &App) {
    // Render tabs (always visible when there are repos)
    let tab_titles: Vec<Line> = app
        .store
        .state()
        .repos
        .recent_repos
        .iter()
        .enumerate()
        .map(|(i, repo)| {
            // Check if this repo is currently loading
            let is_loading = app
                .store
                .state()
                .repos
                .repo_data
                .get(&i)
                .map(|data| matches!(data.loading_state, LoadingState::Loading))
                .unwrap_or(false);

            let number = if i < 9 {
                format!("{} ", i + 1)
            } else {
                String::new()
            };

            // Add sandglass before number if loading
            let prefix = if is_loading { "⏳ " } else { "" };

            Line::from(format!("{}{}{}/{}", prefix, number, repo.org, repo.repo))
        })
        .collect();

    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Projects [Tab/1-9: switch, /: cycle] | Filter: {} [f: cycle]",
            app.store.state().repos.filter.label()
        )))
        .select(app.store.state().repos.selected_repo)
        .style(Style::default().fg(app.store.state().repos.colors.row_fg))
        .highlight_style(
            Style::default()
                .fg(app.store.state().repos.colors.selected_row_style_fg)
                .add_modifier(Modifier::BOLD)
                .bg(app.store.state().repos.colors.header_bg),
        );

    f.render_widget(tabs, area);
}

/// Render the add repository popup as a centered floating window
pub fn render_add_repo_popup(f: &mut Frame, area: Rect, form: &AddRepoForm, theme: &Theme) {
    use ratatui::widgets::{Clear, Wrap};

    // Calculate centered area (60% width, 50% height)
    let popup_width = (area.width * 60 / 100).min(70);
    let popup_height = 14; // Fixed height for the form
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect {
        x: area.x + popup_x,
        y: area.y + popup_y,
        width: popup_width,
        height: popup_height,
    };

    // Clear the area and render background
    f.render_widget(Clear, popup_area);
    f.render_widget(
        Block::default().style(Style::default().bg(theme.bg_panel)),
        popup_area,
    );

    // Render border and title
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Add New Repository ")
        .title_style(
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(theme.bg_panel));

    f.render_widget(block, popup_area);

    // Calculate inner area
    let inner = popup_area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });

    // Build form content
    let mut text_lines = Vec::new();

    // Instructions
    text_lines.push(Line::from(vec![Span::styled(
        "Enter GitHub URL or fill in the fields manually:",
        Style::default().fg(theme.text_secondary),
    )]));
    text_lines.push(Line::from(""));

    // Organization field
    let org_focused = form.focused_field == AddRepoField::Org;
    text_lines.push(Line::from(vec![
        Span::styled(
            if org_focused { "> " } else { "  " },
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Organization: ",
            Style::default()
                .fg(if org_focused {
                    theme.active_fg
                } else {
                    theme.text_primary
                })
                .add_modifier(if org_focused {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
        Span::styled(
            &form.org,
            Style::default()
                .fg(if org_focused {
                    theme.active_fg
                } else {
                    theme.text_primary
                })
                .bg(if org_focused {
                    theme.active_bg
                } else {
                    theme.bg_panel
                }),
        ),
    ]));

    // Repository field
    let repo_focused = form.focused_field == AddRepoField::Repo;
    text_lines.push(Line::from(vec![
        Span::styled(
            if repo_focused { "> " } else { "  " },
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Repository:   ",
            Style::default()
                .fg(if repo_focused {
                    theme.active_fg
                } else {
                    theme.text_primary
                })
                .add_modifier(if repo_focused {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
        Span::styled(
            &form.repo,
            Style::default()
                .fg(if repo_focused {
                    theme.active_fg
                } else {
                    theme.text_primary
                })
                .bg(if repo_focused {
                    theme.active_bg
                } else {
                    theme.bg_panel
                }),
        ),
    ]));

    // Branch field
    let branch_focused = form.focused_field == AddRepoField::Branch;
    let branch_display = if form.branch.is_empty() {
        "main (default)"
    } else {
        &form.branch
    };
    text_lines.push(Line::from(vec![
        Span::styled(
            if branch_focused { "> " } else { "  " },
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Branch:       ",
            Style::default()
                .fg(if branch_focused {
                    theme.active_fg
                } else {
                    theme.text_primary
                })
                .add_modifier(if branch_focused {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
        Span::styled(
            branch_display,
            Style::default()
                .fg(if branch_focused {
                    theme.active_fg
                } else {
                    theme.text_muted
                })
                .bg(if branch_focused {
                    theme.active_bg
                } else {
                    theme.bg_panel
                }),
        ),
    ]));

    text_lines.push(Line::from(""));
    text_lines.push(Line::from(""));

    // Footer with shortcuts
    text_lines.push(Line::from(vec![
        Span::styled(
            "Tab",
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" navigate  ", Style::default().fg(theme.text_muted)),
        Span::styled(
            "Enter",
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" add  ", Style::default().fg(theme.text_muted)),
        Span::styled(
            "Esc",
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" cancel", Style::default().fg(theme.text_muted)),
    ]));

    // Render content
    let paragraph = Paragraph::new(text_lines)
        .wrap(Wrap { trim: false })
        .style(Style::default().bg(theme.bg_panel));

    f.render_widget(paragraph, inner);
}
