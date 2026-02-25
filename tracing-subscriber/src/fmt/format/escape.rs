//! ANSI escape sequence sanitization to prevent terminal injection attacks.

use std::fmt::{self, Write};

/// A wrapper that implements `fmt::Debug` and escapes control sequences on-the-fly.
/// This avoids creating intermediate strings while providing security against terminal injection.
pub(super) struct Escape<T>(pub(super) T);

impl<T: fmt::Debug> fmt::Debug for Escape<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut escaping_writer = EscapingWriter {
            inner: f,
            skip_csi_codes: false,
        };
        write!(escaping_writer, "{:?}", self.0)
    }
}

/// A wrapper that implements `fmt::Debug` and removes control sequences on-the-fly.
/// This avoids creating intermediate strings while providing security against terminal injection.
pub(super) struct EscapeSkip<T>(pub(super) T);

impl<T: fmt::Debug> fmt::Debug for EscapeSkip<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut escaping_writer = EscapingWriter {
            inner: f,
            skip_csi_codes: true,
        };
        write!(escaping_writer, "{:?}", self.0)
    }
}

/// Helper struct that escapes ANSI sequences as characters are written
struct EscapingWriter<'a, 'b> {
    inner: &'a mut fmt::Formatter<'b>,
    skip_csi_codes: bool,
}

impl<'a, 'b> fmt::Write for EscapingWriter<'a, 'b> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Stream the string character by character, escaping all control sequences
        let mut chars = s.chars().peekable();
        while let Some(ch) = chars.next() {
            // Recognize and remove ECMA-48 CSI codes
            // This is a best effort to clean up colour codes in the message field
            if ch == '\x1b' && self.skip_csi_codes {
                if chars.next_if_eq(&'[').is_some() {
                    // Remove parameter and intermediate bytes
                    while chars.next_if(|x| matches!(x, '\x20'..='\x3f')).is_some() {}
                    // Remove final byte
                    chars.next_if(|x| matches!(x, '\x40'..='\x7E'));
                    continue;
                }
            }

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
