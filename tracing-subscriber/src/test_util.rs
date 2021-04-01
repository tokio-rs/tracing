use crate::field::{MakeVisitor, VisitFmt, VisitOutput};
use core::fmt;
use tracing::field::Visit;
use tracing::span::Attributes;
use tracing_core::{
    callsite::Callsite,
    field::{Field, Value},
    metadata::{Kind, Level, Metadata},
};

pub(crate) struct TestAttrs1;
pub(crate) struct TestAttrs2;

impl TestAttrs1 {
    pub(crate) fn with<T>(f: impl FnOnce(Attributes<'_>) -> T) -> T {
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
    pub(crate) fn with<T>(f: impl FnOnce(Attributes<'_>) -> T) -> T {
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
    fn set_interest(&self, _: tracing_core::collect::Interest) {
        unimplemented!()
    }

    fn metadata(&self) -> &Metadata<'_> {
        &TEST_META_1
    }
}

pub(crate) struct MakeDebug;
pub(crate) struct DebugVisitor<'a> {
    writer: &'a mut dyn fmt::Write,
    err: fmt::Result,
}

impl<'a> DebugVisitor<'a> {
    pub(crate) fn new(writer: &'a mut dyn fmt::Write) -> Self {
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
