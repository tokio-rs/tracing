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

#[instrument(tracing = ::tracing, fields(empty))]
pub fn empty() {}
