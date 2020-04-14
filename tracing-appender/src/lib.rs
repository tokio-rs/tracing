use crate::non_blocking::{NonBlocking, WorkerGuard};

use std::io::Write;

mod inner;
pub mod non_blocking;
pub mod rolling;
mod worker;

/// Creates a non-blocking, off-thread writer.
///
/// Note that the `WorkerGuard` returned by `non_blocking` _must_ be assigned to a binding that
/// is not `_`, as `_` will result in the `WorkerGuard` being dropped immediately.
///
/// # Examples
/// ``` rust
///
/// # fn docs() {
/// let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
/// let subscriber = tracing_subscriber::fmt().with_writer(non_blocking);
/// tracing::subscriber::with_default(subscriber.finish(), || {
///    tracing::event!(tracing::Level::INFO, "Hello");
/// });
/// # }
/// ```
pub fn non_blocking<T: Write + Send + Sync + 'static>(writer: T) -> (NonBlocking, WorkerGuard) {
    NonBlocking::new(writer)
}
