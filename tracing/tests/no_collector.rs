#![cfg(feature = "std")]

use tracing::collect;
use tracing_mock::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn no_collector_disables_global() {
    // Reproduces https://github.com/tokio-rs/tracing/issues/1999
    let (collector, handle) = collector::mock().done().run_with_handle();
    collect::set_global_default(collector).expect("setting global default must succeed");
    collect::with_default(collect::NoCollector::default(), || {
        tracing::info!("this should not be recorded");
    });
    handle.assert_finished();
}
