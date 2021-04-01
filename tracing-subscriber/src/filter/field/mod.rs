use std::error::Error;
use std::fmt::{Debug, Write};
use std::sync::Arc;

use matcher::FieldMatcher;
use tracing_core::Field;

use crate::field::{MakeVisitor, Visit, VisitFmt, VisitOutput};

pub(crate) mod matcher;

type MatchersVec = Arc<Vec<Box<dyn FieldMatcher>>>;

#[derive(Debug)]
pub struct FieldFilter<MakeInner> {
    inner: MakeInner,
    allow: Option<MatchersVec>,
    deny: MatchersVec,
}

impl<T> FieldFilter<T> {
    pub fn new(
        inner: T,
        allow: Option<Vec<Box<dyn FieldMatcher>>>,
        deny: Vec<Box<dyn FieldMatcher>>,
    ) -> Self {
        let allow = allow.map(Arc::new);
        let deny = Arc::new(deny);

        Self { inner, allow, deny }
    }
}

impl<T, M: MakeVisitor<T>> MakeVisitor<T> for FieldFilter<M> {
    type Visitor = VisitFiltered<M::Visitor>;

    #[inline]
    fn make_visitor(&self, target: T) -> Self::Visitor {
        VisitFiltered::new(
            self.inner.make_visitor(target),
            self.allow.clone(),
            self.deny.clone(),
        )
    }
}

#[derive(Debug)]
pub struct VisitFiltered<V> {
    inner: V,
    allow: Option<MatchersVec>,
    deny: MatchersVec,
}

impl<V> VisitFiltered<V> {
    pub fn new(inner: V, allow: Option<MatchersVec>, deny: MatchersVec) -> Self {
        Self { inner, allow, deny }
    }

    fn should_record(&self, field: &Field) -> bool {
        // Allow if any match in allowlist or no matches in denylist

        if let Some(allow) = &self.allow {
            if allow.iter().any(|m| m.matches_field(field)) {
                return true;
            }
        }

        !self.deny.iter().any(|m| m.matches_field(field))
    }
}

impl<Inner: Visit> Visit for VisitFiltered<Inner> {
    fn record_i64(&mut self, field: &Field, value: i64) {
        if self.should_record(field) {
            self.inner.record_i64(field, value)
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if self.should_record(field) {
            self.inner.record_u64(field, value)
        }
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        if self.should_record(field) {
            self.inner.record_bool(field, value)
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if self.should_record(field) {
            self.inner.record_str(field, value)
        }
    }

    fn record_error(&mut self, field: &Field, value: &(dyn Error + 'static)) {
        if self.should_record(field) {
            self.inner.record_error(field, value)
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if self.should_record(field) {
            self.inner.record_debug(field, value);
        }
    }
}

impl<O, T: VisitOutput<O>> VisitOutput<O> for VisitFiltered<T> {
    fn finish(self) -> O {
        self.inner.finish()
    }
}

impl<T: VisitFmt> VisitFmt for VisitFiltered<T> {
    fn writer(&mut self) -> &mut dyn Write {
        self.inner.writer()
    }
}

#[cfg(test)]
mod tests {
    use crate::field::MakeVisitor;
    use crate::fmt::format::DefaultFields;
    use crate::test_util::{DebugVisitor, MakeDebug, TestAttrs1};

    use super::*;
    use super::matcher::ExactFieldMatcher;

    #[test]
    fn visitor_denylist_works() {
        let mut out = String::new();
        let mut inner = DebugVisitor::new(&mut out);

        let deny: Vec<Box<dyn FieldMatcher>> = vec![
            Box::new(ExactFieldMatcher::new("question".to_string())),
            Box::new(ExactFieldMatcher::new("can_you_do_it".to_string()))
        ];
        let mut visitor = VisitFiltered::new(inner, None, Arc::new(deny));

        TestAttrs1::with(|attrs| attrs.record(&mut visitor));
        visitor.finish().unwrap();

        assert_eq!(out, "tricky=true");
    }

    #[test]
    fn allowlist_overrides_denylist() {
        let mut out = String::new();
        let mut inner = DebugVisitor::new(&mut out);

        let deny: Vec<Box<dyn FieldMatcher>> = vec![
            Box::new(ExactFieldMatcher::new("question".to_string())),
            Box::new(ExactFieldMatcher::new("can_you_do_it".to_string())),
            Box::new(ExactFieldMatcher::new("tricky".to_string())),
        ];
        let allow: Vec<Box<dyn FieldMatcher>> = vec![
            Box::new(ExactFieldMatcher::new("can_you_do_it".to_string()))
        ];
        let mut visitor = VisitFiltered::new(inner, Some(Arc::new(allow)), Arc::new(deny));

        TestAttrs1::with(|attrs| attrs.record(&mut visitor));
        visitor.finish().unwrap();

        assert_eq!(out, "can_you_do_it=true");
    }
}
