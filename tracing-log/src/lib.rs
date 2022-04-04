//! Adapters for connecting unstructured log records from the `log` crate into
//! the `tracing` ecosystem.
//!
//! # Overview
//!
//! [`tracing`] is a framework for instrumenting Rust programs with context-aware,
//! structured, event-based diagnostic information. This crate provides
//! compatibility layers for using `tracing` alongside the logging facade provided
//! by the [`log`] crate.
//!
//! This crate provides:
//!
//! - [`AsTrace`] and [`AsLog`] traits for converting between `tracing` and `log` types.
//! - [`LogTracer`], a [`log::Log`] implementation that consumes [`log::Record`]s
//!   and outputs them as [`tracing::Event`].
//! - An [`env_logger`] module, with helpers for using the [`env_logger` crate]
//!   with `tracing` (optional, enabled by the `env-logger` feature).
//!
//! *Compiler support: [requires `rustc` 1.49+][msrv]*
//!
//! [msrv]: #supported-rust-versions
//!
//! # Usage
//!
//! ## Convert log records to tracing `Event`s
//!
//! To convert [`log::Record`]s as [`tracing::Event`]s, set `LogTracer` as the default
//! logger by calling its [`init`] or [`init_with_filter`] methods.
//!
//! ```rust
//! # use std::error::Error;
//! use tracing_log::LogTracer;
//! use log;
//!
//! # fn main() -> Result<(), Box<dyn Error>> {
//! LogTracer::init()?;
//!
//! // will be available for Subscribers as a tracing Event
//! log::trace!("an example trace log");
//! # Ok(())
//! # }
//! ```
//!
//! This conversion does not convert unstructured data in log records (such as
//! values passed as format arguments to the `log!` macro) to structured
//! `tracing` fields. However, it *does* attach these new events to to the
//! span that was currently executing when the record was logged. This is the
//! primary use-case for this library: making it possible to locate the log
//! records emitted by dependencies which use `log` within the context of a
//! trace.
//!
//! ## Convert tracing `Event`s to logs
//!
//! Enabling the ["log" and "log-always" feature flags][flags] on the `tracing`
//! crate will cause all `tracing` spans and events to emit `log::Record`s as
//! they occur.
//!
//! ## Caution: Mixing both conversions
//!
//! Note that logger implementations that convert log records to trace events
//! should not be used with `Collector`s that convert trace events _back_ into
//! log records, as doing so will result in the event recursing between the
//! collector and the logger forever (or, in real life, probably overflowing
//! the call stack).
//!
//! If the logging of trace events generated from log records produced by the
//! `log` crate is desired, either the `log` crate should not be used to
//! implement this logging, or an additional subscriber of filtering will be
//! required to avoid infinitely converting between `Event` and `log::Record`.
//!
//! # Feature Flags
//! * `log-tracer`: enables the `LogTracer` type (on by default)
//! * `env_logger`: enables the `env_logger` module, with helpers for working
//!   with the [`env_logger` crate].
//!
//! ## Supported Rust Versions
//!
//! Tracing is built against the latest stable release. The minimum supported
//! version is 1.49. The current Tracing version is not guaranteed to build on
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
//! [`init`]: LogTracer::init()
//! [`init_with_filter`]: LogTracer::init_with_filter()
//! [`tracing`]: https://crates.io/crates/tracing
//! [`log`]: https://crates.io/crates/log
//! [`env_logger` crate]: https://crates.io/crates/env-logger
//! [`tracing::Collector`]: tracing::Collect
//! [`tracing::Event`]: tracing_core::Event
//! [`Collect`]: tracing::Collect
//! [flags]: https://docs.rs/tracing/latest/tracing/#crate-feature-flags
#![doc(html_root_url = "https://docs.rs/tracing-log/0.1.1")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/logo-type.png",
    html_favicon_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/favicon.ico",
    issue_tracker_base_url = "https://github.com/tokio-rs/tracing/issues/"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
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

use std::io;

use tracing_core::{dispatch, dynamic_metadata, field, Event, Metadata};

#[cfg(feature = "log-tracer")]
#[cfg_attr(docsrs, doc(cfg(feature = "log-tracer")))]
pub mod log_tracer;

#[cfg(feature = "log-tracer")]
#[cfg_attr(docsrs, doc(cfg(feature = "log-tracer")))]
#[doc(inline)]
pub use self::log_tracer::LogTracer;

#[cfg(feature = "env_logger")]
#[cfg_attr(docsrs, doc(cfg(feature = "env_logger")))]
pub mod env_logger;

pub use log;

