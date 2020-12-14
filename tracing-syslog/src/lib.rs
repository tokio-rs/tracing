#[cfg(unix)]
mod syslog;
#[cfg(unix)]
pub use syslog::*;
