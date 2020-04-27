//! Writers for logging events/spans
//!
//! # Overview
//!
//! [`tracing`][tracing] is a framework for instrumenting Rust programs to collect
//! structured, event-based diagnostic information. This crate provides the ability
//! for `tracing` events and spans to be recorded in a non blocking manner by using a
//! dedicated logging thread. It also provides a [`RollingFileAppender`][file_appender]
//! that can be used with or without the non blocking writer.
//!
//! [file_appender]: ./rolling/struct.RollingFileAppender.html
//! [tracing]: https://docs.rs/tracing/
//!
//! # Usage
//! This crate can be used in a few ways to record spans/events:
//! 1. Using a `RollingFileAppender` to perform writes to a log file. This will block on writes.
//! 2. Using *any* `std::io::Write` implementation in a non-blocking way
//! 2. Using a combination of `NonBlocking` and `RollingFileAppender` to allow writes to a log file
//! without blocking.
//!
//! ## Rolling File Appender
//!
//! ```rust
//! # fn docs() {
//! use tracing_appender::rolling::{RollingFileAppender, Rotation};
//!
//! let file_appender = RollingFileAppender::new(Rotation::HOURLY, "/some/directory", "prefix.log");
//! # }
//! ```
//! This creates an hourly rotating file appender which outputs to `/some/directory/prefix.log.YYYY-MM-DD-HH`.
//! <br/> [`Rotation::DAILY`] and [`Rotation::NEVER`] are the other available options.
//!
//! It implements the `std::io::Write` trait. To use with a subscriber, it must be combined with a
//! [`MakeWriter`][make_writer] implementation to be able to record tracing spans/event.
//!
//! [make_writer]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/trait.MakeWriter.html
//!
//! The [`rolling module`][rolling]'s documentation provides more detail on how to use this file appender.
//!
//! [rolling]: ./rolling/index.html
//!
//! ## Non Blocking Writer
//!
//! ```rust
//! # fn doc() {
//! let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
//! let subscriber = tracing_subscriber::fmt().with_writer(non_blocking);
//! tracing::subscriber::set_global_default(subscriber.finish()).expect("Could not set global default");
//! # }
//! ```
//!
//! Alternatively, you can provide *any* type that implements `std::io::Write` to `tracing_appender::nonblocking`
//!
//! The [`non_blocking module`][non_blocking]'s documentation provides more detail on how to use `non_blocking`.
//!
//! [non_blocking]: ./non_blocking/index.html
//!
//! ## Non Blocking and rolling file appender
//!
//! ```rust
//! # fn docs() {
//! use tracing_appender::rolling::{RollingFileAppender, Rotation};
//!
//! let file_appender = RollingFileAppender::new(Rotation::HOURLY, "/some/directory", "prefix.log");
//! let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
//! let subscriber = tracing_subscriber::fmt().with_writer(non_blocking);
//!
//! tracing::subscriber::set_global_default(subscriber.finish()).expect("Could not set global default");
//! # }
//! ```
use crate::non_blocking::{NonBlocking, WorkerGuard};

use std::io::Write;

mod inner;

/// A non-blocking, off-thread writer.
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
/// using [`NonBlockingBuilder::default()`][builder].
/// This function returns the default configuration. It is equivalent to:
///
/// ```rust
/// # use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
/// # fn doc() -> (NonBlocking, WorkerGuard) {
/// tracing_appender::non_blocking::NonBlocking::new(std::io::stdout())
/// # }
/// ```
/// [builder]: ./struct.NonBlockingBuilder.html#method.default
///
/// <br/> This function returns a tuple of `NonBlocking` and `WorkerGuard`.
/// `NonBlocking` implements [`MakeWriter`] which integrates with `tracing_subscriber`.
/// `WorkerGuard` is a drop guard that is responsible for flushing any remaining logs when
/// the program terminates.
///
/// Note that the `WorkerGuard` returned by `non_blocking` _must_ be assigned to a binding that
/// is not `_`, as `_` will result in the `WorkerGuard` being dropped immediately.
/// Unintentional drops of `WorkerGuard` remove the guarantee that logs will be flushed
/// during a program's termination, in a panic or otherwise.
///
/// See [`WorkerGuard`][worker_guard] for examples of using the guard.
///
/// [worker_guard]: ./struct.WorkerGuard.html
///
/// # Examples
///
/// ``` rust
/// # fn docs() {
/// let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
/// let subscriber = tracing_subscriber::fmt().with_writer(non_blocking);
/// tracing::subscriber::with_default(subscriber.finish(), || {
///    tracing::event!(tracing::Level::INFO, "Hello");
/// });
/// # }
/// ```
pub mod non_blocking;

/// A rolling file appender.
///
/// Creates a log file at a specified directory and file name prefix which *may* be appended with
/// the date and time.
///
/// The following helpers are available for creating a rolling file appender.
///
/// - [`Rotation::hourly()`][hourly]:
///     <br/> This will result in log file located at `some_directory/log_file_name_prefix.YYYY-MM-DD-HH`
/// - [`Rotation::daily()`][daily]:
///     <br/> This will result in log file located at `some_directory/log_file_name_prefix.YYYY-MM-DD`
/// - [`Rotation::never()`][never]:
///     <br/> This will result in log file located at `some_directory/log_file_name`
///
/// [hourly]: fn.hourly.html
/// [daily]: fn.daily.html
/// [never]: fn.never.html
///
/// # Examples
///
/// ```rust
/// # fn docs() {
/// use tracing_appender::rolling::{RollingFileAppender, Rotation};
/// let file_appender = RollingFileAppender::new(Rotation::HOURLY, "/some/directory", "prefix.log");
/// # }
/// ```
pub mod rolling;

mod worker;

/// Convenience function for creating a non-blocking, off-thread writer.
///
/// See [`non_blocking module`][non_blocking]'s for more details.
///
/// [non_blocking]: ./non_blocking/index.html
///
/// # Examples
///
/// ``` rust
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
