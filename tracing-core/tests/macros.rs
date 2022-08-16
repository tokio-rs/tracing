use tracing_core::{
    callsite::Callsite,
    collect::Interest,
    metadata,
    metadata::{Kind, Level, Metadata},
};
use valuable::NamedField;

const FOO: NamedField = NamedField::new("foo");
const BAR: NamedField = NamedField::new("bar");
const BAZ: NamedField = NamedField::new("baz");

#[test]
fn metadata_macro_api() {
    // This test should catch any inadvertent breaking changes
    // caused by changes to the macro.
    struct TestCallsite;

    impl Callsite for TestCallsite {
        fn set_interest(&self, _: Interest) {
            unimplemented!("test")
        }
        fn metadata(&self) -> &Metadata<'_> {
            unimplemented!("test")
        }
    }

    static CALLSITE: TestCallsite = TestCallsite;
    let _metadata = metadata! {
        name: "test_metadata",
        target: "test_target",
        level: Level::DEBUG,
        fields: &[FOO, BAR, BAZ],
        callsite: &CALLSITE,
        kind: Kind::SPAN,
    };
    let _metadata = metadata! {
        name: "test_metadata",
        target: "test_target",
        level: Level::TRACE,
        fields: &[],
        callsite: &CALLSITE,
        kind: Kind::EVENT,
    };
    let _metadata = metadata! {
        name: "test_metadata",
        target: "test_target",
        level: Level::INFO,
        fields: &[],
        callsite: &CALLSITE,
        kind: Kind::EVENT
    };
}