/// Format a log record as a trace event in the current span.
pub fn format_trace(record: &log::Record<'_>) -> io::Result<()> {
    dispatch_record(record);
    Ok(())
}

// XXX(eliza): this is factored out so that we don't have to deal with the pub
// function `format_trace`'s `Result` return type...maybe we should get rid of
// that in 0.2...
pub(crate) fn dispatch_record(record: &log::Record<'_>) {
    dispatch::get_default(|dispatch| {
        let filter_meta = record.as_trace();
        if !dispatch.enabled(&filter_meta) {
            return;
        }

        let (keys, meta) = level_to_meta(record.level());

        let log_module = record.module_path();
        let log_file = record.file();
        let log_line = record.line();

        let module = log_module.as_ref().map(|s| s as &dyn field::Value);
        let file = log_file.as_ref().map(|s| s as &dyn field::Value);
        let line = log_line.as_ref().map(|s| s as &dyn field::Value);

        dispatch.event(&Event::new(
            meta,
            // The use of the normalized field set here is critical; it's what
            // allows the Event to know to skip over them unless asked for.
            &meta.fields().value_set(&[
                (&keys.name, Some(&"log event" as &dyn field::Value)),
                (&keys.target, Some(&record.target())),
                (&keys.file, file),
                (&keys.line, line),
                (&keys.module, module),
                (&keys.message, Some(record.args())),
            ]),
        ));
    });
}

/// Trait implemented for `tracing` types that can be converted to a `log`
/// equivalent.
pub trait AsLog: crate::sealed::Sealed {
    /// The `log` type that this type can be converted into.
    type Log;
    /// Returns the `log` equivalent of `self`.
    fn as_log(&self) -> Self::Log;
}

/// Trait implemented for `log` types that can be converted to a `tracing`
/// equivalent.
pub trait AsTrace: crate::sealed::Sealed {
    /// The `tracing` type that this type can be converted into.
    type Trace;
    /// Returns the `tracing` equivalent of `self`.
    fn as_trace(&self) -> Self::Trace;
}

impl<'a> crate::sealed::Sealed for Metadata<'a> {}

impl<'a> AsLog for Metadata<'a> {
    type Log = log::Metadata<'a>;
    fn as_log(&self) -> Self::Log {
        log::Metadata::builder()
            .level(self.level().as_log())
            .target(self.target())
            .build()
    }
}
impl<'a> crate::sealed::Sealed for log::Metadata<'a> {}

impl<'a> AsTrace for log::Metadata<'a> {
    type Trace = Metadata<'a>;
    fn as_trace(&self) -> Self::Trace {
        let dynamic = level_to_meta(self.level()).1;
        Metadata::new(
            "log event",
            self.target(),
            self.level().as_trace(),
            None,
            None,
            None,
            dynamic.fields(),
            dynamic.kind().clone(),
        )
    }
}

struct Fields {
    name: field::Field,
    target: field::Field,
    file: field::Field,
    line: field::Field,
    module: field::Field,
    message: field::Field,
}

impl Fields {
    fn new(meta: &Metadata<'_>) -> Self {
        let mut fields = meta.fields_prenormal().iter();
        let name = fields.next().unwrap();
        let target = fields.next().unwrap();
        let _level = fields.next().unwrap();
        let file = fields.next().unwrap();
        let line = fields.next().unwrap();
        let module = fields.next().unwrap();
        let message = fields.next().unwrap();
        assert!(fields.next().is_none());
        Fields {
            name,
            target,
            file,
            line,
            module,
            message,
        }
    }
}

fn level_to_meta(level: log::Level) -> (Fields, &'static Metadata<'static>) {
    match level {
        log::Level::Trace => {
            let meta = dynamic_metadata! {
                name,
                target: "log",
                level: tracing_core::Level::TRACE,
                file,
                line,
                module,
                message,
            };
            (Fields::new(meta), meta)
        }
        log::Level::Debug => {
            let meta = dynamic_metadata! {
                name,
                target: "log",
                level: tracing_core::Level::DEBUG,
                file,
                line,
                module,
                message,
            };
            (Fields::new(meta), meta)
        }
        log::Level::Info => {
            let meta = dynamic_metadata! {
                name,
                target: "log",
                level: tracing_core::Level::INFO,
                file,
                line,
                module,
                message,
            };
            (Fields::new(meta), meta)
        }
        log::Level::Warn => {
            let meta = dynamic_metadata! {
                name,
                target: "log",
                level: tracing_core::Level::WARN,
                file,
                line,
                module,
                message,
            };
            (Fields::new(meta), meta)
        }
        log::Level::Error => {
            let meta = dynamic_metadata! {
                name,
                target: "log",
                level: tracing_core::Level::ERROR,
                file,
                line,
                module,
                message,
            };
            (Fields::new(meta), meta)
        }
    }
}

