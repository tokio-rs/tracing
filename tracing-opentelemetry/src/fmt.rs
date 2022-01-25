use opentelemetry::trace as otel;
use std::fmt;
use tracing_core::{Collect, Event, Level};
#[cfg(feature = "tracing-log")]
use tracing_log::NormalizeEvent;
use tracing_subscriber::{
    fmt::{time, FmtContext, FormatEvent, FormatFields, FormattedFields},
    registry::LookupSpan,
};

#[cfg(feature = "ansi")]
use ansi_term::{Colour, Style};
use otel::TraceContextExt;

/// An [event formatter] for [`tracing_subscriber::fmt`] that encriches logged
/// events with OpenTelemetry-specific data.
///
/// The output format is essentially the same as `tracing-subscriber`'s [default
/// `Full` event format], but with the addition of the current span context's
/// OpenTelemetry trace ID at the beginning of each log line. Additionally, the
/// OpenTelemetry span IDs of each individual `tracing` span in the current
/// context [can be displayed as well](with_span_ids).
///
/// This formatter supports most of the same [configuration options] supported
/// by the default formatter in `tracing_subscriber::fmt`.
///
/// # Usage
///
/// **Note**: This event formatter *must* be used alongside the
/// [OpenTelemetry subscriber] also provided by this crate. Otherwise, stored
/// span data will not be enriched with OpenTelemetry span and trace IDs.
///
///
/// [event formatter]: tracing_subscriber::fmt::FormatEvent
/// [default `Full` event format]: tracing_subscriber::fmt::format::Full
/// [configuration options]: #implementations
#[derive(Debug)]
pub struct OtelEvent<T = time::SystemTime> {
    timer: T,
    ansi: bool,
    display_span_ids: bool,
    display_remote_parents: bool,
    display_target: bool,
    display_level: bool,
    display_thread_id: bool,
    display_thread_name: bool,
}

