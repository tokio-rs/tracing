//! `syslog` logging support for `tracing` backed by `libc`'s
//! [`syslog()`](libc::syslog) function.
//!
//! See [`Syslog`] for documentation and examples.

#[cfg(unix)]
mod syslog;
#[cfg(unix)]
pub use syslog::*;
