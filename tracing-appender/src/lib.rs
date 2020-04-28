//! Writers for logging events and spans
//!
//! # Overview
//!
//! [`tracing`][tracing] is a framework for structured, event-based diagnostic information.
//! `tracing-appender` allows events and spans to be recorded in a non-blocking manner through
//! a dedicated logging thread. It also provides a [`RollingFileAppender`][file_appender] that can
//! be used with _or_ without the non-blocking writer.
//!
//! [file_appender]: ./rolling/struct.RollingFileAppender.html
//! [tracing]: https://docs.rs/tracing/
//!
//! # Usage
//!
//! Add the following to your `Cargo.toml`:
//! ```toml
//! tracing-appender = "0.1"
//! ```
//!
//! This crate can be used in a few ways to record spans/events:
//!  - Using a [`RollingFileAppender`][rolling_struct] to perform writes to a log file. This will block on writes.
//!  - Using *any* type implementing [`std::io::Write`][write] in a non-blocking fashion.
//!  - Using a combination of [`NonBlocking`][non_blocking] and [`RollingFileAppender`][rolling_struct] to allow writes to a log file
//! without blocking.
//!
//! ## Rolling File Appender
//!
//! ```rust
//! # fn docs() {
//! let file_appender = tracing_appender::rolling::hourly("/some/directory", "prefix.log");
//! # }
//! ```
//! This creates an hourly rotating file appender that writes to `/some/directory/prefix.log.YYYY-MM-DD-HH`.
//! [`Rotation::DAILY`] and [`Rotation::NEVER`] are the other available options.
//!
//! The file appender implements [`std::io::Write`][write]. To be used with [`tracing_subscriber::FmtSubscriber`][fmt_subscrier],
//! it must be combined with a [`MakeWriter`][make_writer] implementation to be able to record tracing spans/event.
//!
//! The [`rolling` module][rolling]'s documentation provides more detail on how to use this file appender.
//!
//! ## Non-Blocking Writer
//!
//! The example below demonstrates the construction of a `non_blocking` writer with `std::io::stdout()`,
//! which implements [`MakeWriter`][make_writer].
//!
//! ```rust
//! # fn doc() {
//! let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
//! let subscriber = tracing_subscriber::fmt().with_writer(non_blocking);
//! tracing::subscriber::set_global_default(subscriber.finish()).expect("Could not set global default");
//! # }
//! ```
//! **Note:** `_guard` is a [`WorkerGuard`][guard] which is returned by [`tracing_appender::non_blocking`][non_blocking]
//! to ensure buffered logs are flushed to their output in the case of abrupt terminations of a process.
//! See [`WorkerGuard` module][guard] for more details.
//!
//! The example below demonstrates the construction of a [`tracing_appender::non_blocking`][non_blocking]
//! writer constructed with a [`std::io::Write`][write]:
//!
//! ```rust
//! # fn doc() {
//! let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::Stdout);
//! let subscriber = tracing_subscriber::fmt().with_writer(non_blocking);
//! tracing::subscriber::set_global_default(subscriber.finish()).expect("Could not set global default");
//! # }
//! ```
//!
//! The [`non_blocking module`][non_blocking]'s documentation provides more detail on how to use `non_blocking`.
//!
//! [non_blocking]: ./non_blocking/index.html
//! [write]: https://doc.rust-lang.org/std/io/trait.Write.html
//! [guard]: ./non_blocking/struct.WorkerGuard.html
//! [rolling]: ./rolling/index.html
//! [make_writer]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/trait.MakeWriter.html
//! [rolling_struct]: ./rolling/struct.RollingFileAppender.html
//! [fmt_subscriber]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/struct.Subscriber.html
//!
//! ## Non-Blocking Rolling File Appender
//!
//! ```rust
//! # fn docs() {
//! let file_appender = tracing_appender::rolling::hourly("/some/directory", "prefix.log");
//! let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
//! let subscriber = tracing_subscriber::fmt().with_writer(non_blocking);
//!
//! tracing::subscriber::set_global_default(subscriber.finish()).expect("Could not set global default");
//! # }
//! ```
use crate::non_blocking::{NonBlocking, WorkerGuard};

use std::io::Write;

mod inner;

pub mod non_blocking;

pub mod rolling;

mod worker;

/// Convenience function for creating a non-blocking, off-thread writer.
///
/// See the [`non_blocking` module's docs][non_blocking]'s for more details.
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
