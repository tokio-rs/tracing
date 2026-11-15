//! A `MakeVisitor` wrapper that truncates string field values to a maximum length.
use super::{MakeVisitor, VisitFmt, VisitOutput};
use tracing_core::field::{Field, Visit};

#[cfg(feature = "alloc")]
use alloc::string::String;
use core::fmt;

/// A visitor wrapper that truncates string field values longer than a maximum
/// number of characters.
///
/// String values recorded via [`Visit::record_str`] that exceed `max_len`
/// characters are shortened to `max_len` characters before being forwarded to
/// the wrapped visitor. All other field types are forwarded unchanged.
///
/// This is useful for keeping log output readable when some fields may contain
/// large strings.
///
/// Truncation is counted in [`char`]s rather than bytes, so the truncated value
/// is always valid UTF-8.
#[derive(Debug, Clone)]
pub struct Truncated<V> {
    inner: V,
    max_len: usize,
    #[cfg(feature = "alloc")]
    indicator: Option<String>,
}

// === impl Truncated ===

impl<V> Truncated<V> {
    /// Returns a new [`MakeVisitor`] implementation that wraps `inner` so that
    /// string field values longer than `max_len` characters are truncated.
    ///
    /// A `max_len` of 0 is allowed, and records every string value as the
    /// empty string.
    ///
    /// [`MakeVisitor`]: super::MakeVisitor
    pub fn new(inner: V, max_len: usize) -> Self {
        Self {
            inner,
            max_len,
            #[cfg(feature = "alloc")]
            indicator: None,
        }
    }
}

feature! {
    #![feature = "alloc"]

    impl<V> Truncated<V> {
        /// Appends `indicator` (such as `"..."` or `"[truncated]"`) to values
        /// that were truncated, so that truncation is visible in the output.
        ///
        /// Values short enough to be recorded unchanged do not get the
        /// indicator.
        pub fn with_indicator(self, indicator: impl Into<String>) -> Self {
            Self {
                indicator: Some(indicator.into()),
                ..self
            }
        }
    }
}

impl<T, V> MakeVisitor<T> for Truncated<V>
where
    V: MakeVisitor<T>,
{
    type Visitor = Truncated<V::Visitor>;

    #[inline]
    fn make_visitor(&self, target: T) -> Self::Visitor {
        Truncated {
            inner: self.inner.make_visitor(target),
            max_len: self.max_len,
            #[cfg(feature = "alloc")]
            indicator: self.indicator.clone(),
        }
    }
}

impl<V> Visit for Truncated<V>
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

    /// Visit a string value, truncating it to `max_len` characters if longer.
    fn record_str(&mut self, field: &Field, value: &str) {
        match value.char_indices().nth(self.max_len) {
            Some((byte_idx, _)) => {
                #[cfg(feature = "alloc")]
                if let Some(indicator) = &self.indicator {
                    let mut truncated = String::with_capacity(byte_idx + indicator.len());
                    truncated.push_str(&value[..byte_idx]);
                    truncated.push_str(indicator);
                    return self.inner.record_str(field, &truncated);
                }
                self.inner.record_str(field, &value[..byte_idx])
            }
            None => self.inner.record_str(field, value),
        }
    }

    #[inline]
    fn record_bytes(&mut self, field: &Field, value: &[u8]) {
        self.inner.record_bytes(field, value)
    }

    #[cfg(feature = "std")]
    #[inline]
    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.inner.record_error(field, value)
    }

    #[cfg(all(tracing_unstable, feature = "valuable"))]
    fn record_value(&mut self, field: &Field, value: valuable_crate::Value<'_>) {
        self.inner.record_value(field, value)
    }

    /// Note: values recorded via [`Visit::record_debug`] are *not* truncated,
    /// since their formatted length is not known without rendering them first.
    #[inline]
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.inner.record_debug(field, value)
    }
}

impl<V, O> VisitOutput<O> for Truncated<V>
where
    V: VisitOutput<O>,
{
    #[inline]
    fn finish(self) -> O {
        self.inner.finish()
    }
}

feature! {
    #![feature = "std"]
    use super::VisitWrite;
    use std::io;

    impl<V> VisitWrite for Truncated<V>
    where
        V: VisitWrite,
    {
        #[inline]
        fn writer(&mut self) -> &mut dyn io::Write {
            self.inner.writer()
        }
    }
}

impl<V> VisitFmt for Truncated<V>
where
    V: VisitFmt,
{
    #[inline]
    fn writer(&mut self) -> &mut dyn fmt::Write {
        self.inner.writer()
    }
}

#[cfg(all(test, feature = "alloc"))]
mod test {
    use super::*;
    use crate::field::test_util::*;

    #[test]
    fn truncates_long_string_values() {
        let make = Truncated::new(MakeDebug, 4);

        TestAttrs1::with(|attrs| {
            let mut s = String::new();
            {
                let mut v = make.make_visitor(&mut s);
                attrs.record(&mut v);
            }
            // "life, the universe, and everything" is truncated to "life".
            assert_eq!(s.as_str(), "question=\"life\"tricky=truecan_you_do_it=true");
        });
    }

    #[test]
    fn leaves_short_values_unchanged() {
        let make = Truncated::new(MakeDebug, 1000);

        TestAttrs1::with(|attrs| {
            let mut s = String::new();
            {
                let mut v = make.make_visitor(&mut s);
                attrs.record(&mut v);
            }
            assert_eq!(
                s.as_str(),
                "question=\"life, the universe, and everything\"tricky=truecan_you_do_it=true"
            );
        });
    }

    #[test]
    fn truncates_multibyte_chars() {
        let make = Truncated::new(MakeDebug, 3);

        TestAttrs1::with(|attrs| {
            let field = attrs.fields().field("question").unwrap();
            let mut s = String::new();
            {
                let mut v = make.make_visitor(&mut s);
                // "lïfë" has 4 characters, so it is truncated to "lïf" (3 characters).
                v.record_str(&field, "lïfë");
            }
            assert_eq!(s.as_str(), "question=\"lïf\"");
        });
    }

    #[test]
    fn max_len_zero_records_empty_strings() {
        let make = Truncated::new(MakeDebug, 0);

        TestAttrs1::with(|attrs| {
            let field = attrs.fields().field("question").unwrap();
            let mut s = String::new();
            {
                let mut v = make.make_visitor(&mut s);
                v.record_str(&field, "life, the universe, and everything");
            }
            assert_eq!(s.as_str(), "question=\"\"");
        });
    }

    #[test]
    fn appends_indicator_to_truncated_values() {
        let make = Truncated::new(MakeDebug, 4).with_indicator("...");

        TestAttrs1::with(|attrs| {
            let mut s = String::new();
            {
                let mut v = make.make_visitor(&mut s);
                attrs.record(&mut v);
            }
            // Only the truncated "question" value gets the indicator.
            assert_eq!(
                s.as_str(),
                "question=\"life...\"tricky=truecan_you_do_it=true"
            );
        });
    }

    #[test]
    fn make_ext_truncated_combinator() {
        use crate::field::MakeExt;
        // Going through the MakeExt convenience wrapper must produce the same
        // truncation as constructing Truncated::new directly.
        let make = MakeDebug.truncated(4);

        TestAttrs1::with(|attrs| {
            let mut s = String::new();
            {
                let mut v = make.make_visitor(&mut s);
                attrs.record(&mut v);
            }
            assert_eq!(s.as_str(), "question=\"life\"tricky=truecan_you_do_it=true");
        });
    }
}
