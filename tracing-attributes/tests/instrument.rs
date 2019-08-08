mod support;
use support::*;

use tracing::subscriber::with_default;
use tracing::Level;
use tracing_attributes::instrument;

#[test]
fn override_everything() {
    #[instrument(target = "my_target", level = "debug")]
    fn my_fn() {}
    let span = span::mock()
        .named("my_fn")
        .at_level(Level::DEBUG)
        .with_target("my_target");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        my_fn();
    });

    handle.assert_finished();
}
