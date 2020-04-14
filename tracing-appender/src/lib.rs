use crate::non_blocking::{NonBlocking, WorkerGuard};

use std::io::Write;

mod inner;
pub mod non_blocking;
pub mod rolling;
mod worker;


/// Creates a non-blocking, off-thread writer.
///
/// This spawns a dedicated worker thread which is responsible for writing log
/// lines to the provided writer. When a line is written using the returned 
/// `NonBlocking` struct's `make_writer` method, it will be enqueued to be
/// written by the worker thread. 
///
/// The queue has a fixed capacity, and if it becomes full, any logs written
/// to it will be dropped until capacity is once again available. This may 
/// occur if logs are consistently produced faster than the worker thread can
/// output them. The queue capacity and behavior when full (i.e., whether to 
/// drop logs or to exert backpressure to slow down senders) can be configured
/// using [`NonBlocking::builder()`]. This function simply returns the default 
/// configuration &mdash; it is equivalent to
///
/// ```rust
/// # use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
/// # fn doc() -> (NonBlocking, WorkerGuard) {
/// tracing_appender::non_blocking::NonBlocking::builder().finish()
/// # }
/// ```
/// [`NonBlocking::builder()`]: /non_blocking/struct.NonBlocking.html#method.builder
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
