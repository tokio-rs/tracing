//! Ensures that all unqualified uses of `tracing` can be overridden.
//!
//! As compilation fails directly when `tracing` is shadowed, no test functions are run here.

use ::tracing::instrument;

/// Shadows the crate `tracing` locally.
#[allow(dead_code)]
mod tracing {}

#[instrument(tracing = ::tracing)]
pub fn plain() {}

#[instrument(tracing = ::tracing)]
pub async fn async_fn() {}

#[derive(Debug)]
pub struct Custom;

#[instrument(tracing = ::tracing)]
pub fn implicit_field_debug(custom: Custom) {}

#[instrument(tracing = ::tracing, level = "error")]
pub fn level_error() {}
#[instrument(tracing = ::tracing, level = "warn")]
pub fn level_warn() {}
#[instrument(tracing = ::tracing, level = "info")]
pub fn level_info() {}
#[instrument(tracing = ::tracing, level = "debug")]
pub fn level_debug() {}
#[instrument(tracing = ::tracing, level = "trace")]
pub fn level_trace() {}

#[instrument(tracing = ::tracing, level = 5)]
pub fn level_5() {}
#[instrument(tracing = ::tracing, level = 4)]
pub fn level_4() {}
#[instrument(tracing = ::tracing, level = 3)]
pub fn level_3() {}
#[instrument(tracing = ::tracing, level = 2)]
pub fn level_2() {}
#[instrument(tracing = ::tracing, level = 1)]
pub fn level_1() {}

const A_LEVEL: ::tracing::Level = ::tracing::Level::INFO;
#[instrument(tracing = ::tracing, level = A_LEVEL)]
pub fn level_ident() {}

#[::tracing::instrument(tracing = ::tracing, fields(empty))]
pub fn empty() {}

#[::tracing::instrument(tracing = ::tracing, ret)]
pub fn ret() {}

#[instrument(tracing = ::tracing, ret(Debug))]
pub fn ret_debug() {}

#[instrument(tracing = ::tracing, ret(Display))]
pub fn ret_display() -> &'static str {
    ""
}

#[instrument(tracing = ::tracing, err)]
pub fn err() -> Result<(), &'static str> {
    Ok(())
}

#[instrument(tracing = ::tracing, err(Debug))]
pub fn err_debug() -> Result<(), &'static str> {
    Ok(())
}

#[instrument(tracing = ::tracing, err(Display))]
pub fn err_display() -> Result<(), &'static str> {
    Ok(())
}
