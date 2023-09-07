use tracing::collect::with_default;
use tracing::Level;
use tracing_attributes::instrument;
use tracing_mock::*;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::subscribe::CollectExt;

use std::convert::TryFrom;
use std::num::TryFromIntError;

#[instrument(err)]
fn err() -> Result<u8, TryFromIntError> {
    u8::try_from(1234)
}

#[instrument(err)]
fn err_suspicious_else() -> Result<u8, TryFromIntError> {
    {}
    u8::try_from(1234)
}

#[test]
fn test() {
    let span = expect::span().named("err");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(expect::event().at_level(Level::ERROR))
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();
    with_default(collector, || err().ok());
    handle.assert_finished();
}

#[instrument(err)]
fn err_early_return() -> Result<u8, TryFromIntError> {
    u8::try_from(1234)?;
    Ok(5)
}

#[test]
fn test_early_return() {
    let span = expect::span().named("err_early_return");
    let (subscriber, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(expect::event().at_level(Level::ERROR))
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();
    with_default(subscriber, || err_early_return().ok());
    handle.assert_finished();
}

#[instrument(err)]
async fn err_async(polls: usize) -> Result<u8, TryFromIntError> {
    let future = PollN::new_ok(polls);
    tracing::trace!(awaiting = true);
    future.await.ok();
    u8::try_from(1234)
}

#[test]
fn test_async() {
    let span = expect::span().named("err_async");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            expect::event()
                .with_fields(expect::field("awaiting").with_value(&true))
                .at_level(Level::TRACE),
        )
        .exit(span.clone())
        .enter(span.clone())
        .event(expect::event().at_level(Level::ERROR))
        .exit(span.clone())
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();
    with_default(collector, || {
        block_on_future(async { err_async(2).await }).ok();
    });
    handle.assert_finished();
}

#[instrument(err)]
fn err_mut(out: &mut u8) -> Result<(), TryFromIntError> {
    *out = u8::try_from(1234)?;
    Ok(())
}

#[test]
fn test_mut() {
    let span = expect::span().named("err_mut");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(expect::event().at_level(Level::ERROR))
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();
    with_default(collector, || err_mut(&mut 0).ok());
    handle.assert_finished();
}

#[instrument(err)]
async fn err_mut_async(polls: usize, out: &mut u8) -> Result<(), TryFromIntError> {
    let future = PollN::new_ok(polls);
    tracing::trace!(awaiting = true);
    future.await.ok();
    *out = u8::try_from(1234)?;
    Ok(())
}

#[test]
fn test_mut_async() {
    let span = expect::span().named("err_mut_async");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            expect::event()
                .with_fields(expect::field("awaiting").with_value(&true))
                .at_level(Level::TRACE),
        )
        .exit(span.clone())
        .enter(span.clone())
        .event(expect::event().at_level(Level::ERROR))
        .exit(span.clone())
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();
    with_default(collector, || {
        block_on_future(async { err_mut_async(2, &mut 0).await }).ok();
    });
    handle.assert_finished();
}

#[test]
fn impl_trait_return_type() {
    // Reproduces https://github.com/tokio-rs/tracing/issues/1227

    #[instrument(err, fields(?x))]
    fn returns_impl_trait(x: usize) -> Result<impl Iterator<Item = usize>, String> {
        Ok(0..x)
    }

    let span = expect::span().named("returns_impl_trait");

    let (collector, handle) = collector::mock()
        .new_span(
            span.clone().with_field(
                expect::field("x")
                    .with_value(&tracing::field::debug(10usize))
                    .only(),
            ),
        )
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    with_default(collector, || {
        for _ in returns_impl_trait(10).unwrap() {
            // nop
        }
    });

    handle.assert_finished();
}

#[instrument(err(Debug))]
fn err_dbg() -> Result<u8, TryFromIntError> {
    u8::try_from(1234)
}

#[test]
fn test_err_dbg() {
    let span = expect::span().named("err_dbg");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            expect::event().at_level(Level::ERROR).with_fields(
                expect::field("error")
                    // use the actual error value that will be emitted, so
                    // that this test doesn't break if the standard library
                    // changes the `fmt::Debug` output from the error type
                    // in the future.
                    .with_value(&tracing::field::debug(u8::try_from(1234).unwrap_err())),
            ),
        )
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();
    with_default(collector, || err_dbg().ok());
    handle.assert_finished();
}

#[test]
fn test_err_display_default() {
    let span = expect::span().named("err");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            expect::event().at_level(Level::ERROR).with_fields(
                expect::field("error")
                    // by default, errors will be emitted with their display values
                    .with_value(&tracing::field::display(u8::try_from(1234).unwrap_err())),
            ),
        )
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();
    with_default(collector, || err().ok());
    handle.assert_finished();
}

#[test]
fn test_err_custom_target() {
    let filter: EnvFilter = "my_target=error".parse().expect("filter should parse");
    let span = expect::span().named("error_span").with_target("my_target");

    let (subscriber, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            expect::event()
                .at_level(Level::ERROR)
                .with_target("my_target"),
        )
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    let subscriber = subscriber.with(filter);

    with_default(subscriber, || {
        let error_span = tracing::error_span!(target: "my_target", "error_span");

        {
            let _enter = error_span.enter();
            tracing::error!(target: "my_target", "This should display")
        }
    });
    handle.assert_finished();
}

#[instrument(err(level = "info"))]
fn err_info() -> Result<u8, TryFromIntError> {
    u8::try_from(1234)
}

#[test]
fn test_err_info() {
    let span = expect::span().named("err_info");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(expect::event().at_level(Level::INFO))
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();
    with_default(collector, || err_info().ok());
    handle.assert_finished();
}

#[instrument(err(Debug, level = "info"))]
fn err_dbg_info() -> Result<u8, TryFromIntError> {
    u8::try_from(1234)
}

#[test]
fn test_err_dbg_info() {
    let span = expect::span().named("err_dbg_info");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            expect::event().at_level(Level::INFO).with_fields(
                expect::field("error")
                    // use the actual error value that will be emitted, so
                    // that this test doesn't break if the standard library
                    // changes the `fmt::Debug` output from the error type
                    // in the future.
                    .with_value(&tracing::field::debug(u8::try_from(1234).unwrap_err())),
            ),
        )
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();
    with_default(collector, || err_dbg_info().ok());
    handle.assert_finished();
}

#[instrument(level = "warn", err(level = "info"))]
fn err_warn_info() -> Result<u8, TryFromIntError> {
    u8::try_from(1234)
}

#[test]
fn test_err_warn_info() {
    let span = expect::span().named("err_warn_info").at_level(Level::WARN);
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(expect::event().at_level(Level::INFO))
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();
    with_default(collector, || err_warn_info().ok());
    handle.assert_finished();
}
