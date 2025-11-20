use crate::log::{JobMetadata, LogPanel, PrContext};
use gh_actions_log_parser::{AnsiStyle, Color as ParserColor, JobLog, NamedColor, StyledSegment};
use ratatui::{prelude::*, widgets::*};

/// Render the log panel as a card overlay with PR context header
/// Takes the available area (excluding top tabs and bottom panels)
/// Returns the calculated viewport height for page down scrolling
pub fn render_log_panel_card(
    f: &mut Frame,
    panel: &LogPanel,
    theme: &crate::theme::Theme,
    available_area: Rect,
) -> usize {
    // Use Clear widget to completely clear the underlying content
    f.render_widget(Clear, available_area);

    // Then render a solid background to ensure complete coverage
    let background = Block::default().style(Style::default().bg(theme.bg_panel));
    f.render_widget(background, available_area);

    // Use the full available area (same dimensions as PR panel)
    let card_area = available_area;

    // Split card into PR header (3 lines) and log content
    let card_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // PR context header
            Constraint::Min(0),    // Log content
        ])
        .split(card_area);

    // Render PR context header
    let pr_header_text = vec![
        Line::from(vec![
            Span::styled(
                format!("#{} ", panel.pr_context.number),
                Style::default()
                    .fg(theme.status_info)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                panel.pr_context.title.clone(),
                Style::default()
                    .fg(theme.text_primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            format!("by {}", panel.pr_context.author),
            Style::default().fg(theme.text_muted),
        )),
    ];

    let pr_header = Paragraph::new(pr_header_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(theme.bg_panel)),
    );

    f.render_widget(pr_header, card_chunks[0]);

    // Render log content in the remaining area and return viewport height
    render_log_panel_content(f, panel, card_chunks[1], theme)
}

/// Render the log panel showing build failure logs using a Table widget
/// OPTIMIZED: Only renders visible lines (viewport-based rendering)
/// Returns the visible viewport height for page down scrolling
fn render_log_panel_content(
    f: &mut Frame,
    panel: &LogPanel,
    area: Rect,
    theme: &crate::theme::Theme,
) -> usize {
    if panel.workflows.is_empty() {
        let empty_msg = Paragraph::new("No build logs found")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Build Logs ")
                    .border_style(Style::default().fg(theme.accent_primary))
                    .style(Style::default().bg(theme.bg_panel)),
            )
            .style(Style::default().fg(theme.text_muted).bg(theme.bg_panel))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(empty_msg, area);
        return 0;
    }

    render_log_tree(f, panel, area, theme)
}

/// Render the tree view of workflows/jobs/steps
fn render_log_tree(
    f: &mut Frame,
    panel: &LogPanel,
    area: Rect,
    theme: &crate::theme::Theme,
) -> usize {
    let visible_height = area.height.saturating_sub(2) as usize;

    // Build tree rows
    let mut rows = Vec::new();
    let visible_paths = panel.flatten_visible_nodes();

    for (display_idx, path) in visible_paths.iter().enumerate() {
        if display_idx < panel.scroll_offset {
            continue;
        }
        if rows.len() >= visible_height {
            break;
        }

        let is_cursor = path == &panel.cursor_path;
        let row_text = build_tree_row(panel, path, theme);

        let style = if is_cursor {
            Style::default()
                .bg(theme.selected_bg)
                .fg(theme.text_primary)
        } else {
            Style::default().bg(theme.bg_panel).fg(theme.text_primary)
        };

        rows.push(Row::new(vec![Cell::from(row_text)]).style(style));
    }

    let table = Table::new(rows, vec![Constraint::Percentage(100)])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Build Logs - PR #{} | j/k: navigate, Enter: toggle, n: next error, x: close ",
                    panel.pr_context.number
                ))
                .border_style(Style::default().fg(theme.accent_primary))
                .style(Style::default().bg(theme.bg_panel)),
        )
        .style(Style::default().bg(theme.bg_panel));

    f.render_widget(table, area);
    visible_height
}

