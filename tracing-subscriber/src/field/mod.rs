use std::{fmt, io};
use tracing_core::{
    field::{Field, Visit},
    span::{Attributes, Record},
    Event,
};

pub mod delimited;

pub trait MakeVisitor<T> {
    type Visitor: Visit;

    fn make_visitor(&self, target: T) -> Self::Visitor;
}

pub trait VisitOutput<Out>: Visit {
    fn finish(self) -> Out;

    fn visit<R>(mut self, fields: &R) -> Out
    where
        R: RecordFields,
        Self: Sized,
    {
        fields.record(&mut self);
        self.finish()
    }
}

pub trait RecordFields: crate::sealed::Sealed<RecordFieldsMarker> {
    fn record(&self, visitor: &mut dyn Visit);
}

pub trait MakeOutput<T, Out>
where
    Self: MakeVisitor<T> + crate::sealed::Sealed<(T, Out)>,
    Self::Visitor: VisitOutput<Out>,
{
    // fn delimited<D>(&self, delimiter: D) -> Delimited<D, Self>
    // where
    //     Self: Clone + Sized,
    //     VisitDelimited<D, Self::Visitor>: Visit,
    //     D: Clone,
    // {
    //     Delimited {
    //         delimiter,
    //         inner: self.clone(),
    //     }
    // }

    fn visit_with<F>(&self, target: T, fields: &F) -> Out
    where
        F: RecordFields,
    {
        let mut v = self.make_visitor(target);
        fields.record(&mut v);
        v.finish()
    }
}

pub trait VisitWrite: VisitOutput<Result<(), io::Error>> {
    fn writer(&mut self) -> &mut dyn io::Write;
}

pub trait VisitFmt: VisitOutput<fmt::Result> {
    fn writer(&mut self) -> &mut dyn fmt::Write;
}

// === impl RecordFields ===

impl<'a> crate::sealed::Sealed<RecordFieldsMarker> for Event<'a> {}
impl<'a> RecordFields for Event<'a> {
    fn record(&self, visitor: &mut dyn Visit) {
        Event::record(&self, visitor)
    }
}

impl<'a> crate::sealed::Sealed<RecordFieldsMarker> for Attributes<'a> {}
impl<'a> RecordFields for Attributes<'a> {
    fn record(&self, visitor: &mut dyn Visit) {
        Attributes::record(&self, visitor)
    }
}

impl<'a> crate::sealed::Sealed<RecordFieldsMarker> for Record<'a> {}
impl<'a> RecordFields for Record<'a> {
    fn record(&self, visitor: &mut dyn Visit) {
        Record::record(&self, visitor)
    }
}

impl<T, V, F> MakeVisitor<T> for F
where
    F: Fn(T) -> V,
    V: Visit,
{
    type Visitor = V;
    fn make_visitor(&self, target: T) -> Self::Visitor {
        (self)(target)
    }
}

impl<T, Out, M> crate::sealed::Sealed<(T, Out)> for M
where
    M: MakeVisitor<T>,
    M::Visitor: VisitOutput<Out>,
{
}

impl<T, Out, M> MakeOutput<T, Out> for M
where
    M: MakeVisitor<T>,
    M::Visitor: VisitOutput<Out>,
{
}

#[doc(hidden)]
pub struct RecordFieldsMarker {
    _p: (),
}

#[cfg(test)]
#[macro_use]
pub(in crate::field) mod test_util {
    use super::*;
    use tracing_core::{
        callsite::Callsite,
        field::Value,
        metadata::{self, Kind, Level, Metadata},
    };

    pub struct TestAttrs1;
    pub struct TestAttrs2;

    impl TestAttrs1 {
        pub fn with<T>(f: impl FnOnce(Attributes) -> T) -> T {
            let fieldset = TEST_META_1.fields();
            let values = &[
                (
                    &fieldset.field("question").unwrap(),
                    Some(&"life, the universe, and everything" as &dyn Value),
                ),
                (&fieldset.field("question.answer").unwrap(), None),
                (
                    &fieldset.field("tricky").unwrap(),
                    Some(&true as &dyn Value),
                ),
                (
                    &fieldset.field("can_you_do_it").unwrap(),
                    Some(&true as &dyn Value),
                ),
            ];
            let valueset = fieldset.value_set(values);
            let attrs = tracing_core::span::Attributes::new(&TEST_META_1, &valueset);
            f(attrs)
        }
    }

    impl TestAttrs2 {
        pub fn with<T>(f: impl FnOnce(Attributes) -> T) -> T {
            let fieldset = TEST_META_1.fields();
            let none = tracing_core::field::debug(&Option::<&str>::None);
            let values = &[
                (
                    &fieldset.field("question").unwrap(),
                    Some(&none as &dyn Value),
                ),
                (
                    &fieldset.field("question.answer").unwrap(),
                    Some(&42 as &dyn Value),
                ),
                (
                    &fieldset.field("tricky").unwrap(),
                    Some(&true as &dyn Value),
                ),
                (
                    &fieldset.field("can_you_do_it").unwrap(),
                    Some(&false as &dyn Value),
                ),
            ];
            let valueset = fieldset.value_set(values);
            let attrs = tracing_core::span::Attributes::new(&TEST_META_1, &valueset);
            f(attrs)
        }
    }

    struct TestCallsite1;
    static TEST_CALLSITE_1: &'static dyn Callsite = &TestCallsite1;
    static TEST_META_1: Metadata<'static> = tracing_core::metadata! {
        name: "field_test1",
        target: module_path!(),
        level: Level::INFO,
        fields: &["question", "question.answer", "tricky", "can_you_do_it"],
        callsite: TEST_CALLSITE_1,
        kind: Kind::SPAN,
    };

    impl Callsite for TestCallsite1 {
        fn set_interest(&self, _: tracing_core::subscriber::Interest) {
            unimplemented!()
        }

        fn metadata(&self) -> &Metadata {
            &TEST_META_1
        }
    }

    pub struct MakeDebug;
    pub struct DebugVisitor<'a> {
        writer: &'a mut dyn fmt::Write,
        err: fmt::Result,
    }

    impl<'a> DebugVisitor<'a> {
        pub fn new(writer: &'a mut dyn fmt::Write) -> Self {
            Self {
                writer,
                err: Ok(()),
            }
        }
    }

    impl<'a> Visit for DebugVisitor<'a> {
        fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
            write!(&mut self.writer, "{}={:?}", field, value).unwrap();
        }
    }

    impl<'a> VisitOutput<fmt::Result> for DebugVisitor<'a> {
        fn finish(self) -> fmt::Result {
            self.err
        }
    }

    impl<'a> VisitFmt for DebugVisitor<'a> {
        fn writer(&mut self) -> &mut dyn fmt::Write {
            self.writer
        }
    }

    impl<'a> MakeVisitor<&'a mut dyn fmt::Write> for MakeDebug {
        type Visitor = DebugVisitor<'a>;
        fn make_visitor(&self, w: &'a mut dyn fmt::Write) -> DebugVisitor<'a> {
            DebugVisitor::new(w)
        }
    }
}
