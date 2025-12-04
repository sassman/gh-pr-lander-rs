//! File tree widget for navigation.

use crate::model::{FileStatus, FileTreeNode, FlatFileEntry};
use crate::traits::ThemeProvider;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Widget};

/// Widget for rendering the file tree navigation pane.
pub struct FileTreeWidget<'a, T: ThemeProvider> {
    /// The file tree to render.
    tree: &'a FileTreeNode,
    /// Currently selected index in the flattened tree.
    selected: usize,
    /// Whether this pane is focused.
    focused: bool,
    /// Theme provider.
    theme: &'a T,
}

impl<'a, T: ThemeProvider> FileTreeWidget<'a, T> {
    /// Create a new file tree widget.
    pub fn new(tree: &'a FileTreeNode, selected: usize, focused: bool, theme: &'a T) -> Self {
        Self {
            tree,
            selected,
            focused,
            theme,
        }
    }
}

impl<T: ThemeProvider> Widget for FileTreeWidget<'_, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Draw border
        let border_style = if self.focused {
            Style::default().fg(self.theme.file_tree_border())
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Files ");

        let inner = block.inner(area);
        block.render(area, buf);

        // Get flattened entries
        let entries = self.tree.flatten();
        let visible_height = inner.height as usize;

        // Calculate scroll offset to keep selected visible
        let scroll_offset = if self.selected >= visible_height {
            self.selected - visible_height + 1
        } else {
            0
        };

        // Render visible entries
        for (i, entry) in entries.iter().skip(scroll_offset).take(visible_height).enumerate() {
            let y = inner.y + i as u16;
            if y >= inner.y + inner.height {
                break;
            }

            let is_selected = i + scroll_offset == self.selected;
            self.render_entry(entry, inner.x, y, inner.width, is_selected, buf);
        }
    }
}

impl<T: ThemeProvider> FileTreeWidget<'_, T> {
    fn render_entry(&self, entry: &FlatFileEntry, x: u16, y: u16, width: u16, selected: bool, buf: &mut Buffer) {
        // Build the line content
        let indent = "  ".repeat(entry.depth);
        let icon = entry.icon();

        // Status indicator
        let status_char = match entry.status {
            Some(FileStatus::Added) => "+",
            Some(FileStatus::Deleted) => "-",
            Some(FileStatus::Modified) => "~",
            Some(FileStatus::Renamed) => "→",
            Some(FileStatus::Copied) => "©",
            None => "",
        };

        // Stats
        let stats = if entry.additions > 0 || entry.deletions > 0 {
            format!(" +{}/-{}", entry.additions, entry.deletions)
        } else {
            String::new()
        };

        // Calculate available width for name
        let prefix_len = indent.len() + icon.len() + status_char.len();
        let stats_len = stats.len();
        let available = (width as usize).saturating_sub(prefix_len + stats_len + 1);

        // Truncate name if needed
        let name = if entry.name.len() > available {
            format!("{}…", &entry.name[..available.saturating_sub(1)])
        } else {
            entry.name.clone()
        };

        // Determine style
        let base_style = if selected {
            Style::default()
                .fg(self.theme.file_tree_selected_foreground())
                .bg(self.theme.file_tree_selected_background())
        } else {
            Style::default()
        };

        // Fill the line with background
        if selected {
            for i in 0..width {
                buf.set_string(x + i, y, " ", base_style);
            }
        }

        let mut current_x = x;

        // Render indent
        buf.set_string(current_x, y, &indent, base_style);
        current_x += indent.len() as u16;

        // Render icon
        let icon_style = if entry.is_dir {
            base_style.fg(self.theme.file_tree_directory_foreground())
        } else {
            base_style
        };
        buf.set_string(current_x, y, icon, icon_style);
        current_x += icon.len() as u16;

        // Render status
        if !status_char.is_empty() {
            let status_color = entry.status.map(|s| s.color()).unwrap_or(Color::White);
            buf.set_string(current_x, y, status_char, base_style.fg(status_color));
            current_x += status_char.len() as u16;
        }

        // Render name
        let name_style = if entry.is_dir {
            base_style.fg(self.theme.file_tree_directory_foreground())
        } else {
            base_style
        };
        buf.set_string(current_x, y, &name, name_style);
        current_x += name.len() as u16;

        // Render stats at the end
        if !stats.is_empty() {
            let stats_x = x + width - stats.len() as u16;
            if stats_x > current_x {
                let add_style = base_style.fg(Color::Green);
                let del_style = base_style.fg(Color::Red);

                // Parse and render colored stats
                let parts: Vec<&str> = stats.split('/').collect();
                if parts.len() == 2 {
                    buf.set_string(stats_x, y, parts[0], add_style);
                    buf.set_string(stats_x + parts[0].len() as u16, y, "/", base_style);
                    buf.set_string(stats_x + parts[0].len() as u16 + 1, y, parts[1], del_style);
                } else {
                    buf.set_string(stats_x, y, &stats, base_style);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FileDiff;
    use crate::traits::DefaultTheme;

    #[test]
    fn test_file_tree_widget_creation() {
        let files = vec![
            {
                let mut f = FileDiff::new("src/main.rs");
                f.additions = 10;
                f
            },
            {
                let mut f = FileDiff::new("src/lib.rs");
                f.additions = 5;
                f
            },
        ];

        let tree = FileTreeNode::from_files(&files);
        let theme = DefaultTheme;
        let _widget = FileTreeWidget::new(&tree, 0, true, &theme);
    }
}
