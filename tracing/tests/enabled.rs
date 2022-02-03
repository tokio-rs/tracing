// liballoc is required because the test subscriber cannot be constructed
// statically
#![cfg(feature = "alloc")]

mod support;

use self::support::*;
use tracing::Level;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn level_and_target() {
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
        .done()
        .run();

    tracing::collect::set_global_default(collector).unwrap();

    assert!(tracing::enabled!(target: "debug_module", Level::DEBUG));
    assert!(tracing::enabled!(Level::ERROR));
    assert!(!tracing::enabled!(Level::DEBUG));

    // Ensure that the `_event` and `_span` alternatives work corretly
    assert!(!tracing::enabled_event!(Level::TRACE));
    assert!(tracing::enabled_event!(Level::DEBUG));

    assert!(tracing::enabled_span!(Level::TRACE));
}