impl OtelEvent {
    /// Returns a new `OtelEvent` formatter with the default configuration.
    ///
    /// By default, this includes the [targets] and [levels] of formatted
    /// formatted output, and thread names and IDs are
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T> OtelEvent<T> {
    pub fn with_span_ids(self, display_span_ids: bool) -> Self {
        Self {
            display_span_ids,
            ..self
        }
    }

    pub fn with_remote_parents(self, display_remote_parents: bool) -> Self {
        Self {
            display_remote_parents,
            ..self
        }
    }

    // pub fn with_trace_states(self, display_trace_states: bool) -> Self {}

    /// Use the given [`timer`] for log message timestamps.
    ///
    /// See [`tracing_subscriber::fmt::time`] for the provided timer
    /// implementations.
    ///
    /// [`timer`]: tracing_subscriber::fmt::time::FormatTime
    pub fn with_timer<T2>(self, timer: T2) -> OtelEvent<T2> {
        OtelEvent {
            timer,
            #[cfg(feature = "ansi")]
            ansi: self.ansi,
            display_span_ids: self.display_span_ids,
            display_remote_parents: self.display_remote_parents,
            display_target: self.display_target,
            display_level: self.display_level,
            display_thread_id: self.display_thread_id,
            display_thread_name: self.display_thread_name,
        }
    }

    /// Do not emit timestamps with log messages.
    pub fn without_time(self) -> OtelEvent<()> {
        OtelEvent {
            timer: (),
            #[cfg(feature = "ansi")]
            ansi: self.ansi,
            display_span_ids: self.display_span_ids,
            display_remote_parents: self.display_remote_parents,
            display_target: self.display_target,
            display_level: self.display_level,
            display_thread_id: self.display_thread_id,
            display_thread_name: self.display_thread_name,
        }
    }

    /// Enable ANSI terminal colors for formatted output.
    #[cfg_attr(docsrs, doc(cfg(feature = "ansi")))]
    #[cfg(feature = "ansi")]
    pub fn with_ansi(self, ansi: bool) -> Self {
        Self { ansi, ..self }
    }

    /// Sets whether or not an event's target is displayed.
    pub fn with_target(self, display_target: bool) -> Self {
        Self {
            display_target,
            ..self
        }
    }

    /// Sets whether or not an event's level is displayed.
    pub fn with_level(self, display_level: bool) -> Self {
        Self {
            display_level,
            ..self
        }
    }

    /// Sets whether or not the [thread ID] of the current thread is displayed
    /// when formatting events
    ///
    /// [thread ID]: std::thread::ThreadId
    pub fn with_thread_ids(self, display_thread_id: bool) -> Self {
        Self {
            display_thread_id,
            ..self
        }
    }

    /// Sets whether or not the [name] of the current thread is displayed
    /// when formatting events
    ///
    /// [name]: std::thread#naming-threads
    pub fn with_thread_names(self, display_thread_name: bool) -> Self {
        Self {
            display_thread_name,
            ..self
        }
    }

    fn format_timestamp(&self, writer: &mut dyn fmt::Write) -> fmt::Result
    where
        T: time::FormatTime,
    {
        #[cfg(feature = "ansi")]
        if self.ansi {
            let style = Style::new().dimmed();
            write!(writer, "{}", style.prefix())?;
            self.timer.format_time(writer)?;
            write!(writer, "{}", style.suffix())?;
            return writer.write_char(' ');
        }

        self.timer.format_time(writer)?;
        writer.write_char(' ')
    }

    #[cfg(not(feature = "ansi"))]
    fn format_timestamp(&self, writer: &mut dyn fmt::Write) -> fmt::Result
    where
        T: time::FormatTime,
    {
        self.timer.format_time(writer)?;
        writer.write_char(' ')?;
        Ok(())
    }

    fn format_level(&self, level: Level, writer: &mut dyn fmt::Write) -> fmt::Result {
        const TRACE_STR: &'static str = "TRACE";
        const DEBUG_STR: &'static str = "DEBUG";
        const INFO_STR: &'static str = " INFO";
        const WARN_STR: &'static str = " WARN";
        const ERROR_STR: &'static str = "ERROR";
        #[cfg(feature = "ansi")]
        {
            if self.ansi {
                return match level {
                    Level::TRACE => write!(writer, "{} ", Colour::Purple.paint(TRACE_STR)),
                    Level::DEBUG => write!(writer, "{} ", Colour::Blue.paint(DEBUG_STR)),
                    Level::INFO => write!(writer, "{} ", Colour::Green.paint(INFO_STR)),
                    Level::WARN => write!(writer, "{} ", Colour::Yellow.paint(WARN_STR)),
                    Level::ERROR => write!(writer, "{} ", Colour::Red.paint(ERROR_STR)),
                };
            }
        }

        match level {
            Level::TRACE => writer.write_str(TRACE_STR),
            Level::DEBUG => writer.write_str(DEBUG_STR),
            Level::INFO => writer.write_str(INFO_STR),
            Level::WARN => writer.write_str(WARN_STR),
            Level::ERROR => writer.write_str(ERROR_STR),
        }?;
        writer.write_char(' ')
    }
}

impl Default for OtelEvent {
    fn default() -> Self {
        Self {
            timer: time::SystemTime,
            #[cfg(feature = "ansi")]
            ansi: true,
            display_remote_parents: true,
            display_span_ids: false,
            display_target: true,
            display_level: true,
            display_thread_id: false,
            display_thread_name: false,
        }
    }
}

impl<S, N, T> FormatEvent<S, N> for OtelEvent<T>
where
    S: Collect + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
    T: time::FormatTime,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        writer: &mut dyn fmt::Write,
        event: &Event<'_>,
    ) -> fmt::Result {
        #[cfg(feature = "tracing-log")]
        let normalized_meta = event.normalized_metadata();
        #[cfg(feature = "tracing-log")]
        let meta = normalized_meta.as_ref().unwrap_or_else(|| event.metadata());
        #[cfg(not(feature = "tracing-log"))]
        let meta = event.metadata();

        let span = event
            .parent()
            .and_then(|id| ctx.span(&id))
            .or_else(|| ctx.lookup_current());

        let trace_id = {
            span.as_ref()
                .and_then(|span| {
                    let ext = span.extensions();
                    let builder = ext.get::<otel::SpanBuilder>()?;
                    let tid = builder.trace_id.unwrap_or_else(otel::TraceId::invalid);
                    if tid != otel::TraceId::invalid() {
                        return Some(tid);
                    }

                    let parent = builder.parent_context.as_ref()?;
                    let parent_span = parent.span().span_context();
                    if parent_span.is_remote() {
                        parent.remote_span_context().map(|cx| cx.trace_id())
                    } else {
                        Some(parent_span.trace_id())
                    }
                })
                .unwrap_or_else(otel::TraceId::invalid)
        };

        let bold = if self.ansi {
            Style::new().bold()
        } else {
            Style::new()
        };

        let dimmed = if self.ansi {
            Style::new().dimmed()
        } else {
            Style::new()
        };

        write!(
            writer,
            "{}{:032x}{} ",
            bold.paint("["),
            trace_id.to_u128(),
            bold.paint("]")
        )?;

        self.format_timestamp(writer)?;

        self.format_level(*meta.level(), writer)?;

        if self.display_thread_name {
            let current_thread = std::thread::current();
            match current_thread.name() {
                Some(name) => {
                    write!(writer, "{} ", FmtThreadName::new(name))?;
                }
                // fall-back to thread id when name is absent and ids are not enabled
                None if !self.display_thread_id => {
                    write!(writer, "{:0>2?} ", current_thread.id())?;
                }
                _ => {}
            }
        }

        if self.display_thread_id {
            write!(writer, "{:0>2?} ", std::thread::current().id())?;
        }

        let mut wrote_spans = false;
        for span in ctx.scope() {
            let meta = span.metadata();
            let ext = span.extensions();
            let fields = &ext
                .get::<FormattedFields<N>>()
                .expect("Unable to find FormattedFields in extensions; this is a bug");

            if let Some(builder) = ext.get::<otel::SpanBuilder>() {
                if self.display_remote_parents {
                    if let Some(remote_parent) = builder
                        .parent_context
                        .as_ref()
                        .and_then(TraceContextExt::remote_span_context)
                    {
                        write!(writer, "{}", dimmed.prefix())?;
                        if self.display_span_ids {
                            write!(writer, "[{:x}]", remote_parent.span_id().to_u64())?;
                        }
                        // TODO(eliza): it would be nice if we could not print
                        // anything here if the remote parent's trace state is
                        // empty...but there's no way to check currently.
                        write!(
                            writer,
                            "{{{:?}}}{}:",
                            remote_parent.trace_state(),
                            dimmed.suffix()
                        )?;
                    }
                }
                write!(writer, "{}", bold.paint(meta.name()))?;
                if self.display_span_ids {
                    if let Some(id) = builder.span_id {
                        write!(
                            writer,
                            "{}{:x}{}",
                            bold.paint("["),
                            id.to_u64(),
                            bold.paint("]")
                        )?;
                    }
                }
            } else {
                write!(writer, "{}", bold.paint(meta.name()))?;
            }
            if !fields.is_empty() {
                write!(writer, "{}{}{}", bold.paint("{"), fields, bold.paint("}"))?;
            }

            wrote_spans = true;
            writer.write_char(':')?;
        }

        if wrote_spans {
            writer.write_char(' ')?;
        }

        if self.display_target {
            write!(writer, "{}: ", meta.target())?;
        }

        ctx.format_fields(writer, event)?;
        writeln!(writer)
    }
}

