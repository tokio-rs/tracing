use tracing::collect::with_default;
use tracing_attributes::instrument;
use tracing_mock::*;

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
    let (collector, handle) = collector::mock()
        .new_span(
            span::expect()
                .named("default_target")
                .with_target(module_path!()),
        )
        .enter(
            span::expect()
                .named("default_target")
                .with_target(module_path!()),
        )
        .exit(
            span::expect()
                .named("default_target")
                .with_target(module_path!()),
        )
        .new_span(
            span::expect()
                .named("default_target")
                .with_target(my_mod::MODULE_PATH),
        )
        .enter(
            span::expect()
                .named("default_target")
                .with_target(my_mod::MODULE_PATH),
        )
        .exit(
            span::expect()
                .named("default_target")
                .with_target(my_mod::MODULE_PATH),
        )
        .only()
        .run_with_handle();

    with_default(collector, || {
        default_target();
        my_mod::default_target();
    });

    handle.assert_finished();
}

#[test]
fn custom_targets() {
    let (collector, handle) = collector::mock()
        .new_span(
            span::expect()
                .named("custom_target")
                .with_target("my_target"),
        )
        .enter(
            span::expect()
                .named("custom_target")
                .with_target("my_target"),
        )
        .exit(
            span::expect()
                .named("custom_target")
                .with_target("my_target"),
        )
        .new_span(
            span::expect()
                .named("custom_target")
                .with_target("my_other_target"),
        )
        .enter(
            span::expect()
                .named("custom_target")
                .with_target("my_other_target"),
        )
        .exit(
            span::expect()
                .named("custom_target")
                .with_target("my_other_target"),
        )
        .only()
        .run_with_handle();

    with_default(collector, || {
        custom_target();
        my_mod::custom_target();
    });

    handle.assert_finished();
}
