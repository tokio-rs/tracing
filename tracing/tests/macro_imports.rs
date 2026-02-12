use tracing::Level;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn prefixed_span_macros() {
    let _ = tracing::span!(Level::DEBUG, "foo");
    let _ = tracing::trace_span!("foo");
    let _ = tracing::debug_span!("foo");
    let _ = tracing::info_span!("foo");
    let _ = tracing::warn_span!("foo");
    let _ = tracing::error_span!("foo");
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn prefixed_event_macros() {
    let _ = tracing::event!(Level::DEBUG, "foo");
    let _ = tracing::trace!("foo");
    let _ = tracing::debug!("foo");
    let _ = tracing::info!("foo");
    let _ = tracing::warn!("foo");
    let _ = tracing::error!("foo");
}
