// Only test on nightly, since UI tests are bound to change over time
#[rustversion::nightly]
#[test]
fn async_instrument() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/async_instrument.rs");
}
