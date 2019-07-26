//! Adapters for connecting unstructured log records from the `log` crate into
//! the `tracing` ecosystem.
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
//! # fn main() -> Result<(), Box<Error>> {
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
//! This conversion can be done with [`TraceLogger`], a [`Subscriber`] which
//! records `tracing` spans and events and outputs log records.
//!
//! ## Caution: Mixing both conversions
//!
//! Note that logger implementations that convert log records to trace events
//! should not be used with `Subscriber`s that convert trace events _back_ into
//! log records (such as the `TraceLogger`), as doing so will result in the
//! event recursing between the subscriber and the logger forever (or, in real
//! life, probably overflowing the call stack).
//!
//! If the logging of trace events generated from log records produced by the
//! `log` crate is desired, either the `log` crate should not be used to
//! implement this logging, or an additional layer of filtering will be
//! required to avoid infinitely converting between `Event` and `log::Record`.
//!
//! [`init`]: struct.LogTracer.html#method.init
//! [`init_with_filter`]: struct.LogTracer.html#method.init_with_filter
//! [`TraceLogger`]: struct.TraceLogger.html
//! [`tracing::Event`]: https://docs.rs/tracing/0.1.3/tracing/struct.Event.html
//! [`log::Record`]: https://docs.rs/log/0.4.7/log/struct.Record.html
extern crate log;
extern crate tracing_core;
extern crate tracing_subscriber;

use lazy_static::lazy_static;

use std::{fmt, io};

use tracing_core::{
    callsite::{self, Callsite},
    dispatcher,
    field::{self, Field, Visit},
    identify_callsite,
    metadata::{Kind, Level},
    subscriber, Event, Metadata,
};

pub mod log_tracer;
pub use self::log_tracer::LogTracer;
pub mod trace_logger;
pub use self::trace_logger::{Builder as TraceLoggerBuilder, TraceLogger};

/// Format a log record as a trace event in the current span.
pub fn format_trace(record: &log::Record) -> io::Result<()> {
    let filter_meta = record.as_trace();
    if !dispatcher::get_default(|dispatch| dispatch.enabled(&filter_meta)) {
        return Ok(());
    };

    let (cs, keys) = loglevel_to_cs(record.level());

    let log_module = record.module_path();
    let log_file = record.file();
    let log_line = record.line();

    let module = log_module.as_ref().map(|s| s as &dyn field::Value);
    let file = log_file.as_ref().map(|s| s as &dyn field::Value);
    let line = log_line.as_ref().map(|s| s as &dyn field::Value);

    let meta = cs.metadata();
    Event::dispatch(
        &meta,
        &meta.fields().value_set(&[
            (&keys.message, Some(record.args() as &dyn field::Value)),
            (&keys.target, Some(&record.target())),
            (&keys.module, module),
            (&keys.file, file),
            (&keys.line, line),
        ]),
    );
    Ok(())
}

pub trait AsLog {
    type Log;
    fn as_log(&self) -> Self::Log;
}

pub trait AsTrace {
    type Trace;
    fn as_trace(&self) -> Self::Trace;
}

impl<'a> AsLog for Metadata<'a> {
    type Log = log::Metadata<'a>;
    fn as_log(&self) -> Self::Log {
        log::Metadata::builder()
            .level(self.level().as_log())
            .target(self.target())
            .build()
    }
}

struct Fields {
    message: field::Field,
    target: field::Field,
    module: field::Field,
    file: field::Field,
    line: field::Field,
}

static FIELD_NAMES: &'static [&'static str] = &[
    "message",
    "log.target",
    "log.module_path",
    "log.file",
    "log.line",
];

impl Fields {
    fn new(cs: &'static dyn Callsite) -> Self {
        let fieldset = cs.metadata().fields();
        let message = fieldset.field("message").unwrap();
        let target = fieldset.field("log.target").unwrap();
        let module = fieldset.field("log.module_path").unwrap();
        let file = fieldset.field("log.file").unwrap();
        let line = fieldset.field("log.line").unwrap();
        Fields {
            message,
            target,
            module,
            file,
            line,
        }
    }
}

macro_rules! log_cs {
    ($level:expr) => {{
        struct Callsite;
        static META: Metadata = Metadata::new(
            "log event",
            "log",
            $level,
            None,
            None,
            None,
            field::FieldSet::new(FIELD_NAMES, identify_callsite!(&Callsite)),
            Kind::EVENT,
        );

        impl callsite::Callsite for Callsite {
            fn set_interest(&self, _: subscriber::Interest) {}
            fn metadata(&self) -> &'static Metadata<'static> {
                &META
            }
        }

        &Callsite
    }};
}

static TRACE_CS: &'static dyn Callsite = log_cs!(tracing_core::Level::TRACE);
static DEBUG_CS: &'static dyn Callsite = log_cs!(tracing_core::Level::DEBUG);
static INFO_CS: &'static dyn Callsite = log_cs!(tracing_core::Level::INFO);
static WARN_CS: &'static dyn Callsite = log_cs!(tracing_core::Level::WARN);
static ERROR_CS: &'static dyn Callsite = log_cs!(tracing_core::Level::ERROR);

lazy_static! {
    static ref TRACE_FIELDS: Fields = Fields::new(TRACE_CS);
    static ref DEBUG_FIELDS: Fields = Fields::new(DEBUG_CS);
    static ref INFO_FIELDS: Fields = Fields::new(INFO_CS);
    static ref WARN_FIELDS: Fields = Fields::new(WARN_CS);
    static ref ERROR_FIELDS: Fields = Fields::new(ERROR_CS);
}

