use super::*;
use tracing_mock::subscriber::MockSubscriber;

#[test]
fn filters_span_scopes() {
    let (debug_subscriber, debug_handle) = subscriber::named("debug")
        .enter(expect::span().at_level(Level::DEBUG))
        .enter(expect::span().at_level(Level::INFO))
        .enter(expect::span().at_level(Level::WARN))
        .enter(expect::span().at_level(Level::ERROR))
        .event(
            expect::event()
                .with_fields(expect::msg("hello world"))
                .in_scope(vec![
                    expect::span().at_level(Level::ERROR),
                    expect::span().at_level(Level::WARN),
                    expect::span().at_level(Level::INFO),
                    expect::span().at_level(Level::DEBUG),
                ]),
        )
        .exit(expect::span().at_level(Level::ERROR))
        .exit(expect::span().at_level(Level::WARN))
        .exit(expect::span().at_level(Level::INFO))
        .exit(expect::span().at_level(Level::DEBUG))
        .only()
        .run_with_handle();
    let (info_subscriber, info_handle) = subscriber::named("info")
        .enter(expect::span().at_level(Level::INFO))
        .enter(expect::span().at_level(Level::WARN))
        .enter(expect::span().at_level(Level::ERROR))
        .event(
            expect::event()
                .with_fields(expect::msg("hello world"))
                .in_scope(vec![
                    expect::span().at_level(Level::ERROR),
                    expect::span().at_level(Level::WARN),
                    expect::span().at_level(Level::INFO),
                ]),
        )
        .exit(expect::span().at_level(Level::ERROR))
        .exit(expect::span().at_level(Level::WARN))
        .exit(expect::span().at_level(Level::INFO))
        .only()
        .run_with_handle();
    let (warn_subscriber, warn_handle) = subscriber::named("warn")
        .enter(expect::span().at_level(Level::WARN))
        .enter(expect::span().at_level(Level::ERROR))
        .event(
            expect::event()
                .with_fields(expect::msg("hello world"))
                .in_scope(vec![
                    expect::span().at_level(Level::ERROR),
                    expect::span().at_level(Level::WARN),
                ]),
        )
        .exit(expect::span().at_level(Level::ERROR))
        .exit(expect::span().at_level(Level::WARN))
        .only()
        .run_with_handle();

    let _subscriber = tracing_subscriber::registry()
        .with(debug_subscriber.with_filter(LevelFilter::DEBUG))
        .with(info_subscriber.with_filter(LevelFilter::INFO))
        .with(warn_subscriber.with_filter(LevelFilter::WARN))
        .set_default();

    {
        let _trace = tracing::trace_span!("my_span").entered();
        let _debug = tracing::debug_span!("my_span").entered();
        let _info = tracing::info_span!("my_span").entered();
        let _warn = tracing::warn_span!("my_span").entered();
        let _error = tracing::error_span!("my_span").entered();
        tracing::error!("hello world");
    }

    debug_handle.assert_finished();
    info_handle.assert_finished();
    warn_handle.assert_finished();
}

#[test]
fn filters_interleaved_span_scopes() {
    fn target_subscriber(target: &'static str) -> (MockSubscriber, collector::MockHandle) {
        subscriber::named(format!("target_{}", target))
            .enter(expect::span().with_target(target))
            .enter(expect::span().with_target(target))
            .event(
                expect::event()
                    .with_fields(expect::msg("hello world"))
                    .in_scope(vec![
                        expect::span().with_target(target),
                        expect::span().with_target(target),
                    ]),
            )
            .event(
                expect::event()
                    .with_fields(expect::msg("hello to my target"))
                    .in_scope(vec![
                        expect::span().with_target(target),
                        expect::span().with_target(target),
                    ])
                    .with_target(target),
            )
            .exit(expect::span().with_target(target))
            .exit(expect::span().with_target(target))
            .only()
            .run_with_handle()
    }

    let (a_subscriber, a_handle) = target_subscriber("a");
    let (b_subscriber, b_handle) = target_subscriber("b");
    let (all_subscriber, all_handle) = subscriber::named("all")
        .enter(expect::span().with_target("b"))
        .enter(expect::span().with_target("a"))
        .event(
            expect::event()
                .with_fields(expect::msg("hello world"))
                .in_scope(vec![
                    expect::span().with_target("a"),
                    expect::span().with_target("b"),
                ]),
        )
        .exit(expect::span().with_target("a"))
        .exit(expect::span().with_target("b"))
        .only()
        .run_with_handle();

    let _subscriber = tracing_subscriber::registry()
        .with(all_subscriber.with_filter(LevelFilter::INFO))
        .with(a_subscriber.with_filter(filter::filter_fn(|meta| {
            let target = meta.target();
            target == "a" || target == module_path!()
        })))
        .with(b_subscriber.with_filter(filter::filter_fn(|meta| {
            let target = meta.target();
            target == "b" || target == module_path!()
        })))
        .set_default();

    {
        let _a1 = tracing::trace_span!(target: "a", "a/trace").entered();
        let _b1 = tracing::info_span!(target: "b", "b/info").entered();
        let _a2 = tracing::info_span!(target: "a", "a/info").entered();
        let _b2 = tracing::trace_span!(target: "b", "b/trace").entered();
        tracing::info!("hello world");
        tracing::debug!(target: "a", "hello to my target");
        tracing::debug!(target: "b", "hello to my target");
    }

    a_handle.assert_finished();
    b_handle.assert_finished();
    all_handle.assert_finished();
}
