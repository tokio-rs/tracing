mod support;
use support::*;

use tracing::collect::with_default;
use tracing_attributes::instrument;

#[instrument]
fn default_name() {
    let _ = 1;
}

#[instrument(name = "my_name")]
fn custom_name() {
    let _ = 1;
}

// XXX: it's weird that we support both of these forms, but apparently we
// managed to release a version that accepts both syntax, so now we have to
// support it! yay!
#[instrument("my_other_name")]
fn custom_name_no_equals() {
    let _ = 1;
}

#[test]
fn default_name_test() {
    let (collector, handle) = collector::mock()
        .new_span(span::mock().named("default_name"))
        .enter(span::mock().named("default_name"))
        .exit(span::mock().named("default_name"))
        .done()
        .run_with_handle();

    with_default(collector, || {
        default_name();
    });

    handle.assert_finished();
}

#[test]
fn custom_name_test() {
    let (collector, handle) = collector::mock()
        .new_span(span::mock().named("my_name"))
        .enter(span::mock().named("my_name"))
        .exit(span::mock().named("my_name"))
        .done()
        .run_with_handle();

    with_default(collector, || {
        custom_name();
    });

    handle.assert_finished();
}

#[test]
fn custom_name_no_equals_test() {
    let (collector, handle) = collector::mock()
        .new_span(span::mock().named("my_other_name"))
        .enter(span::mock().named("my_other_name"))
        .exit(span::mock().named("my_other_name"))
        .done()
        .run_with_handle();

    with_default(collector, || {
        custom_name_no_equals();
    });

    handle.assert_finished();
}
