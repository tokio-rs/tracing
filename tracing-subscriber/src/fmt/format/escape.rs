//! ANSI escape sequence sanitization to prevent terminal injection attacks.

use std::{
    convert::TryInto,
    fmt::{self, Write},
};

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

/// Whether `b` may start a character that needs escaping: ESC, BEL, BS, FF,
/// DEL, or 0xC2, the lead byte of the C1 controls U+0080..=U+009F.
///
/// Uses `|` rather than `matches!` so the chunk scan below compiles to SIMD.
#[inline(always)]
fn may_escape(b: u8) -> bool {
    (b == 0x1b) | (b == 0x07) | (b == 0x08) | (b == 0x0c) | (b == 0x7f) | (b == 0xc2)
}

/// Index of the next byte at or after `from` for which [`may_escape`] holds.
fn next_candidate(bytes: &[u8], mut from: usize) -> Option<usize> {
    const CHUNK: usize = 16;
    while let Some(chunk) = bytes.get(from..from + CHUNK) {
        let chunk: &[u8; CHUNK] = chunk.try_into().unwrap();
        if chunk.iter().fold(0u8, |acc, &b| acc | may_escape(b) as u8) != 0 {
            break;
        }
        from += CHUNK;
    }
    bytes[from..]
        .iter()
        .position(|&b| may_escape(b))
        .map(|offset| from + offset)
}

impl<'a, 'b> fmt::Write for EscapingWriter<'a, 'b> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Escape ANSI and C1 control sequences, but pass the text between them
        // through in one call: writing char by char is slow on long messages.
        let bytes = s.as_bytes();
        let mut start = 0;
        let mut idx = 0;
        while let Some(found) = next_candidate(bytes, idx) {
            idx = found;
            match (bytes[idx], bytes.get(idx + 1)) {
                // C1 control characters (\x80-\x9f) - 8-bit control codes
                // These can be used as alternative escape sequences in some terminals
                (0xc2, Some(&c1 @ 0x80..=0x9f)) => {
                    self.inner.write_str(&s[start..idx])?;
                    write!(self.inner, "\\u{{{:x}}}", c1)?;
                    idx += 2;
                }
                // Any other character starting with 0xC2 is printable
                (0xc2, _) => {
                    idx += 1;
                    continue;
                }
                // C0 control characters that can be used in terminal escape sequences
                // (ESC, BEL, BS, FF, DEL)
                (b, _) => {
                    self.inner.write_str(&s[start..idx])?;
                    write!(self.inner, "\\x{:02x}", b)?;
                    idx += 1;
                }
            }
            start = idx;
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{format, string::String};

    fn escape_char_by_char(s: &str) -> String {
        let mut out = String::new();
        for ch in s.chars() {
            match ch {
                '\x1b' | '\x07' | '\x08' | '\x0c' | '\x7f' => {
                    write!(out, "\\x{:02x}", ch as u32).unwrap()
                }
                '\u{80}'..='\u{9f}' => write!(out, "\\u{{{:x}}}", ch as u32).unwrap(),
                _ => out.push(ch),
            }
        }
        out
    }

    #[test]
    fn matches_char_by_char_at_every_offset() {
        // '\u{a0}' and '©' share the 0xC2 lead byte with C1 controls but are printable.
        let specials = [
            '\x1b', '\x07', '\x08', '\x0c', '\x7f', '\u{80}', '\u{9b}', '\u{9f}', '\u{a0}',
            '©', 'ü', '日', '😀',
        ];
        for special in specials {
            for len in 0..40 {
                for pos in 0..=len {
                    let mut s: String = "a".repeat(len);
                    s.insert(pos, special);
                    s.push('\x1b');
                    let escaped = format!("{}", EscapeGuard::new(&s, true));
                    assert_eq!(escaped, escape_char_by_char(&s), "input: {:?}", s);
                }
            }
        }
    }
}
