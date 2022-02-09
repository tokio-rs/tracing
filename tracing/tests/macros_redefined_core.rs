extern crate self as core;

use tracing::{enabled, event, span, Level};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn span() {
    span!(Level::DEBUG, "foo");
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn event() {
    event!(Level::DEBUG, "foo");
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn enabled() {
    enabled!(Level::DEBUG);
}
