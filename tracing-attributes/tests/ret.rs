use std::fmt::Display;

use tracing::{collect::with_default, Level};
use tracing_attributes::instrument;
use tracing_mock::{collector, expect, field::ExpectedFields, span::ExpectedSpan};
use tracing_subscriber::subscribe::CollectExt;
use tracing_subscriber::EnvFilter;
use tracing_test::block_on_future;

fn run_collector(
    span: ExpectedSpan,
    field: Option<ExpectedFields>,
    level: Level,
) -> (impl tracing::Collect, collector::MockHandle) {
    let mut c = collector::mock().new_span(span.clone()).enter(span.clone());
    if let Some(field) = field {
        c = c.event(expect::event().with_fields(field).at_level(level));
    }
    c.exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle()
}

fn expect_return<V: tracing::Value, T>(
    span: &'static str,
    value: Option<V>,
    level: Level,
    func: impl FnOnce() -> T,
) {
    let span = expect::span().named(span);
    let (collector, handle) = run_collector(
        span,
        value.map(|v| expect::field("return").with_value(&v).only()),
        level,
    );

    with_default(collector, func);
    handle.assert_finished();
}

fn expect_err<V: tracing::Value, T>(
    span: &'static str,
    value: Option<V>,
    level: Level,
    func: impl FnOnce() -> T,
) {
    let span = expect::span().named(span);
    let (collector, handle) = run_collector(
        span,
        value.map(|v| expect::field("error").with_value(&v).only()),
        level,
    );

    with_default(collector, func);
    handle.assert_finished();
}

// Helper structs that ensure no incorrect formatting is attempted
#[derive(Clone)]
struct OnlyDisplay(i32);
impl Display for OnlyDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OnlyDisplay({})", self.0)
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // field is only used in Debug impl
struct OnlyDebug(i32);

#[instrument(ret)]
fn ret() -> i32 {
    42
}

#[test]
fn test() {
    expect_return("ret", Some(tracing::field::debug(42)), Level::INFO, ret);
}

#[instrument(target = "my_target", ret)]
fn ret_with_target() -> i32 {
    42
}

#[test]
fn test_custom_target() {
    let filter: EnvFilter = "my_target=info".parse().expect("filter should parse");
    let span = expect::span()
        .named("ret_with_target")
        .with_target("my_target");

    let (subscriber, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            expect::event()
                .with_fields(expect::field("return").with_value(&tracing::field::debug(42)))
                .at_level(Level::INFO)
                .with_target("my_target"),
        )
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    let subscriber = subscriber.with(filter);

    with_default(subscriber, ret_with_target);
    handle.assert_finished();
}

#[instrument(level = "warn", ret)]
fn ret_warn() -> i32 {
    42
}

#[test]
fn test_warn() {
    expect_return(
        "ret_warn",
        Some(tracing::field::debug(42)),
        Level::WARN,
        ret_warn,
    );
}

#[instrument(ret)]
fn ret_mut(a: &mut i32) -> i32 {
    *a *= 2;
    tracing::info!(?a);
    *a
}

#[test]
fn test_mut() {
    let span = expect::span().named("ret_mut");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            expect::event()
                .with_fields(expect::field("a").with_value(&tracing::field::display(2)))
                .at_level(Level::INFO),
        )
        .event(
            expect::event()
                .with_fields(expect::field("return").with_value(&tracing::field::debug(2)))
                .at_level(Level::INFO),
        )
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    with_default(collector, || ret_mut(&mut 1));
    handle.assert_finished();
}

#[instrument(ret)]
async fn ret_async() -> i32 {
    42
}

#[test]
fn test_async() {
    let span = expect::span().named("ret_async");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            expect::event()
                .with_fields(expect::field("return").with_value(&tracing::field::debug(42)))
                .at_level(Level::INFO),
        )
        .exit(span.clone())
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    with_default(collector, || block_on_future(async { ret_async().await }));
    handle.assert_finished();
}

#[instrument(ret)]
fn ret_impl_type() -> impl Copy {
    42
}

#[test]
fn test_impl_type() {
    expect_return(
        "ret_impl_type",
        Some(tracing::field::debug(42)),
        Level::INFO,
        ret_impl_type,
    );
}

#[instrument(ret(Display))]
fn ret_display() -> i32 {
    42
}

#[test]
fn test_dbg() {
    expect_return(
        "ret_display",
        Some(tracing::field::display(42)),
        Level::INFO,
        ret_display,
    );
}

#[instrument(skip(x), ret)]
fn ret_result(x: Result<OnlyDebug, OnlyDebug>) -> Result<OnlyDebug, OnlyDebug> {
    x
}

#[test]
fn test_ret_result() {
    let err = Err(OnlyDebug(42));
    expect_return(
        "ret_result",
        Some(tracing::field::debug(err.clone())),
        Level::INFO,
        || ret_result(err).ok(),
    );

    let ok = Ok(OnlyDebug(42));
    expect_return(
        "ret_result",
        Some(tracing::field::debug(ok.clone())),
        Level::INFO,
        || ret_result(ok).ok(),
    );
}

#[instrument(skip(x), ret, err)]
fn ret_err(x: Result<OnlyDebug, OnlyDisplay>) -> Result<OnlyDebug, OnlyDisplay> {
    x
}

#[test]
fn test_ret_err() {
    let err = OnlyDisplay(42);
    expect_err(
        "ret_err",
        Some(tracing::field::display(err.clone())),
        Level::ERROR,
        || ret_err(Err(err)).ok(),
    );

    let ok = OnlyDebug(42);
    expect_return(
        "ret_err",
        Some(tracing::field::debug(ok.clone())),
        Level::INFO,
        || ret_err(Ok(ok)).ok(),
    );
}

#[instrument(level = "warn", ret(level = "info"))]
fn ret_warn_info() -> i32 {
    42
}

#[test]
fn test_warn_info() {
    expect_return(
        "ret_warn_info",
        Some(tracing::field::debug(42)),
        Level::INFO,
        ret_warn_info,
    );
}

#[instrument(ret(level = "warn", Debug))]
fn ret_dbg_warn() -> i32 {
    42
}

#[test]
fn test_dbg_warn() {
    expect_return(
        "ret_dbg_warn",
        Some(tracing::field::debug(42)),
        Level::WARN,
        ret_dbg_warn,
    );
}
