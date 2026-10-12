use super::*;

/// Reproduces #2519: a bare `dispatch.enabled()` call that every per-layer
/// filter rejects must not cause the *next* span — whose callsite cached
/// `Interest::always` and therefore skips `enabled()` — to be dropped from
/// per-layer-filtered layers.
///
/// The production trigger is `log::log_enabled!` via `tracing-log`'s
/// `LogTracer`, which calls `dispatch.enabled()` with per-call dynamic
/// metadata that bypasses callsite caching.
#[test]
fn always_interest_span_after_globally_rejected_enabled() {
    // Two per-layer-filtered layers. Both enable INFO at "test" — so before
    // this fix the Registry cached `Interest::always` for a span there
    // (that's the precondition for the bug; with the fix it caps at
    // `sometimes`) — and both reject DEBUG at "other".
    let (a, a_handle) = layer::named("a")
        .new_span(expect::span().named("before"))
        .new_span(expect::span().named("after"))
        .only()
        .run_with_handle();
    let (b, b_handle) = layer::named("b")
        .new_span(expect::span().named("before"))
        .new_span(expect::span().named("after"))
        .only()
        .run_with_handle();

    let make_filter = || {
        filter::Targets::new()
            .with_target("test", Level::INFO)
            .with_default(LevelFilter::OFF)
    };
    let sub = tracing_subscriber::registry()
        .with(a.with_filter(make_filter()))
        .with(b.with_filter(make_filter()));
    let _guard = sub.set_default();

    let _s1 = tracing::info_span!(target: "test", "before");

    // Bare enabled() at ("other", DEBUG): both per-layer filters reject
    // and set their FilterState bits to disabled. The caller doesn't
    // dispatch (this is a bare enabled() query, not an event), so no on_*
    // follows and nothing resets the bits. Mirrors what
    // LogTracer::enabled does for a `log::log_enabled!` query.
    struct PoisonCallsite;
    static POISON: PoisonCallsite = PoisonCallsite;
    impl tracing_core::Callsite for PoisonCallsite {
        fn set_interest(&self, _: tracing_core::Interest) {}
        fn metadata(&self) -> &tracing_core::Metadata<'_> {
            &META
        }
    }
    static META: tracing_core::Metadata<'static> = tracing_core::Metadata::new(
        "log event",
        "other",
        Level::DEBUG,
        None,
        None,
        None,
        tracing_core::field::FieldSet::new(&[], tracing_core::identify_callsite!(&POISON)),
        tracing_core::Kind::EVENT,
    );
    tracing::dispatcher::get_default(|d| {
        let _ = d.enabled(&META);
    });

    // Before the fix, this span never reached either layer's on_new_span:
    // its Interest::always callsite skipped enabled(), and
    // Filtered::on_new_span read the stale disabled bits left by the
    // poison enabled() call.
    let _s2 = tracing::info_span!(target: "test", "after");

    a_handle.assert_finished();
    b_handle.assert_finished();
}
