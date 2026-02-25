//! ANSI escape sequence sanitization to prevent terminal injection attacks.

use std::fmt::{self, Write};

/// A wrapper that implements `fmt::Debug` and `fmt::Display` and escapes ANSI sequences on-the-fly.
/// This avoids creating intermediate strings while providing security against terminal injection.
pub(super) struct Escape<T>(pub(super) T);

/// Helper struct that escapes ANSI sequences as characters are written
struct EscapingWriter<'a, 'b> {
    inner: &'a mut fmt::Formatter<'b>,
}

impl<'a, 'b> fmt::Write for EscapingWriter<'a, 'b> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Stream the string character by character, escaping all control sequences
        for ch in s.chars() {
            // ESC BEL BS FF DEL
            if matches!(ch, '\x1b' | '\x07' | '\x08' | '\x0c' | '\x7f'..='\u{9f}') {
                if ch.is_ascii() {
                    write!(self.inner, "\\x{:02x}", ch as u32)?
                } else {
                    write!(self.inner, "\\u{{{:x}}}", ch as u32)?
                }
            } else {
                self.inner.write_char(ch)?;
            }
        }
        Ok(())
    }
}

impl<T: fmt::Debug> fmt::Debug for Escape<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut escaping_writer = EscapingWriter { inner: f };
        write!(escaping_writer, "{:?}", self.0)
    }
}

impl<T: fmt::Display> fmt::Display for Escape<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut escaping_writer = EscapingWriter { inner: f };
        write!(escaping_writer, "{}", self.0)
    }
}
