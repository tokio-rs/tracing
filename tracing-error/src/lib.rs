//! Utilities for enriching error handling with [`tracing`] diagnostic
//! information.
//!
//! [`tracing`] is a framework for instrumenting Rust programs to collect
//! scoped, structured, and async-aware diagnostics. This crate provides
//! integrations between [`tracing`] instrumentation and Rust error handling,
//! allowing error types to capture the current [`tracing`] span context when
//! they are constructed, format those contexts when they are displayed, and
//! automatically generate [`tracing`] events when errors occur.
//!
//! **Note**: This crate is currently experimental.
//!
//! [`tracing`]: https://docs.rs/tracing
#![doc(html_root_url = "https://docs.rs/tracing-error/0.1.0")]
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
mod layer;

pub use self::backtrace::SpanTrace;
pub use self::layer::ErrorLayer;
