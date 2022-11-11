use super::*;
use tracing_mock::subscriber::MockSubscriber;

#[test]
fn basic_trees() {
    let (with_target, with_target_handle) = subscriber::named("info_with_target")
        .event(
            expect::event()
                .at_level(Level::INFO)
                .with_target("my_target"),
        )
        .only()
        .run_with_handle();

    let (info, info_handle) = subscriber::named("info")
        .event(
            expect::event()
                .at_level(Level::INFO)
                .with_target(module_path!()),
        )
        .event(
            expect::event()
                .at_level(Level::INFO)
                .with_target("my_target"),
        )
        .only()
        .run_with_handle();

    let (all, all_handle) = subscriber::named("all")
        .event(
            expect::event()
                .at_level(Level::INFO)
                .with_target(module_path!()),
        )
        .event(expect::event().at_level(Level::TRACE))
        .event(
            expect::event()
                .at_level(Level::INFO)
                .with_target("my_target"),
        )
        .event(
            expect::event()
                .at_level(Level::TRACE)
                .with_target("my_target"),
        )
        .only()
        .run_with_handle();

    let info_tree = info
        .and_then(
            with_target.with_filter(filter::filter_fn(|meta| dbg!(meta.target()) == "my_target")),
        )
        .with_filter(LevelFilter::INFO);

    let subscriber = tracing_subscriber::registry().with(info_tree).with(all);
    let _guard = dbg!(subscriber).set_default();

    tracing::info!("hello world");
    tracing::trace!("hello trace");
    tracing::info!(target: "my_target", "hi to my target");
    tracing::trace!(target: "my_target", "hi to my target at trace");

    all_handle.assert_finished();
    info_handle.assert_finished();
    with_target_handle.assert_finished();
}

#[test]
fn filter_span_scopes() {
    fn target_subscriber(target: &'static str) -> (MockSubscriber, collector::MockHandle) {
        subscriber::named(format!("target_{}", target))
            .enter(expect::span().with_target(target).at_level(Level::INFO))
            .event(event::msg("hello world").in_scope(vec![
                expect::span().with_target(target).at_level(Level::INFO),
            ]))
            .exit(expect::span().with_target(target).at_level(Level::INFO))
            .only()
            .run_with_handle()
    }

    let (a_subscriber, a_handle) = target_subscriber("a");
    let (b_subscriber, b_handle) = target_subscriber("b");
    let (info_subscriber, info_handle) = subscriber::named("info")
        .enter(expect::span().with_target("b").at_level(Level::INFO))
        .enter(expect::span().with_target("a").at_level(Level::INFO))
        .event(event::msg("hello world").in_scope(vec![
            expect::span().with_target("a").at_level(Level::INFO),
            expect::span().with_target("b").at_level(Level::INFO),
        ]))
        .exit(expect::span().with_target("a").at_level(Level::INFO))
        .exit(expect::span().with_target("b").at_level(Level::INFO))
        .only()
        .run_with_handle();

    let full_scope = vec![
        expect::span().with_target("b").at_level(Level::TRACE),
        expect::span().with_target("a").at_level(Level::INFO),
        expect::span().with_target("b").at_level(Level::INFO),
        expect::span().with_target("a").at_level(Level::TRACE),
    ];
    let (all_subscriber, all_handle) = subscriber::named("all")
        .enter(expect::span().with_target("a").at_level(Level::TRACE))
        .enter(expect::span().with_target("b").at_level(Level::INFO))
        .enter(expect::span().with_target("a").at_level(Level::INFO))
        .enter(expect::span().with_target("b").at_level(Level::TRACE))
        .event(event::msg("hello world").in_scope(full_scope.clone()))
        .event(
            event::msg("hello to my target")
                .with_target("a")
                .in_scope(full_scope.clone()),
        )
        .event(
            event::msg("hello to my target")
                .with_target("b")
                .in_scope(full_scope),
        )
        .exit(expect::span().with_target("b").at_level(Level::TRACE))
        .exit(expect::span().with_target("a").at_level(Level::INFO))
        .exit(expect::span().with_target("b").at_level(Level::INFO))
        .exit(expect::span().with_target("a").at_level(Level::TRACE))
        .only()
        .run_with_handle();

    let a_subscriber = a_subscriber.with_filter(filter::filter_fn(|meta| {
        let target = meta.target();
        target == "a" || target == module_path!()
    }));

    let b_subscriber = b_subscriber.with_filter(filter::filter_fn(|meta| {
        let target = meta.target();
        target == "b" || target == module_path!()
    }));

    let info_tree = info_subscriber
        .and_then(a_subscriber)
        .and_then(b_subscriber)
        .with_filter(LevelFilter::INFO);

    let subscriber = tracing_subscriber::registry()
        .with(info_tree)
        .with(all_subscriber);
    let _guard = dbg!(subscriber).set_default();

    {
        let _a1 = tracing::trace_span!(target: "a", "a/trace").entered();
        let _b1 = tracing::info_span!(target: "b", "b/info").entered();
        let _a2 = tracing::info_span!(target: "a", "a/info").entered();
        let _b2 = tracing::trace_span!(target: "b", "b/trace").entered();
        tracing::info!("hello world");
        tracing::debug!(target: "a", "hello to my target");
        tracing::debug!(target: "b", "hello to my target");
    }

    all_handle.assert_finished();
    info_handle.assert_finished();
    a_handle.assert_finished();
    b_handle.assert_finished();
}
