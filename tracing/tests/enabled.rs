// liballoc is required because the test subscriber cannot be constructed
// statically
#![cfg(feature = "alloc")]

use tracing::Level;
use tracing_mock::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn level_and_target() {
    let collector = collector::mock()
        .with_filter(|meta| {
            if meta.target() == "debug_module" {
                meta.level() <= &Level::DEBUG
            } else {
                meta.level() <= &Level::INFO
            }
        })
        .only()
        .run();

    let _guard = tracing::collect::set_default(collector);

    assert!(tracing::enabled!(target: "debug_module", Level::DEBUG));
    assert!(tracing::enabled!(Level::ERROR));
    assert!(!tracing::enabled!(Level::DEBUG));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn span_and_event() {
    let collector = collector::mock()
        .with_filter(|meta| {
            if meta.target() == "debug_module" {
                meta.level() <= &Level::DEBUG
            } else if meta.is_span() {
                meta.level() <= &Level::TRACE
            } else if meta.is_event() {
                meta.level() <= &Level::DEBUG
            } else {
                meta.level() <= &Level::INFO
            }
        })
        .only()
        .run();

    let _guard = tracing::collect::set_default(collector);

    // Ensure that the `_event` and `_span` alternatives work correctly
    assert!(!tracing::event_enabled!(Level::TRACE));
    assert!(tracing::event_enabled!(Level::DEBUG));
    assert!(tracing::span_enabled!(Level::TRACE));

    // target variants
    assert!(tracing::span_enabled!(target: "debug_module", Level::DEBUG));
    assert!(tracing::event_enabled!(target: "debug_module", Level::DEBUG));
}
