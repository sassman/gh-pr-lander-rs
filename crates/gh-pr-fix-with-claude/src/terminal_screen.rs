//! Terminal screen model
//!
//! Provides a serializable representation of a terminal screen buffer,
//! extracted from vt100::Screen for use in ratatui rendering.

/// A single terminal cell with character and styling
#[derive(Debug, Clone)]
pub struct TerminalCell {
    pub ch: char,
    pub fg: TerminalColor,
    pub bg: TerminalColor,
    pub bold: bool,
    pub underline: bool,
    pub inverse: bool,
}

impl Default for TerminalCell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: TerminalColor::Default,
            bg: TerminalColor::Default,
            bold: false,
            underline: false,
            inverse: false,
        }
    }
}

/// Terminal color representation
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum TerminalColor {
    #[default]
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

/// A snapshot of the terminal screen buffer
#[derive(Debug, Clone)]
pub struct TerminalScreen {
    pub rows: Vec<Vec<TerminalCell>>,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub cols: u16,
    pub lines: u16,
}

impl TerminalScreen {
    /// Convert a vt100::Screen into our TerminalScreen representation
    pub fn from_vt100(screen: &vt100::Screen) -> Self {
        let (rows_count, cols_count) = screen.size();
        let mut rows = Vec::with_capacity(rows_count as usize);

        for row in 0..rows_count {
            let mut cells = Vec::with_capacity(cols_count as usize);
            for col in 0..cols_count {
                let cell = screen.cell(row, col);
                let terminal_cell = match cell {
                    Some(cell) => {
                        let ch = cell.contents().chars().next().unwrap_or(' ');
                        TerminalCell {
                            ch,
                            fg: convert_color(cell.fgcolor()),
                            bg: convert_color(cell.bgcolor()),
                            bold: cell.bold(),
                            underline: cell.underline(),
                            inverse: cell.inverse(),
                        }
                    }
                    None => TerminalCell::default(),
                };
                cells.push(terminal_cell);
            }
            rows.push(cells);
        }

        let (cursor_row, cursor_col) = screen.cursor_position();

        TerminalScreen {
            rows,
            cursor_row,
            cursor_col,
            cols: cols_count,
            lines: rows_count,
        }
    }
}

/// Convert a vt100::Color to our TerminalColor
fn convert_color(color: vt100::Color) -> TerminalColor {
    match color {
        vt100::Color::Default => TerminalColor::Default,
        vt100::Color::Idx(idx) => TerminalColor::Indexed(idx),
        vt100::Color::Rgb(r, g, b) => TerminalColor::Rgb(r, g, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let mut parser = vt100::Parser::new(24, 80, 0);
        parser.process(b"Hello");

        let screen = TerminalScreen::from_vt100(parser.screen());
        assert_eq!(screen.lines, 24);
        assert_eq!(screen.cols, 80);
        assert_eq!(screen.rows[0][0].ch, 'H');
        assert_eq!(screen.rows[0][1].ch, 'e');
        assert_eq!(screen.rows[0][2].ch, 'l');
        assert_eq!(screen.rows[0][3].ch, 'l');
        assert_eq!(screen.rows[0][4].ch, 'o');
        assert_eq!(screen.rows[0][5].ch, ' ');
    }

    #[test]
    fn test_cursor_position() {
        let mut parser = vt100::Parser::new(24, 80, 0);
        parser.process(b"Hello");

        let screen = TerminalScreen::from_vt100(parser.screen());
        assert_eq!(screen.cursor_row, 0);
        assert_eq!(screen.cursor_col, 5);
    }

    #[test]
    fn test_bold_text() {
        let mut parser = vt100::Parser::new(24, 80, 0);
        // ESC[1m = bold on, then text
        parser.process(b"\x1b[1mBold\x1b[0m");

        let screen = TerminalScreen::from_vt100(parser.screen());
        assert!(screen.rows[0][0].bold);
        assert_eq!(screen.rows[0][0].ch, 'B');
        // After reset, bold should be off
        assert!(!screen.rows[0][4].bold);
    }

    #[test]
    fn test_fg_color() {
        let mut parser = vt100::Parser::new(24, 80, 0);
        // ESC[31m = red foreground
        parser.process(b"\x1b[31mRed\x1b[0m");

        let screen = TerminalScreen::from_vt100(parser.screen());
        assert_eq!(screen.rows[0][0].fg, TerminalColor::Indexed(1));
        assert_eq!(screen.rows[0][0].ch, 'R');
    }

    #[test]
    fn test_bg_color() {
        let mut parser = vt100::Parser::new(24, 80, 0);
        // ESC[42m = green background
        parser.process(b"\x1b[42mGreen\x1b[0m");

        let screen = TerminalScreen::from_vt100(parser.screen());
        assert_eq!(screen.rows[0][0].bg, TerminalColor::Indexed(2));
    }

    #[test]
    fn test_rgb_color() {
        let mut parser = vt100::Parser::new(24, 80, 0);
        // ESC[38;2;255;128;0m = RGB foreground
        parser.process(b"\x1b[38;2;255;128;0mRGB\x1b[0m");

        let screen = TerminalScreen::from_vt100(parser.screen());
        assert_eq!(screen.rows[0][0].fg, TerminalColor::Rgb(255, 128, 0));
    }

    #[test]
    fn test_underline() {
        let mut parser = vt100::Parser::new(24, 80, 0);
        parser.process(b"\x1b[4mUnderlined\x1b[0m");

        let screen = TerminalScreen::from_vt100(parser.screen());
        assert!(screen.rows[0][0].underline);
    }

    #[test]
    fn test_inverse() {
        let mut parser = vt100::Parser::new(24, 80, 0);
        parser.process(b"\x1b[7mInverse\x1b[0m");

        let screen = TerminalScreen::from_vt100(parser.screen());
        assert!(screen.rows[0][0].inverse);
    }

    #[test]
    fn test_newline_moves_to_next_row() {
        let mut parser = vt100::Parser::new(24, 80, 0);
        parser.process(b"Line1\r\nLine2");

        let screen = TerminalScreen::from_vt100(parser.screen());
        assert_eq!(screen.rows[0][0].ch, 'L');
        assert_eq!(screen.rows[0][4].ch, '1');
        assert_eq!(screen.rows[1][0].ch, 'L');
        assert_eq!(screen.rows[1][4].ch, '2');
    }
}
