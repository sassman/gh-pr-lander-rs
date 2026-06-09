//! Key-to-bytes translation
//!
//! Converts crossterm KeyEvent into byte sequences suitable for writing to a PTY.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Translate a crossterm KeyEvent into the byte sequence a terminal would send.
pub fn key_event_to_bytes(key: &KeyEvent) -> Vec<u8> {
    // Ctrl+letter → byte 1..=26
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && let KeyCode::Char(c) = key.code
    {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() {
            return vec![c as u8 - b'a' + 1];
        }
    }

    match key.code {
        KeyCode::Char(c) => {
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            s.as_bytes().to_vec()
        }
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::BackTab => vec![0x1b, b'[', b'Z'],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => vec![0x1b, b'[', b'A'],
        KeyCode::Down => vec![0x1b, b'[', b'B'],
        KeyCode::Right => vec![0x1b, b'[', b'C'],
        KeyCode::Left => vec![0x1b, b'[', b'D'],
        KeyCode::Home => vec![0x1b, b'[', b'H'],
        KeyCode::End => vec![0x1b, b'[', b'F'],
        KeyCode::PageUp => vec![0x1b, b'[', b'5', b'~'],
        KeyCode::PageDown => vec![0x1b, b'[', b'6', b'~'],
        KeyCode::Insert => vec![0x1b, b'[', b'2', b'~'],
        KeyCode::Delete => vec![0x1b, b'[', b'3', b'~'],
        KeyCode::F(n) => f_key_bytes(n),
        _ => vec![],
    }
}

/// Generate xterm escape sequence for function keys
fn f_key_bytes(n: u8) -> Vec<u8> {
    match n {
        1 => b"\x1bOP".to_vec(),
        2 => b"\x1bOQ".to_vec(),
        3 => b"\x1bOR".to_vec(),
        4 => b"\x1bOS".to_vec(),
        5 => b"\x1b[15~".to_vec(),
        6 => b"\x1b[17~".to_vec(),
        7 => b"\x1b[18~".to_vec(),
        8 => b"\x1b[19~".to_vec(),
        9 => b"\x1b[20~".to_vec(),
        10 => b"\x1b[21~".to_vec(),
        11 => b"\x1b[23~".to_vec(),
        12 => b"\x1b[24~".to_vec(),
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn test_printable_char() {
        assert_eq!(key_event_to_bytes(&key(KeyCode::Char('a'))), b"a");
        assert_eq!(key_event_to_bytes(&key(KeyCode::Char('Z'))), b"Z");
        assert_eq!(key_event_to_bytes(&key(KeyCode::Char('5'))), b"5");
    }

    #[test]
    fn test_multibyte_utf8() {
        let bytes = key_event_to_bytes(&key(KeyCode::Char('\u{00e9}'))); // é
        assert_eq!(bytes, "\u{00e9}".as_bytes());
    }

    #[test]
    fn test_enter() {
        assert_eq!(key_event_to_bytes(&key(KeyCode::Enter)), b"\r");
    }

    #[test]
    fn test_backspace() {
        assert_eq!(key_event_to_bytes(&key(KeyCode::Backspace)), b"\x7f");
    }

    #[test]
    fn test_tab() {
        assert_eq!(key_event_to_bytes(&key(KeyCode::Tab)), b"\t");
    }

    #[test]
    fn test_escape() {
        assert_eq!(key_event_to_bytes(&key(KeyCode::Esc)), b"\x1b");
    }

    #[test]
    fn test_arrow_keys() {
        assert_eq!(key_event_to_bytes(&key(KeyCode::Up)), b"\x1b[A");
        assert_eq!(key_event_to_bytes(&key(KeyCode::Down)), b"\x1b[B");
        assert_eq!(key_event_to_bytes(&key(KeyCode::Right)), b"\x1b[C");
        assert_eq!(key_event_to_bytes(&key(KeyCode::Left)), b"\x1b[D");
    }

    #[test]
    fn test_home_end() {
        assert_eq!(key_event_to_bytes(&key(KeyCode::Home)), b"\x1b[H");
        assert_eq!(key_event_to_bytes(&key(KeyCode::End)), b"\x1b[F");
    }

    #[test]
    fn test_page_keys() {
        assert_eq!(key_event_to_bytes(&key(KeyCode::PageUp)), b"\x1b[5~");
        assert_eq!(key_event_to_bytes(&key(KeyCode::PageDown)), b"\x1b[6~");
    }

    #[test]
    fn test_ctrl_letters() {
        assert_eq!(key_event_to_bytes(&ctrl_key('a')), vec![1]);
        assert_eq!(key_event_to_bytes(&ctrl_key('c')), vec![3]);
        assert_eq!(key_event_to_bytes(&ctrl_key('z')), vec![26]);
    }

    #[test]
    fn test_function_keys() {
        assert_eq!(key_event_to_bytes(&key(KeyCode::F(1))), b"\x1bOP");
        assert_eq!(key_event_to_bytes(&key(KeyCode::F(4))), b"\x1bOS");
        assert_eq!(key_event_to_bytes(&key(KeyCode::F(5))), b"\x1b[15~");
        assert_eq!(key_event_to_bytes(&key(KeyCode::F(12))), b"\x1b[24~");
    }
}
