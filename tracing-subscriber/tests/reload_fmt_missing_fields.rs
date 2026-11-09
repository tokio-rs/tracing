//! Regression test for <https://github.com/tokio-rs/tracing/issues/3511>
//!
//! When `reload::Layer` swaps from one fmt layer variant to another (e.g.
//! Full -> Pretty), spans that pre-date the swap have `FormattedFields<OldN>`
//! but not `FormattedFields<NewN>`.  Emitting an event inside such a span
//! must not panic.
#![cfg(all(feature = "fmt", feature = "ansi", feature = "registry"))]

use tracing_subscriber::{fmt, layer::SubscriberExt, reload};

/// Reload from a Full-format layer to a Pretty-format layer.
///
/// The pre-existing span has `FormattedFields<DefaultFields>` but no
/// `FormattedFields<Pretty>`. The old code panicked with `.expect()` on
/// the missing extension; the fix skips the "with ..." segment instead.
#[test]
fn pretty_fmt_does_not_panic_on_reload_from_full() {
    type DynLayer = Box<
        dyn tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync,
    >;

    let full: DynLayer = Box::new(fmt::layer().with_writer(|| std::io::sink()));
    let (dyn_layer, handle) = reload::Layer::new(full);

    let subscriber = tracing_subscriber::registry()
        .with(dyn_layer);

    tracing_core::dispatcher::with_default(&tracing_core::Dispatch::new(subscriber), || {
        // Span created while the Full layer is active.
        // FormattedFields<DefaultFields> is inserted; FormattedFields<Pretty> is NOT.
        let span = tracing::info_span!("pre_existing");
        let _guard = span.enter();

        // Swap to a Pretty layer. The existing span still has no FormattedFields<Pretty>.
        let pretty: DynLayer =
            Box::new(fmt::layer().pretty().with_writer(|| std::io::sink()));
        handle.reload(pretty).expect("reload must succeed");

        // Must NOT panic even though FormattedFields<Pretty> is absent on the span.
        tracing::info!("event after swap to pretty");
    });
}
