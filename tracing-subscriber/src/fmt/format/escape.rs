//! ANSI escape sequence sanitization to prevent terminal injection attacks.

use std::fmt::{self, Write};

/// A wrapper that conditionally escapes ANSI sequences when formatted.
pub(super) struct EscapeGuard<T> {
    pub(super) value: T,
    pub(super) sanitize: bool,
}

impl<T> EscapeGuard<T> {
    pub(super) fn new(value: T, sanitize: bool) -> Self {
        Self { value, sanitize }
    }
}

/// Helper struct that escapes ANSI sequences as characters are written
struct EscapingWriter<'a, 'b> {
    inner: &'a mut fmt::Formatter<'b>,
}

impl<'a, 'b> fmt::Write for EscapingWriter<'a, 'b> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Escape ANSI and C1 control sequences, but pass the text between them
        // through in one call: writing char by char is slow on long messages.
        let mut start = 0;
        for (idx, ch) in s.char_indices() {
            match ch {
                // C0 control characters that can be used in terminal escape sequences
                // (ESC, BEL, BS, FF, DEL)
                '\x1b' | '\x07' | '\x08' | '\x0c' | '\x7f' => {
                    self.inner.write_str(&s[start..idx])?;
                    write!(self.inner, "\\x{:02x}", ch as u32)?
                }

                // C1 control characters (\x80-\x9f) - 8-bit control codes
                // These can be used as alternative escape sequences in some terminals
                '\u{80}'..='\u{9f}' => {
                    self.inner.write_str(&s[start..idx])?;
                    write!(self.inner, "\\u{{{:x}}}", ch as u32)?
                }

                _ => continue,
            }
            start = idx + ch.len_utf8();
        }
        self.inner.write_str(&s[start..])
    }
}

impl<T: fmt::Debug> fmt::Debug for EscapeGuard<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.sanitize {
            let mut escaping_writer = EscapingWriter { inner: f };
            write!(escaping_writer, "{:?}", self.value)
        } else {
            write!(f, "{:?}", self.value)
        }
    }
}

impl<T: fmt::Display> fmt::Display for EscapeGuard<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.sanitize {
            let mut escaping_writer = EscapingWriter { inner: f };
            write!(escaping_writer, "{}", self.value)
        } else {
            write!(f, "{}", self.value)
        }
    }
}
