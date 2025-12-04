//! Diff content widget for rendering the actual diff.

use crate::highlight::DiffHighlighter;
use crate::model::{DiffLine, FileDiff, Hunk, LineKind, PendingComment};
use crate::traits::ThemeProvider;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Widget};

/// Widget for rendering the diff content pane.
pub struct DiffContentWidget<'a, T: ThemeProvider> {
    /// The file diff to render.
    file: Option<&'a FileDiff>,
    /// Current cursor line.
    cursor_line: usize,
    /// Scroll offset.
    scroll_offset: usize,
    /// Visual selection range (if any).
    visual_selection: Option<(usize, usize)>,
    /// Pending comments for this file.
    comments: &'a [&'a PendingComment],
    /// Syntax highlighter.
    highlighter: &'a mut DiffHighlighter,
    /// Theme provider.
    theme: &'a T,
    /// Whether this pane is focused.
    focused: bool,
}

impl<'a, T: ThemeProvider> DiffContentWidget<'a, T> {
    /// Create a new diff content widget.
    pub fn new(
        file: Option<&'a FileDiff>,
        cursor_line: usize,
        scroll_offset: usize,
        comments: &'a [&'a PendingComment],
        highlighter: &'a mut DiffHighlighter,
        theme: &'a T,
        focused: bool,
    ) -> Self {
        Self {
            file,
            cursor_line,
            scroll_offset,
            visual_selection: None,
            comments,
            highlighter,
            theme,
            focused,
        }
    }

    /// Set visual selection range.
    pub fn with_selection(mut self, selection: Option<(usize, usize)>) -> Self {
        self.visual_selection = selection;
        self
    }
}

impl<T: ThemeProvider> Widget for DiffContentWidget<'_, T> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        // Draw border
        let border_style = if self.focused {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let title = self.file
            .map(|f| format!(" {} ", f.display_name()))
            .unwrap_or_else(|| " No file selected ".to_string());

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title);

        let inner = block.inner(area);
        block.render(area, buf);

        let Some(file) = self.file else {
            // Render empty state
            let msg = "Select a file from the tree";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.set_string(x, y, msg, Style::default().fg(Color::DarkGray));
            return;
        };

        // Calculate line number width
        let max_line_no = file.hunks.iter()
            .flat_map(|h| h.lines.iter())
            .filter_map(|l| l.new_line.or(l.old_line))
            .max()
            .unwrap_or(1);
        let line_no_width = max_line_no.to_string().len().max(4);

        // Flatten hunks into display lines - collect indices only
        let display_info = flatten_hunk_info(&file.hunks);
        let visible_height = inner.height as usize;
        let file_path = file.path.as_str();

        // Render visible lines
        for (i, (hunk_idx, line_idx)) in display_info
            .iter()
            .skip(self.scroll_offset)
            .take(visible_height)
            .enumerate()
        {
            let y = inner.y + i as u16;
            if y >= inner.y + inner.height {
                break;
            }

            let display_idx = i + self.scroll_offset;
            let is_cursor = display_idx == self.cursor_line;
            let in_selection = self.visual_selection
                .map(|(start, end)| display_idx >= start && display_idx <= end)
                .unwrap_or(false);

            // Get the actual line data
            let hunk = &file.hunks[*hunk_idx];
            if let Some(line_idx) = line_idx {
                let line = &hunk.lines[*line_idx];
                self.render_diff_line(
                    line,
                    inner.x,
                    y,
                    inner.width,
                    line_no_width,
                    is_cursor,
                    in_selection,
                    file_path,
                    buf,
                );
            } else {
                // Hunk header
                self.render_hunk_header(&hunk.header, inner.x, y, inner.width, is_cursor, buf);
            }
        }
    }
}

/// Flatten hunks into (hunk_idx, Option<line_idx>) pairs for iteration.
fn flatten_hunk_info(hunks: &[Hunk]) -> Vec<(usize, Option<usize>)> {
    let mut result = Vec::new();

    for (hunk_idx, hunk) in hunks.iter().enumerate() {
        // Hunk header
        result.push((hunk_idx, None));

        // Lines
        for line_idx in 0..hunk.lines.len() {
            result.push((hunk_idx, Some(line_idx)));
        }
    }

    result
}

