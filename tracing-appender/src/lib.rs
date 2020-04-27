//! Writers for logging events and spans
//!
//! # Overview
//!
//! [`tracing`][tracing] is a framework for instrumenting Rust programs to collect
//! structured, event-based diagnostic information. This crate provides the ability
//! for `tracing` events and spans to be recorded in a non-blocking manner by using a
//! dedicated logging thread. It also provides a [`RollingFileAppender`][file_appender]
//! that can be used with or without the non-blocking writer.
//!
//! [file_appender]: ./rolling/struct.RollingFileAppender.html
//! [tracing]: https://docs.rs/tracing/
//!
//! # Usage
//!
//! First, add this to your `Cargo.toml`:
//! ```toml
//! tracing-appender = "0.1"
//! ```
//!
//! This crate can be used in a few ways to record spans/events:
//!  - Using a `RollingFileAppender` to perform writes to a log file. This will block on writes.
//!  - Using *any* `std::io::Write` implementation in a non-blocking way
//!  - Using a combination of `NonBlocking` and `RollingFileAppender` to allow writes to a log file
//! without blocking.
//!
//! ## Rolling File Appender
//!
//! ```rust
//! # fn docs() {
//! let file_appender = tracing_appender::rolling::hourly("/some/directory", "prefix.log");
//! # }
//! ```
//! This creates an hourly rotating file appender which outputs to `/some/directory/prefix.log.YYYY-MM-DD-HH`.
//! [`Rotation::DAILY`] and [`Rotation::NEVER`] are the other available options.
//!
//! It implements the `std::io::Write` trait. To use with a subscriber, it must be combined with a
//! [`MakeWriter`][make_writer] implementation to be able to record tracing spans/event.
//!
//! [make_writer]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/trait.MakeWriter.html
//!
//! The [`rolling` module][rolling]'s documentation provides more detail on how to use this file appender.
//!
//! [rolling]: ./rolling/index.html
//!
//! ## Non-Blocking Writer
//!
//! ```rust
//! # fn doc() {
//! let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
//! let subscriber = tracing_subscriber::fmt().with_writer(non_blocking);
//! tracing::subscriber::set_global_default(subscriber.finish()).expect("Could not set global default");
//! # }
//! ```
//! **Note:** `_guard` is a [`WorkerGuard`][guard] which is returned by `tracing_appender::non_blocking`
//! to ensure buffered logs are flushed to their output in the case of abrupt terminations of a process.
//! See [`WorkerGuard` module][guard] for more details.
//!
//! Alternatively, you can provide *any* type that implements [`std::io::Write`][write] to
//! [`tracing_appender::non_blocking`][non_blocking]
//!
//! The [`non_blocking module`][non_blocking]'s documentation provides more detail on how to use `non_blocking`.
//!
//! [non_blocking]: ./non_blocking/index.html
//! [write]: https://doc.rust-lang.org/std/io/trait.Write.html
//! [guard]: ./non_blocking/struct.WorkerGuard.html
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
