use test_log_support::Test;
use tracing::{enabled, warn, Level};
use tracing_core::span::Current;

pub struct LevelCollector {
    max_level: tracing::Level,
}

impl tracing::Collect for LevelCollector {
    fn enabled(&self, metadata: &tracing::Metadata) -> bool {
        metadata.level() <= &self.max_level
    }
    fn new_span(&self, _: &tracing::span::Attributes) -> tracing::span::Id {
        use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        tracing::span::Id::from_u64(NEXT.fetch_add(1, Relaxed))
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, _: &tracing::Event) {}
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
    fn try_close(&self, _: tracing::span::Id) -> bool {
        true
    }
    fn current_span(&self) -> Current {
        Current::unknown()
    }
}


#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn log_with_trace_enabled() {
    tracing::collect::set_global_default(LevelCollector {
        max_level: Level::ERROR,
    })
    .expect("set global should succeed");

    let test = Test::with_filters(&[
        (module_path!(), log::LevelFilter::Warn),
    ]);

    // should evaluate to true because a logger is interested even
    // though the tracing collector is not.
    if enabled!(Level::WARN) {
        warn!(msg = 2);
    }
    test.assert_logged("msg=2");

    if enabled!(target: module_path!(), Level::INFO) {
        panic!(
            "info level unexpectedly enabled for target {}",
            module_path!()
        );
    }
}
