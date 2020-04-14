use crate::non_blocking::{NonBlocking, WorkerGuard};

use std::io::Write;

mod inner;
pub mod non_blocking;
pub mod rolling;
mod worker;

/// A function for which accepts a struct implementing the `Write` Trait and provides a
/// a `MakeWriter` which allows writes to happen in a non blocking manner.
/// # Examples
/// ``` rust,ignore
/// use tracing_subscriber::fmt::MakeWriter;
///
/// let (non_blocking, _guard) = non_blocking(std::io::stdout());
/// let subscriber = tracing_subscriber::fmt().with_writer(non_blocking);
/// tracing::subscriber::with_default(subscriber.finish(), || {
///    tracing::event!(tracing::Level::INFO, "Hello");
/// });
///
/// ```
pub fn non_blocking<T: Write + Send + Sync + 'static>(writer: T) -> (NonBlocking, WorkerGuard) {
    NonBlocking::new(writer)
}
