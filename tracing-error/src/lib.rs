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
mod backtrace;
mod layer;

pub use self::backtrace::SpanTrace;
pub use self::layer::ErrorLayer;
