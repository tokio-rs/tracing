use super::*;

pub(crate) mod basic {
    use super::*;
    pub(crate) fn with_target_subscriber() -> (ExpectSubscriber, collector::MockHandle) {
        subscriber::named("info_with_target")
            .event(event::mock().at_level(Level::INFO).with_target("my_target"))
            .done()
            .run_with_handle()
    }

    pub(crate) fn all_subscriber() -> (ExpectSubscriber, collector::MockHandle) {
        subscriber::named("all")
            .event(
                event::mock()
                    .at_level(Level::INFO)
                    .with_target(module_path!()),
            )
            .event(event::mock().at_level(Level::TRACE))
            .event(event::mock().at_level(Level::INFO).with_target("my_target"))
            .event(
                event::mock()
                    .at_level(Level::TRACE)
                    .with_target("my_target"),
            )
            .done()
            .run_with_handle()
    }

    pub(crate) fn info_subscriber() -> (ExpectSubscriber, collector::MockHandle) {
        subscriber::named("info")
            .event(
                event::mock()
                    .at_level(Level::INFO)
                    .with_target(module_path!()),
            )
            .event(event::mock().at_level(Level::INFO).with_target("my_target"))
            .done()
            .run_with_handle()
    }

    pub(crate) fn run() {
        tracing::info!("hello world");
        tracing::trace!("hello trace");
        tracing::info!(target: "my_target", "hi to my target");
        tracing::trace!(target: "my_target", "hi to my target at trace");
    }

    pub(crate) fn target_filter_fn(meta: &tracing_core::Metadata<'_>) -> bool {
        dbg!(meta.target()) == "my_target"
    }
}

#[test]
fn basic_trees() {
    let (with_target, with_target_handle) = basic::with_target_subscriber();
    let (info, info_handle) = basic::info_subscriber();
    let (all, all_handle) = basic::all_subscriber();

    let info_tree = info
        .and_then(with_target.with_filter(filter::filter_fn(basic::target_filter_fn)))
        .with_filter(LevelFilter::INFO);

    let subscriber = tracing_subscriber::registry().with(info_tree).with(all);
    let _guard = dbg!(subscriber).set_default();

    basic::run();

    all_handle.assert_finished();
    info_handle.assert_finished();
    with_target_handle.assert_finished();
}

pub(crate) mod span_scopes {
    use super::*;
    fn target_subscriber(target: &'static str) -> (ExpectSubscriber, collector::MockHandle) {
        subscriber::named(format!("target_{}", target))
            .enter(span::mock().with_target(target).at_level(Level::INFO))
            .event(
                event::msg("hello world")
                    .in_scope(vec![span::mock().with_target(target).at_level(Level::INFO)]),
            )
            .exit(span::mock().with_target(target).at_level(Level::INFO))
            .done()
            .run_with_handle()
    }

    pub(crate) fn info_subscriber() -> (ExpectSubscriber, collector::MockHandle) {
        subscriber::named("info")
            .enter(span::mock().with_target("b").at_level(Level::INFO))
            .enter(span::mock().with_target("a").at_level(Level::INFO))
            .event(event::msg("hello world").in_scope(vec![
                span::mock().with_target("a").at_level(Level::INFO),
                span::mock().with_target("b").at_level(Level::INFO),
            ]))
            .exit(span::mock().with_target("a").at_level(Level::INFO))
            .exit(span::mock().with_target("b").at_level(Level::INFO))
            .done()
            .run_with_handle()
    }

    pub(crate) fn a_subscriber() -> (ExpectSubscriber, collector::MockHandle) {
        target_subscriber("a")
    }

    pub(crate) fn b_subscriber() -> (ExpectSubscriber, collector::MockHandle) {
        target_subscriber("b")
    }

    pub(crate) fn all_subscriber() -> (ExpectSubscriber, collector::MockHandle) {
        let full_scope = vec![
            span::mock().with_target("b").at_level(Level::TRACE),
            span::mock().with_target("a").at_level(Level::INFO),
            span::mock().with_target("b").at_level(Level::INFO),
            span::mock().with_target("a").at_level(Level::TRACE),
        ];
        subscriber::named("all")
            .enter(span::mock().with_target("a").at_level(Level::TRACE))
            .enter(span::mock().with_target("b").at_level(Level::INFO))
            .enter(span::mock().with_target("a").at_level(Level::INFO))
            .enter(span::mock().with_target("b").at_level(Level::TRACE))
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
            .exit(span::mock().with_target("b").at_level(Level::TRACE))
            .exit(span::mock().with_target("a").at_level(Level::INFO))
            .exit(span::mock().with_target("b").at_level(Level::INFO))
            .exit(span::mock().with_target("a").at_level(Level::TRACE))
            .done()
            .run_with_handle()
    }

    pub(crate) fn a_filter_fn(meta: &tracing_core::Metadata<'_>) -> bool {
        let target = meta.target();
        target == "a" || target == module_path!()
    }

    pub(crate) fn b_filter_fn(meta: &tracing_core::Metadata<'_>) -> bool {
        let target = meta.target();
        target == "b" || target == module_path!()
    }

    pub(crate) fn run() {
        let _a1 = tracing::trace_span!(target: "a", "a/trace").entered();
        let _b1 = tracing::info_span!(target: "b", "b/info").entered();
        let _a2 = tracing::info_span!(target: "a", "a/info").entered();
        let _b2 = tracing::trace_span!(target: "b", "b/trace").entered();
        tracing::info!("hello world");
        tracing::debug!(target: "a", "hello to my target");
        tracing::debug!(target: "b", "hello to my target");
    }
}

#[test]
fn filter_span_scopes() {
    let (a_subscriber, a_handle) = span_scopes::a_subscriber();
    let (b_subscriber, b_handle) = span_scopes::b_subscriber();
    let (info_subscriber, info_handle) = span_scopes::info_subscriber();
    let (all_subscriber, all_handle) = span_scopes::all_subscriber();

    let a_subscriber = a_subscriber.with_filter(filter::filter_fn(span_scopes::a_filter_fn));
    let b_subscriber = b_subscriber.with_filter(filter::filter_fn(span_scopes::b_filter_fn));

    let info_tree = info_subscriber
        .and_then(a_subscriber)
        .and_then(b_subscriber)
        .with_filter(LevelFilter::INFO);

    let subscriber = tracing_subscriber::registry()
        .with(info_tree)
        .with(all_subscriber);
    let _guard = dbg!(subscriber).set_default();

    span_scopes::run();

    all_handle.assert_finished();
    info_handle.assert_finished();
    a_handle.assert_finished();
    b_handle.assert_finished();
}