struct FmtThreadName<'a> {
    name: &'a str,
}

impl<'a> FmtThreadName<'a> {
    pub(crate) fn new(name: &'a str) -> Self {
        Self { name }
    }
}

impl<'a> fmt::Display for FmtThreadName<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use std::sync::atomic::{
            AtomicUsize,
            Ordering::{AcqRel, Acquire, Relaxed},
        };

        // Track the longest thread name length we've seen so far in an atomic,
        // so that it can be updated by any thread.
        static MAX_LEN: AtomicUsize = AtomicUsize::new(0);
        let len = self.name.len();
        // Snapshot the current max thread name length.
        let mut max_len = MAX_LEN.load(Relaxed);

        while len > max_len {
            // Try to set a new max length, if it is still the value we took a
            // snapshot of.
            match MAX_LEN.compare_exchange(max_len, len, AcqRel, Acquire) {
                // We successfully set the new max value
                Ok(_) => break,
                // Another thread set a new max value since we last observed
                // it! It's possible that the new length is actually longer than
                // ours, so we'll loop again and check whether our length is
                // still the longest. If not, we'll just use the newer value.
                Err(actual) => max_len = actual,
            }
        }

        // pad thread name using `max_len`
        write!(f, "{:>width$}", self.name, width = max_len)
    }
}

#[cfg(not(feature = "ansi"))]
struct Style;

#[cfg(not(feature = "ansi"))]
impl Style {
    fn new() -> Self {
        Style
    }
    fn paint(&self, d: impl fmt::Display) -> impl fmt::Display {
        d
    }
    fn prefix(&self) -> impl fmt::Display {
        ""
    }

    fn suffix(&self) -> impl fmt::Display {
        ""
    }
}
