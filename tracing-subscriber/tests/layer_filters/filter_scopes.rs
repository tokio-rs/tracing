use super::*;
use tracing_mock::layer::MockLayer;

#[test]
fn filters_span_scopes() {
    let (debug_layer, debug_handle) = layer::named("debug")
        .enter(span::expect().at_level(Level::DEBUG))
        .enter(span::expect().at_level(Level::INFO))
        .enter(span::expect().at_level(Level::WARN))
        .enter(span::expect().at_level(Level::ERROR))
        .event(event::msg("hello world").in_scope(vec![
            span::expect().at_level(Level::ERROR),
            span::expect().at_level(Level::WARN),
            span::expect().at_level(Level::INFO),
            span::expect().at_level(Level::DEBUG),
        ]))
        .exit(span::expect().at_level(Level::ERROR))
        .exit(span::expect().at_level(Level::WARN))
        .exit(span::expect().at_level(Level::INFO))
        .exit(span::expect().at_level(Level::DEBUG))
        .only()
        .run_with_handle();
    let (info_layer, info_handle) = layer::named("info")
        .enter(span::expect().at_level(Level::INFO))
        .enter(span::expect().at_level(Level::WARN))
        .enter(span::expect().at_level(Level::ERROR))
        .event(event::msg("hello world").in_scope(vec![
            span::expect().at_level(Level::ERROR),
            span::expect().at_level(Level::WARN),
            span::expect().at_level(Level::INFO),
        ]))
        .exit(span::expect().at_level(Level::ERROR))
        .exit(span::expect().at_level(Level::WARN))
        .exit(span::expect().at_level(Level::INFO))
        .only()
        .run_with_handle();
    let (warn_layer, warn_handle) = layer::named("warn")
        .enter(span::expect().at_level(Level::WARN))
        .enter(span::expect().at_level(Level::ERROR))
        .event(event::msg("hello world").in_scope(vec![
            span::expect().at_level(Level::ERROR),
            span::expect().at_level(Level::WARN),
        ]))
        .exit(span::expect().at_level(Level::ERROR))
        .exit(span::expect().at_level(Level::WARN))
        .only()
        .run_with_handle();

    let _subscriber = tracing_subscriber::registry()
        .with(debug_layer.with_filter(LevelFilter::DEBUG))
        .with(info_layer.with_filter(LevelFilter::INFO))
        .with(warn_layer.with_filter(LevelFilter::WARN))
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
    fn target_layer(target: &'static str) -> (MockLayer, subscriber::MockHandle) {
        layer::named(format!("target_{}", target))
            .enter(span::expect().with_target(target))
            .enter(span::expect().with_target(target))
            .event(event::msg("hello world").in_scope(vec![
                span::expect().with_target(target),
                span::expect().with_target(target),
            ]))
            .event(
                event::msg("hello to my target")
                    .in_scope(vec![
                        span::expect().with_target(target),
                        span::expect().with_target(target),
                    ])
                    .with_target(target),
            )
            .exit(span::expect().with_target(target))
            .exit(span::expect().with_target(target))
            .only()
            .run_with_handle()
    }

    let (a_layer, a_handle) = target_layer("a");
    let (b_layer, b_handle) = target_layer("b");
    let (all_layer, all_handle) = layer::named("all")
        .enter(span::expect().with_target("b"))
        .enter(span::expect().with_target("a"))
        .event(event::msg("hello world").in_scope(vec![
            span::expect().with_target("a"),
            span::expect().with_target("b"),
        ]))
        .exit(span::expect().with_target("a"))
        .exit(span::expect().with_target("b"))
        .only()
        .run_with_handle();

    let _subscriber = tracing_subscriber::registry()
        .with(all_layer.with_filter(LevelFilter::INFO))
        .with(a_layer.with_filter(filter::filter_fn(|meta| {
            let target = meta.target();
            target == "a" || target == module_path!()
        })))
        .with(b_layer.with_filter(filter::filter_fn(|meta| {
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
