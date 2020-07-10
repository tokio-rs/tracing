use tracing::subscriber::with_default;
use tracing_attributes::instrument;
use tracing_test::*;

#[instrument]
fn default_target() {}

#[instrument(target = "my_target")]
fn custom_target() {}

mod my_mod {
    use tracing_attributes::instrument;

    pub const MODULE_PATH: &str = module_path!();

    #[instrument]
    pub fn default_target() {}

    #[instrument(target = "my_other_target")]
    pub fn custom_target() {}
}

#[test]
fn default_targets() {
    let (subscriber, handle) = subscriber::expect()
        .new_span(span("default_target").with_target(module_path!()))
        .enter(span("default_target").with_target(module_path!()))
        .exit(span("default_target").with_target(module_path!()))
        .new_span(span("default_target").with_target(my_mod::MODULE_PATH))
        .enter(span("default_target").with_target(my_mod::MODULE_PATH))
        .exit(span("default_target").with_target(my_mod::MODULE_PATH))
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        default_target();
        my_mod::default_target();
    });

    handle.assert_finished();
}

#[test]
fn custom_targets() {
    let (subscriber, handle) = subscriber::expect()
        .new_span(span("custom_target").with_target("my_target"))
        .enter(span("custom_target").with_target("my_target"))
        .exit(span("custom_target").with_target("my_target"))
        .new_span(span("custom_target").with_target("my_other_target"))
        .enter(span("custom_target").with_target("my_other_target"))
        .exit(span("custom_target").with_target("my_other_target"))
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        custom_target();
        my_mod::custom_target();
    });

    handle.assert_finished();
}
