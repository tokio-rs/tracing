use core::fmt;

use tracing_core::{field::Visit, Field};

use super::{MakeVisitor, VisitFmt, VisitOutput};

/// A visitor wrapper that filter fields on the wrapped visitor
#[derive(Debug, Clone)]
pub struct FieldFilter<F, V> {
    inner: V,
    filter_fn: F,
}

impl<T, F, V> MakeVisitor<T> for FieldFilter<F, V>
where
    F: Fn(&Field) -> bool,
    F: Clone,
    V: MakeVisitor<T>,
{
    type Visitor = FieldFilter<F, V::Visitor>;
    fn make_visitor(&self, target: T) -> Self::Visitor {
        let inner = self.inner.make_visitor(target);
        FieldFilter::new(self.filter_fn.clone(), inner)
    }
}

impl<F, V> FieldFilter<F, V> {
    /// Returns a new [`Visit`] implementation that wraps `inner` so that
    /// each formatted field is separated by the provided `filter_fn`.
    ///
    /// [`Visit`]: tracing_core::field::Visit
    pub fn new(filter_fn: F, inner: V) -> Self {
        Self { inner, filter_fn }
    }
}

impl<F, V> Visit for FieldFilter<F, V>
where
    V: Visit,
    F: Fn(&Field) -> bool,
{
    fn record_i64(&mut self, field: &Field, value: i64) {
        if (self.filter_fn)(field) {
            self.inner.record_i64(field, value);
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if (self.filter_fn)(field) {
            self.inner.record_u64(field, value);
        }
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        if (self.filter_fn)(field) {
            self.inner.record_bool(field, value);
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if (self.filter_fn)(field) {
            self.inner.record_str(field, value);
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if (self.filter_fn)(field) {
            self.inner.record_debug(field, value);
        }
    }
}

impl<F, V> VisitOutput<fmt::Result> for FieldFilter<F, V>
where
    V: VisitFmt,
    F: Fn(&Field) -> bool,
{
    fn finish(self) -> fmt::Result {
        self.inner.finish()
    }
}

impl<F, V> VisitFmt for FieldFilter<F, V>
where
    V: VisitFmt,
    F: Fn(&Field) -> bool,
{
    fn writer(&mut self) -> &mut dyn fmt::Write {
        self.inner.writer()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::{delimited::VisitDelimited, test_util::*};

    #[test]
    fn filter_field_visitor() {
        let mut s = String::new();
        let visitor = DebugVisitor::new(&mut s);
        let mut visitor = FieldFilter::new(|field: &Field| field.name() == "question", visitor);

        TestAttrs1::with(|attrs| attrs.record(&mut visitor));

        assert_eq!(
            s.as_str(),
            "question=\"life, the universe, and everything\""
        );

        let mut s = String::new();
        let visitor = DebugVisitor::new(&mut s);
        let mut visitor = FieldFilter::new(
            |field: &Field| field.name() == "question" || field.name() == "can_you_do_it",
            VisitDelimited::new(", ", visitor),
        );

        TestAttrs1::with(|attrs| attrs.record(&mut visitor));
        visitor.finish().unwrap();

        assert_eq!(
            s.as_str(),
            "question=\"life, the universe, and everything\", can_you_do_it=true"
        );
    }
}