/// Build display text for a single tree row
fn build_tree_row(panel: &LogPanel, path: &[usize], theme: &crate::theme::Theme) -> Line<'static> {
    let indent_level = path.len();
    let indent = "  ".repeat(indent_level.saturating_sub(1));

    match path.len() {
        1 => {
            // Workflow node
            let workflow = &panel.workflows[path[0]];
            let has_children = !workflow.jobs.is_empty();
            let expanded = panel.is_expanded(path);
            let icon = if !has_children {
                " " // No icon if no children
            } else if expanded {
                "▼"
            } else {
                "▶"
            };
            let status_icon = if workflow.has_failures { "✗" } else { "✓" };
            let error_info = if workflow.total_errors > 0 {
                format!(" ({} errors)", workflow.total_errors)
            } else {
                String::new()
            };

            Line::from(format!(
                "{}{} {} {}{}",
                indent, icon, status_icon, workflow.name, error_info
            ))
        }
        2 => {
            // Job node
            let workflow = &panel.workflows[path[0]];
            let job = &workflow.jobs[path[1]];
            let has_children = !job.steps.is_empty();
            let expanded = panel.is_expanded(path);
            let icon = if !has_children {
                " " // No icon if no children
            } else if expanded {
                "▼"
            } else {
                "▶"
            };
            let status_icon = if job.error_count > 0 { "✗" } else { "✓" };

            let error_info = if job.error_count > 0 {
                format!(" ({} errors)", job.error_count)
            } else {
                String::new()
            };

            // Try to get metadata for duration
            let key = format!("{}:{}", workflow.name, job.name);
            let duration_info = if let Some(metadata) = panel.job_metadata.get(&key) {
                if let Some(duration) = metadata.duration {
                    let secs = duration.as_secs();
                    if secs >= 60 {
                        format!(", {}m {}s", secs / 60, secs % 60)
                    } else {
                        format!(", {}s", secs)
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            Line::from(format!(
                "{}├─ {} {} {}{}{}",
                indent, icon, status_icon, job.name, error_info, duration_info
            ))
        }
        3 => {
            // Step node
            let workflow = &panel.workflows[path[0]];
            let job = &workflow.jobs[path[1]];
            let step = &job.steps[path[2]];
            let has_children = !step.lines.is_empty();
            let expanded = panel.is_expanded(path);
            let icon = if !has_children {
                " " // No icon if no children
            } else if expanded {
                "▼"
            } else {
                "▶"
            };
            let status_icon = if step.error_count > 0 { "✗" } else { "✓" };
            let error_info = if step.error_count > 0 {
                format!(" ({} errors)", step.error_count)
            } else {
                String::new()
            };

            Line::from(format!(
                "{}│  ├─ {} {}{}{}",
                indent, icon, status_icon, step.name, error_info
            ))
        }
        4 => {
            // Log line (leaf node - no icon)
            let workflow = &panel.workflows[path[0]];
            let job = &workflow.jobs[path[1]];
            let step = &job.steps[path[2]];
            let line = &step.lines[path[3]];

            // Build styled line content with proper indentation
            let base_style = Style::default().fg(theme.text_primary).bg(theme.bg_panel);

            // Check if this is an error line
            let is_error = if let Some(ref cmd) = line.command {
                matches!(cmd, gh_actions_log_parser::WorkflowCommand::Error { .. })
            } else {
                line.display_content.to_lowercase().contains("error:")
            };

            // Determine line style based on type
            let line_style = if is_error {
                Style::default()
                    .fg(theme.status_error)
                    .add_modifier(Modifier::BOLD)
                    .bg(theme.bg_panel)
            } else if line.is_command {
                // Style command invocations in yellow
                Style::default().fg(Color::Yellow).bg(theme.bg_panel)
            } else {
                base_style
            };

            // Use styled segments from parser if available
            let content = if !line.styled_segments.is_empty() {
                styled_segments_to_line(&line.styled_segments, line_style, panel.horizontal_scroll)
            } else {
                // Fallback to plain text
                let text = if panel.horizontal_scroll > 0 {
                    line.display_content
                        .chars()
                        .skip(panel.horizontal_scroll)
                        .collect::<String>()
                } else {
                    line.display_content.clone()
                };
                Line::from(Span::styled(text, line_style))
            };

            // Add tree indentation prefix
            let prefix = format!("{}│     ", indent);
            let mut spans = vec![Span::styled(
                prefix,
                Style::default().fg(theme.text_muted).bg(theme.bg_panel),
            )];

            // Add timestamp if enabled and available
            if panel.show_timestamps
                && let Some(ref timestamp) = line.timestamp
            {
                spans.push(Span::styled(
                    format!("[{}] ", timestamp),
                    Style::default().fg(theme.text_muted).bg(theme.bg_panel),
                ));
            }

            spans.extend(content.spans);
            Line::from(spans)
        }
        _ => Line::from(""),
    }
}

/// Convert parser ANSI color to ratatui Color
fn convert_color(color: &ParserColor) -> Color {
    match color {
        ParserColor::Rgb(r, g, b) => Color::Rgb(*r, *g, *b),
        ParserColor::Palette256(idx) => Color::Indexed(*idx),
        ParserColor::Named(named) => match named {
            NamedColor::Black => Color::Black,
            NamedColor::Red => Color::Red,
            NamedColor::Green => Color::Green,
            NamedColor::Yellow => Color::Yellow,
            NamedColor::Blue => Color::Blue,
            NamedColor::Magenta => Color::Magenta,
            NamedColor::Cyan => Color::Cyan,
            NamedColor::White => Color::White,
            NamedColor::BrightBlack => Color::DarkGray,
            NamedColor::BrightRed => Color::LightRed,
            NamedColor::BrightGreen => Color::LightGreen,
            NamedColor::BrightYellow => Color::LightYellow,
            NamedColor::BrightBlue => Color::LightBlue,
            NamedColor::BrightMagenta => Color::LightMagenta,
            NamedColor::BrightCyan => Color::LightCyan,
            NamedColor::BrightWhite => Color::Gray,
        },
    }
}

/// Convert parser ANSI style to ratatui Style
fn convert_style(ansi_style: &AnsiStyle, base_style: Style) -> Style {
    let mut style = base_style;

    if let Some(fg) = &ansi_style.fg_color {
        style = style.fg(convert_color(fg));
    }

    if let Some(bg) = &ansi_style.bg_color {
        style = style.bg(convert_color(bg));
    }

    if ansi_style.bold {
        style = style.add_modifier(Modifier::BOLD);
    }

    if ansi_style.italic {
        style = style.add_modifier(Modifier::ITALIC);
    }

    if ansi_style.underline {
        style = style.add_modifier(Modifier::UNDERLINED);
    }

    if ansi_style.reversed {
        style = style.add_modifier(Modifier::REVERSED);
    }

    if ansi_style.strikethrough {
        style = style.add_modifier(Modifier::CROSSED_OUT);
    }

    style
}

/// Convert styled segments to ratatui Line with proper styling
fn styled_segments_to_line(
    segments: &[StyledSegment],
    base_style: Style,
    h_scroll: usize,
) -> Line<'static> {
    if segments.is_empty() {
        return Line::from("");
    }

    let mut spans = Vec::new();
    let mut char_count = 0;

    for segment in segments {
        let text_len = segment.text.chars().count();

        // Apply horizontal scrolling
        if char_count + text_len <= h_scroll {
            // This segment is completely before the scroll offset
            char_count += text_len;
            continue;
        }

        let skip_chars = h_scroll.saturating_sub(char_count);

        let visible_text: String = segment.text.chars().skip(skip_chars).collect();
        if !visible_text.is_empty() {
            let style = convert_style(&segment.style, base_style);
            spans.push(Span::styled(visible_text, style));
        }

        char_count += text_len;
    }

    Line::from(spans)
}

/// Create LogPanel from parsed job logs (tree view)
/// Builds a hierarchical tree: Workflow → Job → Step
pub fn create_log_panel_from_jobs(
    jobs: Vec<(JobMetadata, JobLog)>,
    pr_context: PrContext,
) -> LogPanel {
    use std::collections::HashMap;

    // Group jobs by workflow name and convert to tree using parser
    let mut workflows_map: HashMap<String, Vec<(JobMetadata, gh_actions_log_parser::JobNode)>> =
        HashMap::new();
    let mut job_metadata_map: HashMap<String, JobMetadata> = HashMap::new();

    for (metadata, job_log) in jobs {
        let job_node = gh_actions_log_parser::job_log_to_tree(job_log);

        // Filter out jobs with no logs AND "/system" in name
        let has_logs =
            !job_node.steps.is_empty() && job_node.steps.iter().any(|step| !step.lines.is_empty());
        let has_system = job_node.name.contains("/system");

        // Skip this job if it has no logs AND has /system in name
        if !has_logs && has_system {
            continue;
        }

        let key = format!("{}:{}", metadata.workflow_name, job_node.name);
        job_metadata_map.insert(key, metadata.clone());

        workflows_map
            .entry(metadata.workflow_name.clone())
            .or_default()
            .push((metadata, job_node));
    }

    // Build workflow nodes (jobs already filtered above)
    let mut workflows: Vec<gh_actions_log_parser::WorkflowNode> = workflows_map
        .into_iter()
        .map(|(workflow_name, jobs)| {
            let mut job_nodes: Vec<gh_actions_log_parser::JobNode> =
                jobs.into_iter().map(|(_, job)| job).collect();

            // Sort jobs alphabetically by name
            job_nodes.sort_by(|a, b| a.name.cmp(&b.name));

            let total_errors: usize = job_nodes.iter().map(|j| j.error_count).sum();
            let has_failures = total_errors > 0;

            gh_actions_log_parser::WorkflowNode {
                name: workflow_name,
                jobs: job_nodes,
                total_errors,
                has_failures,
            }
        })
        .collect();

    // Sort workflows: failed first, then by name
    workflows.sort_by(|a, b| {
        b.has_failures
            .cmp(&a.has_failures)
            .then(a.name.cmp(&b.name))
    });

    // Auto-expand workflows (top level) and nodes with errors
    let mut expanded_nodes = std::collections::HashSet::new();
    for (w_idx, workflow) in workflows.iter().enumerate() {
        // Always expand workflows (top level)
        expanded_nodes.insert(w_idx.to_string());

        // Auto-expand jobs and steps with errors
        for (j_idx, job) in workflow.jobs.iter().enumerate() {
            if job.error_count > 0 {
                expanded_nodes.insert(format!("{}:{}", w_idx, j_idx));

                for (s_idx, step) in job.steps.iter().enumerate() {
                    if step.error_count > 0 {
                        expanded_nodes.insert(format!("{}:{}:{}", w_idx, j_idx, s_idx));
                    }
                }
            }
        }
    }

    LogPanel {
        workflows,
        job_metadata: job_metadata_map,
        expanded_nodes,
        cursor_path: vec![0], // Start at first workflow
        scroll_offset: 0,
        horizontal_scroll: 0,
        show_timestamps: false,
        viewport_height: 20,
        pr_context,
    }
}
