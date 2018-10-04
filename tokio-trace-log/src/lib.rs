//! Adapters for connecting unstructured log records from the `log` crate into
//! the `tokio_trace` ecosystem.
//!
//! This conversion does not convert unstructured data in log records (such as
//! values passed as format arguments to the `log!` macro) to structured
//! `tokio_trace` fields. However, it *does* attach these new events to to the
//! span that was currently executing when the record was logged. This is the
//! primary use-case for this library: making it possible to locate the log
//! records emitted by dependencies which use `log` within the context of a
//! trace.
//!
//! Note that logger implementations that convert log records to trace events
//! should not be used with `Subscriber`s that convert trace events _back_ into
//! log records (such as the `LogSubscriber` in the `tokio_trace` crate), as
//! doing so will result in the event recursing between the subscriber and the
//! logger forever (or, in real life, probably overflowing the call stack)
//!
//! If the logging of trace events generated from log records produced by the
//! `log` crate is desired, either the `log` crate should not be used to
//! implement this logging, or an additional layer of filtering will be
//! required to avoid infinitely converting between `Event` and `log::Record`.
extern crate tokio_trace;
extern crate log;

use std::{io, time::Instant};
use tokio_trace::{span, Subscriber, Event, SpanData, Meta};

/// Format a log record as a trace event in the current span.
pub fn format_trace(record: &log::Record) -> io::Result<()> {
    let meta: tokio_trace::Meta = record.as_trace();
    let event = Event {
        timestamp: Instant::now(),
        parent: tokio_trace::SpanData::current(),
        follows_from: &[],
        meta: &meta,
        field_values: &[],
        message: record.args().clone()
    };
    tokio_trace::Dispatcher::current().observe_event(&event);
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

impl<'a> AsLog for Meta<'a> {
    type Log = log::Metadata<'a>;
    fn as_log(&self) -> Self::Log {
        log::Metadata::builder()
            .level(self.level.as_log())
            .target(self.target.unwrap_or(""))
            .build()
    }
}

impl<'a> AsTrace for log::Record<'a> {
    type Trace = Meta<'a>;
    fn as_trace(&self) -> Self::Trace {
        Meta {
            name: None,
            target: Some(self.target()),
            level: self.level().as_trace(),
            module_path: self
                .module_path()
                // TODO: make symmetric
                .unwrap_or_else(|| self.target()),
            line: self.line().unwrap_or(0),
            file: self.file().unwrap_or("???"),
            field_names: &[],
        }
    }
}

impl AsLog for tokio_trace::Level {
    type Log = log::Level;
    fn as_log(&self) -> log::Level {
        match self {
            tokio_trace::Level::Error => log::Level::Error,
            tokio_trace::Level::Warn => log::Level::Warn,
            tokio_trace::Level::Info => log::Level::Info,
            tokio_trace::Level::Debug => log::Level::Debug,
            tokio_trace::Level::Trace => log::Level::Trace,
        }
    }
}

impl AsTrace for log::Level {
    type Trace = tokio_trace::Level;
    fn as_trace(&self) -> tokio_trace::Level {
        match self {
            log::Level::Error => tokio_trace::Level::Error,
            log::Level::Warn => tokio_trace::Level::Warn,
            log::Level::Info => tokio_trace::Level::Info,
            log::Level::Debug => tokio_trace::Level::Debug,
            log::Level::Trace => tokio_trace::Level::Trace,
        }
    }
}

/// A simple "logger" that converts all log records into `tokio_trace` `Event`s,
/// with an optional level filter.
#[derive(Debug)]
pub struct SimpleTraceLogger {
    filter: log::LevelFilter,
}

/// A `tokio_trace` subscriber that logs all recorded trace events.
pub struct LogSubscriber;

// ===== impl SimpleTraceLogger =====

impl SimpleTraceLogger {
    pub fn with_filter(filter: log::LevelFilter) -> Self {
       Self {
           filter,
       }
    }
}

impl Default for SimpleTraceLogger {
    fn default() -> Self {
        Self::with_filter(log::LevelFilter::Info)
    }
}

impl log::Log for SimpleTraceLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.filter
    }

    fn log(&self, record: &log::Record) {
        format_trace(record).unwrap();
    }

    fn flush(&self) {}
}

// ===== impl LogSubscriber =====

impl LogSubscriber {
    pub fn new() -> Self {
        LogSubscriber
    }
}

impl Subscriber for LogSubscriber {
    fn enabled(&self, metadata: &Meta) -> bool {
        log::logger().enabled(&metadata.as_log())
    }

    fn new_span(&self, _new_span: &span::NewSpan) -> span::Id {
        span::Id::from_u64(0)
    }

    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>) {
        let fields = event.debug_fields();
        let meta = event.meta.as_log();
        let logger = log::logger();
        let parents = event.parents().filter_map(SpanData::name).collect::<Vec<_>>();
        if logger.enabled(&meta) {
            logger.log(
                &log::Record::builder()
                    .metadata(meta)
                    .module_path(Some(event.meta.module_path))
                    .file(Some(event.meta.file))
                    .line(Some(event.meta.line))
                    .args(format_args!(
                        "[{}] {:?} {}",
                        parents.join(":"),
                        fields,
                        event.message
                    )).build(),
            );
        }
    }

    fn enter(&self, span: &SpanData, _at: Instant) {
        let logger = log::logger();
        logger.log(&log::Record::builder()
            .args(format_args!("-> {:?}", span.name()))
            .build()
        )
    }

    fn exit(&self, span: &SpanData, _at: Instant) {
        let logger = log::logger();
        logger.log(&log::Record::builder().args(format_args!("<- {:?}", span.name())).build())
    }
}
