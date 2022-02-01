mod support;

use self::support::*;
use tracing::Level;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn level_and_target() {
    let (subscriber, handle) = subscriber::mock()
        .with_filter(|meta| {
            if meta.target() == "debug_module" {
                meta.level() <= &Level::DEBUG
            } else {
                meta.level() <= &Level::INFO
            }
        })
        .done()
        .run_with_handle();

    tracing::subscriber::set_global_default(subscriber).unwrap();

    assert!(tracing::enabled!(target: "debug_module", Level::DEBUG));
    assert!(tracing::enabled!(Level::ERROR));
    assert!(!tracing::enabled!(Level::DEBUG));
    handle.assert_finished();
}
