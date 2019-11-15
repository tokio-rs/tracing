//! Synchronization primitives based on spinning

pub(crate) use mutex::*;
pub use once::Once;

mod mutex;
mod once;
