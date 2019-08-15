//! Utilities for working with [fields] and [field visitors].
//!
//! [field]: https://docs.rs/tracing-core/latest/tracing_core/field/index.html
//! [field visitors]: https://docs.rs/tracing-core/latest/tracing_core/field/trait.Visit.html
use std::{fmt, io};
use tracing_core::{
    field::{Field, Visit},
    span::{Attributes, Record},
    Event,
};

pub mod delimited;

/// Creates new [visitors].
///
/// A type implementing `MakeVisitor` represents a composable factory for types
/// implementing the [`Visit` trait][visitors]. The `MakeVisitor` trait defines
/// a single function, `make_visitor`, which takes in a `T`-typed `target` and
/// returns a type implementing `Visit` configured for that target. A target may
/// be a string, output stream, or data structure that the visitor will record
/// data to, configuration variables that determine the visitor's behavior, or
/// `()` when no input is required to produce a visitor.
///
/// [visitors]: https://docs.rs/tracing-core/latest/tracing_core/field/trait.Visit.html
pub trait MakeVisitor<T> {
    type Visitor: Visit;

    /// Make a new visitor for the provided `target`.
    fn make_visitor(&self, target: T) -> Self::Visitor;
}

/// A [visitor] that produces output once it has visited a set of fields.
///
/// [visitor]: https://docs.rs/tracing-core/latest/tracing_core/field/trait.Visit.html
pub trait VisitOutput<Out>: Visit {
    /// Completes the visitor, returning any output.
    ///
    /// This is called once a full set of fields has been visited.
    fn finish(self) -> Out;

    /// Visit a set of fields, and return the output of finishing the visitor
    /// once the fields have been visited.
    fn visit<R>(mut self, fields: &R) -> Out
    where
        R: RecordFields,
        Self: Sized,
    {
        fields.record(&mut self);
        self.finish()
    }
}

/// Extension trait implemented by types which can be recorded by a [visitor].
///
/// This allows writing code that is generic over `tracing_core`'s
/// [`span::Attributes`], [`span::Record`], and [`Event`] types. These types all
/// provide inherent `record` methods that allow a visitor to record their
/// fields, but there is no common trait representing this.
///
/// With `RecordFields`, we can write code like this:
/// ```
/// use tracing_core::field::Visit;
/// # use tracing_core::field::Field;
/// use tracing_subscriber::field::RecordFields;
///
/// struct MyVisitor {
///     // ...
/// }
/// # impl MyVisitor { fn new() -> Self { Self{} } }
/// impl Visit for MyVisitor {
///     // ...
/// # fn record_debug(&mut self, _: &Field, _: &dyn std::fmt::Debug) {}
/// }
///
/// fn record_with_my_visitor<R>(r: R)
/// where
///     R: RecordFields,
/// {
///     let mut visitor = MyVisitor::new();
///     r.record(&mut visitor);
/// }
/// # fn main() {}
/// ```
pub trait RecordFields: crate::sealed::Sealed<RecordFieldsMarker> {
    fn record(&self, visitor: &mut dyn Visit);
}

/// Extension trait implemented for all `MakeVisitor` implementations that
/// produce a visitor implementing `VisitOutput`.
pub trait MakeOutput<T, Out>
where
    Self: MakeVisitor<T> + crate::sealed::Sealed<(T, Out)>,
    Self::Visitor: VisitOutput<Out>,
{
    /// Visits all fields in `fields` with a new visitor constructed from
    /// `target`.
    fn visit_with<F>(&self, target: T, fields: &F) -> Out
    where
        F: RecordFields,
    {
        self.make_visitor(target).visit(fields)
    }
}

/// Extension trait implemented by visitors to indicate that they write to an
/// `io::Write` instance, and allow access to that writer.
pub trait VisitWrite: VisitOutput<Result<(), io::Error>> {
    fn writer(&mut self) -> &mut dyn io::Write;
}
/// Extension trait implemented by visitors to indicate that they write to a
/// `fmt::Write` instance, and allow access to that writer.
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

// === blanket impls ===

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
        metadata::{Kind, Level, Metadata},
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