fn level_to_cs(level: &Level) -> (&'static dyn Callsite, &'static Fields) {
    match *level {
        Level::TRACE => (TRACE_CS, &*TRACE_FIELDS),
        Level::DEBUG => (DEBUG_CS, &*DEBUG_FIELDS),
        Level::INFO => (INFO_CS, &*INFO_FIELDS),
        Level::WARN => (WARN_CS, &*WARN_FIELDS),
        Level::ERROR => (ERROR_CS, &*ERROR_FIELDS),
    }
}

fn loglevel_to_cs(level: log::Level) -> (&'static dyn Callsite, &'static Fields) {
    match level {
        log::Level::Trace => (TRACE_CS, &*TRACE_FIELDS),
        log::Level::Debug => (DEBUG_CS, &*DEBUG_FIELDS),
        log::Level::Info => (INFO_CS, &*INFO_FIELDS),
        log::Level::Warn => (WARN_CS, &*WARN_FIELDS),
        log::Level::Error => (ERROR_CS, &*ERROR_FIELDS),
    }
}

impl<'a> AsTrace for log::Record<'a> {
    type Trace = Metadata<'a>;
    fn as_trace(&self) -> Self::Trace {
        let cs_id = identify_callsite!(loglevel_to_cs(self.level()).0);
        Metadata::new(
            "log record",
            self.target(),
            self.level().as_trace(),
            self.file(),
            self.line(),
            self.module_path(),
            field::FieldSet::new(FIELD_NAMES, cs_id),
            Kind::EVENT,
        )
    }
}

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

impl AsTrace for log::Level {
    type Trace = tracing_core::Level;
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

/// Extends log `Event`s to provide complete `Metadata`.
///
/// In `tracing-log`, an `Event` produced by a log (through `AsTrace`) has an hard coded
/// "log" target and no file, line, or module_path attributes. This happens because `Event`
/// requires its `Metadata` to be `'static`, while log records provide them with a generic
/// lifetime.
///
/// However, these values are stored in the `Event`'s fields and
/// the [`normalized_metadata`] method allows to build a new `Metadata`
/// that only lives as long as its source `Event`, but provides complete
/// data.
///
/// It can typically be used by `Subscriber`s when processing an `Event`,
/// to allow accessing its complete metadata in a consistent way,
/// regardless of the source of its source.
///
/// [`normalized_metadata`]: trait.NormalizeEvent.html#normalized_metadata
pub trait NormalizeEvent<'a> {
    /// If this `Event` comes from a `log`, this method provides a new
    /// normalized `Metadata` which has all available attributes
    /// from the original log, including `file`, `line`, `module_path`
    /// and `target`.
    /// Returns `None` is the `Event` is not issued from a `log`.
    fn normalized_metadata(&'a self) -> Option<Metadata<'a>>;
    /// Returns wether this `Event` represents a log (from the `log` crate)
    fn is_log(&self) -> bool;
}

impl<'a> NormalizeEvent<'a> for Event<'a> {
    fn normalized_metadata(&'a self) -> Option<Metadata<'a>> {
        let original = self.metadata();
        if self.is_log() {
            let mut fields = LogVisitor::new_for(self, level_to_cs(original.level()).1);
            self.record(&mut fields);

            Some(Metadata::new(
                "log event",
                fields.target.unwrap_or("log"),
                original.level().clone(),
                fields.file,
                fields.line.map(|l| l as u32),
                fields.module_path,
                field::FieldSet::new(&["message"], original.callsite()),
                Kind::EVENT,
            ))
        } else {
            None
        }
    }

    fn is_log(&self) -> bool {
        self.metadata().callsite() == identify_callsite!(level_to_cs(self.metadata().level()).0)
    }
}

struct LogVisitor<'a> {
    target: Option<&'a str>,
    module_path: Option<&'a str>,
    file: Option<&'a str>,
    line: Option<u64>,
    fields: &'static Fields,
}

impl<'a> LogVisitor<'a> {
    // We don't actually _use_ the provided event argument; it is simply to
    // ensure that the `LogVisitor` does not outlive the event whose fields it
    // is visiting, so that the reference casts in `record_str` are safe.
    fn new_for(_event: &'a Event<'a>, fields: &'static Fields) -> Self {
        Self {
            target: None,
            module_path: None,
            file: None,
            line: None,
            fields,
        }
    }
}

impl<'a> Visit for LogVisitor<'a> {
    fn record_debug(&mut self, _field: &Field, _value: &fmt::Debug) {}

    fn record_u64(&mut self, field: &Field, value: u64) {
        if field == &self.fields.line {
            self.line = Some(value);
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        unsafe {
            // The `Visit` API erases the string slice's lifetime. However, we
            // know it is part of the `Event` struct with a lifetime of `'a`. If
            // (and only if!) this `LogVisitor` was constructed with the same
            // lifetime parameter `'a` as the event in question, it's safe to
            // cast these string slices to the `'a` lifetime.
            if field == &self.fields.file {
                self.file = Some(&*(value as *const _));
            } else if field == &self.fields.target {
                self.target = Some(&*(value as *const _));
            } else if field == &self.fields.module {
                self.module_path = Some(&*(value as *const _));
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn log_callsite_is_correct() {
        let record = log::Record::builder()
            .args(format_args!("Error!"))
            .level(log::Level::Error)
            .target("myApp")
            .file(Some("server.rs"))
            .line(Some(144))
            .module_path(Some("server"))
            .build();

        let meta = record.as_trace();
        let (cs, _keys) = loglevel_to_cs(record.level());
        let cs_meta = cs.metadata();
        assert_eq!(meta.callsite(), cs_meta.callsite());
    }
}
