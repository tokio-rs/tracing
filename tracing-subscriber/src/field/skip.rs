use super::{MakeVisitor, VisitFmt, VisitOutput, VisitWrite};
use std::collections::HashSet;
use tracing_core::field::{Field, Visit};

#[derive(Debug, Clone)]
pub struct Skip<V, F = fn(&Field) -> bool> {
    inner: V,
    predicate: F,
}

pub trait Predicate {
    fn skip(&self, field: &Field) -> bool;
}

#[derive(Debug, Clone)]
pub struct StartsWith(&'static str);

// === impl Skip ===

impl<V, F> Skip<V, F> {
    pub fn new(inner: V, predicate: F) -> Self {
        Self { inner, predicate }
    }
}

impl<T, V, F> MakeVisitor<T> for Skip<V, F>
where
    V: MakeVisitor<T>,
    F: Predicate,
    F: Clone,
{
    type Visitor = Skip<V::Visitor, F>;

    #[inline]
    fn make_visitor(&self, target: T) -> Self::Visitor {
        Skip::new(self.inner.make_visitor(target), self.predicate.clone())
    }
}

impl<V, F> Visit for Skip<V, F>
where
    V: Visit,
    F: Predicate,
{
    #[inline]
    fn record_i64(&mut self, field: &Field, value: i64) {
        if self.predicate.skip(field) {
            return;
        }

        self.inner.record_i64(field, value)
    }

    #[inline]
    fn record_u64(&mut self, field: &Field, value: u64) {
        if self.predicate.skip(field) {
            return;
        }

        self.inner.record_u64(field, value)
    }

    #[inline]
    fn record_bool(&mut self, field: &Field, value: bool) {
        if self.predicate.skip(field) {
            return;
        }

        self.inner.record_bool(field, value)
    }

    /// Visit a string value.
    fn record_str(&mut self, field: &Field, value: &str) {
        if self.predicate.skip(field) {
            return;
        }

        self.inner.record_str(field, value)
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        if self.predicate.skip(field) {
            return;
        }
        self.inner.record_error(field, value)
    }

    #[inline]
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.inner
            .record_debug(field, &format_args!("{:#?}", value))
    }
}

// impl<V, O> VisitOutput<O> for Alt<V>
// where
//     V: VisitOutput<O>,
// {
//     #[inline]
//     fn finish(self) -> O {
//         self.0.finish()
//     }
// }

// impl<V> VisitWrite for Alt<V>
// where
//     V: VisitWrite,
// {
//     #[inline]
//     fn writer(&mut self) -> &mut dyn io::Write {
//         self.0.writer()
//     }
// }

// impl<V> VisitFmt for Alt<V>
// where
//     V: VisitFmt,
// {
//     #[inline]
//     fn writer(&mut self) -> &mut dyn fmt::Write {
//         self.0.writer()
//     }
// }

impl<F> Predicate for F
where
    F: Fn(&Field) -> bool,
{
    fn skip(&self, field: &Field) -> bool {
        (self)(field)
    }
}

impl Predicate for HashSet<Field> {
    fn skip(&self, field: &Field) -> bool {
        self.contains(field)
    }
}

impl Predicate for HashSet<String> {
    fn skip(&self, field: &Field) -> bool {
        self.contains(field.name())
    }
}

impl Predicate for Prefix {
    fn skip(&self)
}