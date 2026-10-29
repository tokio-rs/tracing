//! A `MakeVisitor` wrapper that truncates long formatted field values.
use super::{MakeVisitor, VisitFmt, VisitOutput};

use alloc::string::String;
use core::fmt;
use tracing_core::field::{Field, Visit};

const ELLIPSIS: &str = "...";

/// A `MakeVisitor` wrapper that truncates long string and `fmt::Debug` field
/// values.
#[derive(Debug, Clone)]
pub struct Truncated<V> {
    max_len: usize,
    inner: V,
}

/// A visitor wrapper that truncates long string and `fmt::Debug` field values.
#[derive(Debug)]
pub struct VisitTruncated<V> {
    max_len: usize,
    inner: V,
}

struct TruncatedDebug<'a, T: ?Sized>(&'a T, usize);

struct TruncateWriter<'a, W: ?Sized> {
    inner: &'a mut W,
    remaining: usize,
    truncated: bool,
}

// === impl Truncated ===

impl<V> Truncated<V> {
    /// Returns a new [`MakeVisitor`] implementation that wraps `inner` so that
    /// string and `fmt::Debug` field values longer than `max_len` characters
    /// are truncated and followed by `...`.
    ///
    /// [`MakeVisitor`]: super::MakeVisitor
    pub fn new(max_len: usize, inner: V) -> Self {
        Self { max_len, inner }
    }
}

impl<T, V> MakeVisitor<T> for Truncated<V>
where
    V: MakeVisitor<T>,
{
    type Visitor = VisitTruncated<V::Visitor>;

    fn make_visitor(&self, target: T) -> Self::Visitor {
        VisitTruncated::new(self.max_len, self.inner.make_visitor(target))
    }
}

// === impl VisitTruncated ===

impl<V> VisitTruncated<V> {
    /// Returns a new [`Visit`] implementation that wraps `inner` so that
    /// string and `fmt::Debug` field values longer than `max_len` characters
    /// are truncated and followed by `...`.
    ///
    /// [`Visit`]: tracing_core::field::Visit
    pub fn new(max_len: usize, inner: V) -> Self {
        Self { max_len, inner }
    }
}

impl<V> Visit for VisitTruncated<V>
where
    V: Visit,
{
    #[inline]
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.inner.record_f64(field, value)
    }

    #[inline]
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.inner.record_i64(field, value)
    }

    #[inline]
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.inner.record_u64(field, value)
    }

    #[inline]
    fn record_i128(&mut self, field: &Field, value: i128) {
        self.inner.record_i128(field, value)
    }

    #[inline]
    fn record_u128(&mut self, field: &Field, value: u128) {
        self.inner.record_u128(field, value)
    }

    #[inline]
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.inner.record_bool(field, value)
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        let Some((end, _)) = value.char_indices().nth(self.max_len) else {
            self.inner.record_str(field, value);
            return;
        };

        let mut truncated = String::with_capacity(end + ELLIPSIS.len());
        truncated.push_str(&value[..end]);
        truncated.push_str(ELLIPSIS);
        self.inner.record_str(field, &truncated);
    }

    #[inline]
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.inner
            .record_debug(field, &TruncatedDebug(value, self.max_len))
    }
}

impl<V> VisitOutput<fmt::Result> for VisitTruncated<V>
where
    V: VisitOutput<fmt::Result>,
{
    #[inline]
    fn finish(self) -> fmt::Result {
        self.inner.finish()
    }
}

impl<V> VisitFmt for VisitTruncated<V>
where
    V: VisitFmt,
{
    #[inline]
    fn writer(&mut self) -> &mut dyn fmt::Write {
        self.inner.writer()
    }
}

impl<T> fmt::Debug for TruncatedDebug<'_, T>
where
    T: fmt::Debug + ?Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let alternate = f.alternate();
        let mut writer = TruncateWriter {
            inner: f,
            remaining: self.1,
            truncated: false,
        };

        if alternate {
            fmt::write(&mut writer, format_args!("{:#?}", self.0))
        } else {
            fmt::write(&mut writer, format_args!("{:?}", self.0))
        }
    }
}

impl<W> fmt::Write for TruncateWriter<'_, W>
where
    W: fmt::Write + ?Sized,
{
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if self.truncated {
            return Ok(());
        }

        if let Some((end, _)) = value.char_indices().nth(self.remaining) {
            self.inner.write_str(&value[..end])?;
            self.inner.write_str(ELLIPSIS)?;
            self.truncated = true;
        } else {
            self.remaining -= value.chars().count();
            self.inner.write_str(value)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::field::test_util::*;

    #[test]
    fn long_string_is_truncated() {
        let mut s = String::new();
        let visitor = DebugVisitor::new(&mut s);
        let mut visitor = VisitTruncated::new(10, visitor);

        TestAttrs1::with(|attrs| attrs.record(&mut visitor));
        visitor.finish().unwrap();

        assert_eq!(
            s.as_str(),
            "question=\"life, the ...\"tricky=truecan_you_do_it=true"
        );
    }

    #[test]
    fn short_string_is_untouched() {
        let mut s = String::new();
        let visitor = DebugVisitor::new(&mut s);
        let mut visitor = VisitTruncated::new(35, visitor);

        TestAttrs1::with(|attrs| attrs.record(&mut visitor));
        visitor.finish().unwrap();

        assert_eq!(
            s.as_str(),
            "question=\"life, the universe, and everything\"tricky=truecan_you_do_it=true"
        );
    }

    #[test]
    fn long_debug_value_is_truncated() {
        let make = Truncated::new(1, MakeDebug);

        TestAttrs2::with(|attrs| {
            let mut s = String::new();
            {
                let mut visitor = make.make_visitor(&mut s);
                attrs.record(&mut visitor);
            }
            assert_eq!(
                s.as_str(),
                "question=N...question.answer=42tricky=truecan_you_do_it=false"
            );
        });
    }

    #[test]
    fn string_truncation_is_utf8_safe() {
        let mut s = String::new();
        let visitor = DebugVisitor::new(&mut s);
        let mut visitor = VisitTruncated::new(2, visitor);

        TestAttrs1::with(|attrs| {
            let field = attrs.metadata().fields().field("question").unwrap();
            visitor.record_str(&field, "é日🙂");
            visitor.record_debug(&field, &format_args!("é日🙂"));
        });
        visitor.finish().unwrap();

        assert_eq!(s.as_str(), "question=\"é日...\"question=é日...");
    }
}