impl<'a> crate::sealed::Sealed for log::Record<'a> {}

impl<'a> AsTrace for log::Record<'a> {
    type Trace = Metadata<'a>;
    fn as_trace(&self) -> Self::Trace {
        let dynamic = level_to_meta(self.level()).1;
        Metadata::new(
            "log event",
            self.target(),
            self.level().as_trace(),
            self.file(),
            self.line(),
            self.module_path(),
            dynamic.fields(),
            dynamic.kind().clone(),
        )
    }
}

impl crate::sealed::Sealed for tracing_core::Level {}

impl AsLog for tracing_core::Level {
    type Log = log::Level;
    fn as_log(&self) -> log::Level {
        match *self {
            tracing_core::Level::ERROR => log::Level::Error,
            tracing_core::Level::WARN => log::Level::Warn,
            tracing_core::Level::INFO => log::Level::Info,
            tracing_core::Level::DEBUG => log::Level::Debug,
            tracing_core::Level::TRACE => log::Level::Trace,
        }
    }
}

impl crate::sealed::Sealed for log::Level {}

impl AsTrace for log::Level {
    type Trace = tracing_core::Level;
    #[inline]
    fn as_trace(&self) -> tracing_core::Level {
        match self {
            log::Level::Error => tracing_core::Level::ERROR,
            log::Level::Warn => tracing_core::Level::WARN,
            log::Level::Info => tracing_core::Level::INFO,
            log::Level::Debug => tracing_core::Level::DEBUG,
            log::Level::Trace => tracing_core::Level::TRACE,
        }
    }
}

impl crate::sealed::Sealed for log::LevelFilter {}

impl AsTrace for log::LevelFilter {
    type Trace = tracing_core::LevelFilter;
    #[inline]
    fn as_trace(&self) -> tracing_core::LevelFilter {
        match self {
            log::LevelFilter::Off => tracing_core::LevelFilter::OFF,
            log::LevelFilter::Error => tracing_core::LevelFilter::ERROR,
            log::LevelFilter::Warn => tracing_core::LevelFilter::WARN,
            log::LevelFilter::Info => tracing_core::LevelFilter::INFO,
            log::LevelFilter::Debug => tracing_core::LevelFilter::DEBUG,
            log::LevelFilter::Trace => tracing_core::LevelFilter::TRACE,
        }
    }
}

impl crate::sealed::Sealed for tracing_core::LevelFilter {}

impl AsLog for tracing_core::LevelFilter {
    type Log = log::LevelFilter;
    #[inline]
    fn as_log(&self) -> Self::Log {
        match *self {
            tracing_core::LevelFilter::OFF => log::LevelFilter::Off,
            tracing_core::LevelFilter::ERROR => log::LevelFilter::Error,
            tracing_core::LevelFilter::WARN => log::LevelFilter::Warn,
            tracing_core::LevelFilter::INFO => log::LevelFilter::Info,
            tracing_core::LevelFilter::DEBUG => log::LevelFilter::Debug,
            tracing_core::LevelFilter::TRACE => log::LevelFilter::Trace,
        }
    }
}

mod sealed {
    pub trait Sealed {}
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_callsite(level: log::Level) {
        let record = log::Record::builder()
            .args(format_args!("Error!"))
            .level(level)
            .target("myApp")
            .file(Some("server.rs"))
            .line(Some(144))
            .module_path(Some("server"))
            .build();

        let meta = record.as_trace();
        let (_keys, cs_meta) = level_to_meta(record.level());
        assert_eq!(
            meta.callsite(),
            cs_meta.callsite(),
            "actual: {:#?}\nexpected: {:#?}",
            meta,
            cs_meta
        );
        assert_eq!(meta.level(), &level.as_trace());
    }

    #[test]
    fn error_callsite_is_correct() {
        test_callsite(log::Level::Error);
    }

    #[test]
    fn warn_callsite_is_correct() {
        test_callsite(log::Level::Warn);
    }

    #[test]
    fn info_callsite_is_correct() {
        test_callsite(log::Level::Info);
    }

    #[test]
    fn debug_callsite_is_correct() {
        test_callsite(log::Level::Debug);
    }

    #[test]
    fn trace_callsite_is_correct() {
        test_callsite(log::Level::Trace);
    }
}
