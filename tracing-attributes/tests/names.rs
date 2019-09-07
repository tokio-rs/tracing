mod support;
use support::*;

use tracing::subscriber::with_default;
use tracing_attributes::instrument;

#[instrument]
fn default_name() {}

#[instrument(name = "my_name")]
fn custom_name() {}

#[test]
fn default_name_test() {
    let (subscriber, handle) = subscriber::mock()
        .new_span(span::mock().named("default_name"))
        .enter(span::mock().named("default_name"))
        .exit(span::mock().named("default_name"))
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        default_name();
    });

    handle.assert_finished();
}

#[test]
fn custom_name_test() {
    let (subscriber, handle) = subscriber::mock()
        .new_span(span::mock().named("my_name"))
        .enter(span::mock().named("my_name"))
        .exit(span::mock().named("my_name"))
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        custom_name();
    });

    handle.assert_finished();
}