impl<T: ThemeProvider> DiffContentWidget<'_, T> {
    fn render_hunk_header(&self, header: &str, x: u16, y: u16, width: u16, is_cursor: bool, buf: &mut Buffer) {
        let bg = if is_cursor {
            self.theme.cursor_background()
        } else {
            self.theme.hunk_header_background()
        };

        let style = Style::default()
            .fg(self.theme.hunk_header_foreground())
            .bg(bg);

        // Fill background
        for i in 0..width {
            buf.set_string(x + i, y, " ", style);
        }

        // Render header text (truncate if needed)
        let display_header = if header.len() > width as usize {
            &header[..width as usize]
        } else {
            header
        };
        buf.set_string(x, y, display_header, style);
    }

    fn render_diff_line(
        &mut self,
        line: &DiffLine,
        x: u16,
        y: u16,
        width: u16,
        line_no_width: usize,
        is_cursor: bool,
        in_selection: bool,
        file_path: &str,
        buf: &mut Buffer,
    ) {
        // Determine background color
        let bg = if is_cursor {
            self.theme.cursor_background()
        } else if in_selection {
            Color::Rgb(60, 60, 80) // Selection highlight
        } else {
            match line.kind {
                LineKind::Addition => self.theme.addition_background(),
                LineKind::Deletion => self.theme.deletion_background(),
                LineKind::Expansion => self.theme.expansion_marker_background(),
                _ => self.theme.context_background(),
            }
        };

        let base_style = Style::default().bg(bg);

        // Fill background
        for i in 0..width {
            buf.set_string(x + i, y, " ", base_style);
        }

        let mut current_x = x;

        // Render old line number
        let old_no = line.old_line
            .map(|n| format!("{:>width$}", n, width = line_no_width))
            .unwrap_or_else(|| " ".repeat(line_no_width));
        buf.set_string(
            current_x,
            y,
            &old_no,
            base_style.fg(self.theme.line_number_foreground()),
        );
        current_x += line_no_width as u16;

        // Separator
        buf.set_string(current_x, y, " ", base_style);
        current_x += 1;

        // Render new line number
        let new_no = line.new_line
            .map(|n| format!("{:>width$}", n, width = line_no_width))
            .unwrap_or_else(|| " ".repeat(line_no_width));
        buf.set_string(
            current_x,
            y,
            &new_no,
            base_style.fg(self.theme.line_number_foreground()),
        );
        current_x += line_no_width as u16;

        // Separator and prefix
        buf.set_string(current_x, y, " ", base_style);
        current_x += 1;

        let prefix = match line.kind {
            LineKind::Addition => "+",
            LineKind::Deletion => "-",
            LineKind::Expansion => "~",
            _ => " ",
        };
        let prefix_style = match line.kind {
            LineKind::Addition => base_style.fg(Color::Green),
            LineKind::Deletion => base_style.fg(Color::Red),
            LineKind::Expansion => base_style.fg(self.theme.expansion_marker_foreground()),
            _ => base_style,
        };
        buf.set_string(current_x, y, prefix, prefix_style);
        current_x += 1;

        // Content area width
        let content_width = width.saturating_sub(current_x - x) as usize;

        // Render content (with syntax highlighting for non-expansion lines)
        if line.kind == LineKind::Expansion {
            // Expansion marker text
            let text = "... expand to see more ...";
            buf.set_string(
                current_x,
                y,
                text,
                base_style.fg(self.theme.expansion_marker_foreground()),
            );
        } else {
            // Syntax highlight and render
            let highlighted = self.highlighter.highlight_line(file_path, &line.content);

            let mut col = 0;
            for span in highlighted {
                if col >= content_width {
                    break;
                }

                let available = content_width - col;
                let text = if span.text.len() > available {
                    &span.text[..available]
                } else {
                    &span.text
                };

                let mut style = base_style;
                if let Some(fg) = span.fg {
                    style = style.fg(fg);
                }
                if span.bold {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if span.italic {
                    style = style.add_modifier(Modifier::ITALIC);
                }

                buf.set_string(current_x + col as u16, y, text, style);
                col += text.len();
            }
        }

        // Show expanded indicator
        if line.is_expanded {
            let indicator_x = x + width - 2;
            if indicator_x > current_x {
                buf.set_string(indicator_x, y, "↕", base_style.fg(Color::DarkGray));
            }
        }

        // Show comment indicator
        let has_comment = self.comments.iter().any(|c| {
            c.position.line == line.new_line.or(line.old_line).unwrap_or(0)
        });
        if has_comment {
            let indicator_x = x + width - 4;
            if indicator_x > current_x {
                buf.set_string(
                    indicator_x,
                    y,
                    "💬",
                    base_style.fg(self.theme.comment_indicator_foreground()),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::DefaultTheme;

    #[test]
    fn test_diff_content_widget_creation() {
        let mut file = FileDiff::new("src/test.rs");
        let mut hunk = Hunk::new(1, 3, 1, 4);
        hunk.lines.push(DiffLine::context("line 1", 1, 1));
        hunk.lines.push(DiffLine::addition("new line", 2));
        file.hunks.push(hunk);

        let comments: Vec<&PendingComment> = vec![];
        let mut highlighter = DiffHighlighter::new();
        let theme = DefaultTheme;

        let _widget = DiffContentWidget::new(
            Some(&file),
            0,
            0,
            &comments,
            &mut highlighter,
            &theme,
            true,
        );
    }
}
