// Only test on stable, since UI tests are bound to change over time

#[rustversion::stable]
#[test]
fn trybuild() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
