#[cfg(test)]
pub use self::support::*;
// This has to have the same name as the module in `tracing`.
#[path = "../../tracing/tests/support/mod.rs"]
#[cfg(test)]
mod support;
