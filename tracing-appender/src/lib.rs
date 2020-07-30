// TODO (cfirt): add rotating (01/08/2020)
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
//!  - Using a [`RollingFileAppender`][rolling_struct] to perform writes to a log file, with time based roll. This will block on writes.
//!  - Using a [`RotatingFileAppender`][rotating_struct] to perform writes to a log file, with size based rotation. This will block on writes.
//!  - Using *any* type implementing [`std::io::Write`][write] in a non-blocking fashion.
//!  - Using a combination of [`NonBlocking`][non_blocking] and one of the appenders( [`RollingFileAppender`][rolling_struct] or [`RotatingFileAppender`][rotating_struct])
//! to allow writes to a log file
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
//! The file appender implements [`std::io::Write`][write]. To be used with [`tracing_subscriber::FmtSubscriber`][fmt_subscriber],
//! it must be combined with a [`MakeWriter`][make_writer] implementation to be able to record tracing spans/event.
//!
//! The [`rolling` module][rolling]'s documentation provides more detail on how to use this file appender.
//!
//! ## Rotating File Appender
//!
//! ```rust
//! # fn docs() {
//! let file_appender = tracing_appender::rotating::mb_100("/some/directory", "prefix.log");
//! # }
//! ```
//! This creates an rotating file appender that writes to `/some/directory/prefix.log`, with a maximum
//! size of 100 MB for log file, and with 9 maximum backups log files.
//!
//! The file appender implements [`std::io::Write`][write]. To be used with [`tracing_subscriber::FmtSubscriber`][fmt_subscriber],
//! it must be combined with a [`MakeWriter`][make_writer] implementation to be able to record tracing spans/event.
//!
//! The [`rotating` module][rotating]'s documentation provides more detail on how to use this file appender.
//!
//! ## Non-Blocking Writer
//!
//! The example below demonstrates the construction of a `non_blocking` writer with `std::io::stdout()`,
//! which implements [`MakeWriter`][make_writer].
//!
//! ```rust
//! # fn doc() {
//! let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
//! tracing_subscriber::fmt()
//!     .with_writer(non_blocking)
//!     .init();
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
//! use std::io::Error;
//!
//! struct TestWriter;
//!
//! impl std::io::Write for TestWriter {
//!     fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
//!         let buf_len = buf.len();
//!         println!("{:?}", buf);
//!         Ok(buf_len)
//!     }
//!
//!     fn flush(&mut self) -> std::io::Result<()> {
//!         Ok(())
//!     }
//! }
//!
//! # fn doc() {
//! let (non_blocking, _guard) = tracing_appender::non_blocking(TestWriter);
//! tracing_subscriber::fmt()
//!     .with_writer(non_blocking)
//!     .init();
//! # }
//! ```
//!
//! The [`non_blocking` module][non_blocking]'s documentation provides more detail on how to use `non_blocking`.
//!
//! [non_blocking]: ./non_blocking/index.html
//! [write]: https://doc.rust-lang.org/std/io/trait.Write.html
//! [guard]: ./non_blocking/struct.WorkerGuard.html
//! [rolling]: ./rolling/index.html
//! [rotating]: ./rotating/index.html
//! [make_writer]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/trait.MakeWriter.html
//! [rolling_struct]: ./rolling/struct.RollingFileAppender.html
//! [rotating_struct]: ./rotating/struct.RotatingFileAppender.html

//! [fmt_subscriber]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/struct.Subscriber.html
//!
//! ## Non-Blocking Rolling File Appender
//!
//! ```rust
//! # fn docs() {
//! let file_appender = tracing_appender::rolling::hourly("/some/directory", "prefix.log");
//! let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
//! tracing_subscriber::fmt()
//!     .with_writer(non_blocking)
//!     .init();
//! # }
//! ```
#![doc(html_root_url = "https://docs.rs/tracing-appender/0.1.1")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/logo.svg",
    issue_tracker_base_url = "https://github.com/tokio-rs/tracing/issues/"
)]
#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    unreachable_pub,
    bad_style,
    const_err,
    dead_code,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    private_in_public,
    unconditional_recursion,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true
)]
use crate::non_blocking::{NonBlocking, WorkerGuard};

use std::io::Write;

mod inner;

pub mod non_blocking;

pub mod rolling;

pub mod rotating;

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

#[derive(Debug)]
pub(crate) enum Msg {
    Line(Vec<u8>),
    Shutdown,
}
