//! Synchronization primitives based on spinning

#[cfg(test)]
#[macro_use]
extern crate std;

pub use mutex::*;
pub use once::*;

mod mutex;
mod once;
