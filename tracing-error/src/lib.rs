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
//! *Compiler support: requires `rustc` 1.39+*
//!
//! ## Usage
//!
//! Currently, `tracing-error` provides the [`SpanTrace`] type, which captures
//! the current `tracing` span context when it is constructed and allows it to
//! be displayed at a later time.
//!
//! This crate does not _currently_ provide any actual error types implementing
//! `std::error::Error`. Instead, user-constructed errors or libraries
//! implementing error types may capture a [`SpanTrace`] and include it as part
//! of their error types.
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
//! In the future, this crate may also provide its own `Error` types as well,
//! for users who do not wish to use other error-handling libraries.
//!
//! Applications that wish to use `tracing-error`-enabled errors should
//! construct an [`ErrorLayer`] and add it to their [`Subscriber`] in order to
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
//!     tracing::subscriber::set_global_default(subscriber);
//! }
//! ```
//!
//! [`SpanTrace`]: struct.SpanTrace.html
//! [`ErrorLayer`]: struct.ErrorLayer.html
//! [span]: https://docs.rs/tracing/latest/tracing/span/index.html
//! [event]: https://docs.rs/tracing/latest/tracing/struct.Event.html
//! [subscriber layer]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html
//! [`tracing`]: https://docs.rs/tracing
#![doc(html_root_url = "https://docs.rs/tracing-error/0.1.1")]
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
///
pub mod heap_error;
mod layer;
///
pub mod stack_error;

pub use self::backtrace::SpanTrace;
pub use self::layer::ErrorLayer;

/// The `tracing-error` prelude.
///
/// This brings into scope the `Instrument` and `SpanTraceExt` extension traits that are used to
/// attach Spantraces to errors and subsequently retrieve them from dyn Errors.
pub mod prelude {
    pub use crate::heap_error::{Instrument as _, SpanTraceExt as _};
    // pub use crate::stack_error::{Instrument as _, SpanTraceExt as _};
}
