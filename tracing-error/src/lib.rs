//! Utilities for enriching error handling with [`tracing`] diagnostic
//! information.
//!
//! # Overview
//!
//! [`tracing`] is a framework for instrumenting Rust programs to collect
//! scoped, structured, and async-aware diagnostics. This crate provides
//! integrations between [`tracing`] instrumentation and Rust error handling. It
//! enables enriching error types with diagnostic information from `tracing`
//! [span] contexts, formatting those contexts when errors are displayed, and
//! automatically generate `tracing` [events] when errors occur.
//!
//! The crate provides the following:
//!
//! * [`SpanTrace`], a captured trace of the current `tracing` [span] context
//!
//! * [`ErrorLayer`], a [subscriber layer] which enables capturing `SpanTrace`s
//!
//! **Note**: This crate is currently experimental.
//!
//! *Compiler support: [requires `rustc` 1.42+][msrv]*
//!
//! [msrv]: #supported-rust-versions
//!
//! ## Feature Flags
//!
//! - `traced-error` - Enables the [`TracedError`] type and related Traits
//!     - [`InstrumentResult`] and [`InstrumentError`] extension traits, which
//!     provide an [`in_current_span()`] method for bundling errors with a
//!     [`SpanTrace`].
//!     - [`ExtractSpanTrace`] extension trait, for extracting `SpanTrace`s from
//!     behind `dyn Error` trait objects.
//!
//! ## Usage
//!
//! `tracing-error` provides the [`SpanTrace`] type, which captures the current
//! `tracing` span context when it is constructed and allows it to be displayed
//! at a later time.
//!
//! For example:
//!
//! ```rust
//! use std::{fmt, error::Error};
//! use tracing_error::SpanTrace;
//!
//! #[derive(Debug)]
//! pub struct MyError {
//!     context: SpanTrace,
//!     // ...
//! }
//!
//! impl fmt::Display for MyError {
//!     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//!         // ... format other parts of the error ...
//!
//!         self.context.fmt(f)?;
//!
//!         // ... format other error context information, cause chain, etc ...
//!         # Ok(())
//!     }
//! }
//!
//! impl Error for MyError {}
//!
//! impl MyError {
//!     pub fn new() -> Self {
//!         Self {
//!             context: SpanTrace::capture(),
//!             // ... other error information ...
//!         }
//!     }
//! }
//! ```
//!
//! This crate also provides [`TracedError`], for attaching a [`SpanTrace`] to
//! an existing error. The easiest way to wrap errors in `TracedError` is to
//! either use the [`InstrumentResult`] and [`InstrumentError`] traits or the
//! `From`/`Into` traits.
//!
//! ```rust
//! # use std::error::Error;
//! use tracing_error::prelude::*;
//!
//! # fn fake_main() -> Result<(), Box<dyn Error>> {
//! std::fs::read_to_string("myfile.txt").in_current_span()?;
//! # Ok(())
//! # }
//! ```
//!
//! Once an error has been wrapped with with a [`TracedError`] the [`SpanTrace`]
//! can be extracted one of 3 ways: either via [`TracedError`]'s
//! `Display`/`Debug` implementations, or via the [`ExtractSpanTrace`] trait.
//!
//! For example, here is how one might print the errors but specialize the
//! printing when the error is a placeholder for a wrapping [`SpanTrace`]:
//!
//! ```rust
//! use std::error::Error;
//! use tracing_error::ExtractSpanTrace as _;
//!
//! fn print_extracted_spantraces(error: &(dyn Error + 'static)) {
//!     let mut error = Some(error);
//!     let mut ind = 0;
//!
//!     eprintln!("Error:");
//!
//!     while let Some(err) = error {
//!         if let Some(spantrace) = err.span_trace() {
//!             eprintln!("found a spantrace:\n{}", spantrace);
//!         } else {
//!             eprintln!("{:>4}: {}", ind, err);
//!         }
//!
//!         error = err.source();
//!         ind += 1;
//!     }
//! }
//!
//! ```
//!
//! Whereas here, we can still display the content of the `SpanTraces` without
//! any special casing by simply printing all errors in our error chain.
//!
//! ```rust
//! use std::error::Error;
//!
//! fn print_naive_spantraces(error: &(dyn Error + 'static)) {
//!     let mut error = Some(error);
//!     let mut ind = 0;
//!
//!     eprintln!("Error:");
//!
//!     while let Some(err) = error {
//!         eprintln!("{:>4}: {}", ind, err);
//!         error = err.source();
//!         ind += 1;
//!     }
//! }
//! ```
//!
//! Applications that wish to use `tracing-error`-enabled errors should
//! construct an [`ErrorLayer`] and add it to their [collector` in order to
//! enable capturing [`SpanTrace`]s. For example:
//!
//! ```rust
//! use tracing_error::ErrorLayer;
//! use tracing_subscriber::prelude::*;
//!
//! fn main() {
//!     let subscriber = tracing_subscriber::Registry::default()
//!         // any number of other subscriber layers may be added before or
//!         // after the `ErrorLayer`...
//!         .with(ErrorLayer::default());
//!
//!     // set the subscriber as the default for the application
//!     tracing::collect::set_global_default(subscriber);
//! }
//! ```
//!
//! [`SpanTrace`]: struct.SpanTrace.html
//! [`ErrorLayer`]: struct.ErrorLayer.html
//! [`TracedError`]: struct.TracedError.html
//! [`InstrumentResult`]: trait.InstrumentResult.html
//! [`InstrumentError`]: trait.InstrumentError.html
//! [`ExtractSpanTrace`]: trait.ExtractSpanTrace.html
//! [`in_current_span()`]: trait.InstrumentResult.html#tymethod.in_current_span
//! [span]: https://docs.rs/tracing/latest/tracing/span/index.html
//! [events]: https://docs.rs/tracing/latest/tracing/struct.Event.html
//! [collector]: https://docs.rs/tracing/latest/tracing/trait.Collect.html
//! [subscriber layer]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html
//! [`tracing`]: https://docs.rs/tracing
//! [`std::error::Error`]: https://doc.rust-lang.org/stable/std/error/trait.Error.html
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
#![cfg_attr(docsrs, feature(doc_cfg), deny(broken_intra_doc_links))]
#![doc(html_root_url = "https://docs.rs/tracing-error/0.1.2")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/logo-type.png",
    html_favicon_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/favicon.ico",
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
mod backtrace;
#[cfg(feature = "traced-error")]
mod error;
mod layer;

pub use self::backtrace::{SpanTrace, SpanTraceStatus};
#[cfg(feature = "traced-error")]
pub use self::error::{ExtractSpanTrace, InstrumentError, InstrumentResult, TracedError};
pub use self::layer::ErrorLayer;

#[cfg(feature = "traced-error")]
#[cfg_attr(docsrs, doc(cfg(feature = "traced-error")))]
pub mod prelude {
    //! The `tracing-error` prelude.
    //!
    //! This brings into scope the `InstrumentError, `InstrumentResult`, and `ExtractSpanTrace`
    //! extension traits. These traits allow attaching `SpanTrace`s to errors and
    //! subsequently retrieving them from `dyn Error` trait objects.

    pub use crate::{ExtractSpanTrace as _, InstrumentError as _, InstrumentResult as _};
}
