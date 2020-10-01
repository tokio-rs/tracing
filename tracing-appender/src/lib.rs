//! Writers for logging events and spans
//!
//! # Overview
//!
//! [`tracing`][tracing] is a framework for structured, event-based diagnostic information.
//! `tracing-appender` allows events and spans to be recorded in a non-blocking manner through
//! a dedicated logging thread. It also provides a [`RollingFileAppender`][file_appender] that can
//! be used with _or_ without the non-blocking writer.
//!
//! *Compiler support: [requires `rustc` 1.42+][msrv]*
//!
//! [msrv]: #supported-rust-versions
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
//! [`Rotation::DAILY`](rolling::Rotation::DAILY) and [`Rotation::NEVER`](rolling::Rotation::NEVER) are the other available options.
//!
//! The file appender implements [`std::io::Write`][write]. To be used with [`tracing_subscriber::FmtSubscriber`][fmt_subscriber],
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
//! tracing_subscriber::fmt()
//!     .with_writer(non_blocking)
//!     .init();
//! # }
//! ```
//!
//! ## Supported Rust Versions
//!
//! Tracing is built against the latest stable release. The minimum supported
//! version is 1.42. The current Tracing version is not guaranteed to build on
//! Rust versions earlier than the minimum supported version.
//!
//! Tracing follows the same compiler support policies as the rest of the Tokio
//! project. The current stable Rust compiler and the three most recent minor
//! versions before it will always be supported. For example, if the current
//! stable compiler version is 1.45, the minimum supported version will not be
//! increased past 1.42, three minor versions prior. Increasing the minimum
//! supported compiler version is not considered a semver breaking change as
//! long as doing so complies with this policy.
//!
#![doc(html_root_url = "https://docs.rs/tracing-appender/0.1.1")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/logo-type.png",
    issue_tracker_base_url = "https://github.com/tokio-rs/tracing/issues/"
)]
#![cfg_attr(docsrs, deny(broken_intra_doc_links))]
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
