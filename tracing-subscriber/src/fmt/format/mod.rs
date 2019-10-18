//! Formatters for logging `tracing` events.
use super::span;
use super::time::{self, FormatTime, SystemTime};
use crate::field::{MakeOutput, MakeVisitor, RecordFields, VisitFmt, VisitOutput};

use std::fmt::{self, Write};
use std::marker::PhantomData;
use tracing_core::{
    field::{self, Field, Visit},
    Event, Level,
};

#[cfg(feature = "tracing-log")]
use tracing_log::NormalizeEvent;

#[cfg(feature = "ansi")]
use ansi_term::{Colour, Style};

#[cfg(feature = "json")]
mod json;

#[cfg(feature = "json")]
pub use json::*;

/// A type that can format a tracing `Event` for a `fmt::Write`.
///
/// `FormatEvent` is primarily used in the context of [`FmtSubscriber`]. Each time an event is
/// dispatched to [`FmtSubscriber`], the subscriber forwards it to its associated `FormatEvent` to
/// emit a log message.
///
/// This trait is already implemented for function pointers with the same
/// signature as `format_event`.
///
/// [`FmtSubscriber`]: ../fmt/struct.Subscriber.html
pub trait FormatEvent<N> {
    /// Write a log message for `Event` in `Context` to the given `Write`.
    fn format_event(
        &self,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        event: &Event<'_>,
    ) -> fmt::Result;
}

impl<N> FormatEvent<N>
    for fn(&span::Context<'_, N>, &mut dyn fmt::Write, &Event<'_>) -> fmt::Result
{
    fn format_event(
        &self,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        event: &Event<'_>,
    ) -> fmt::Result {
        (*self)(ctx, writer, event)
    }
}
/// A type that can format a [set of fields] to a `fmt::Write`.
///
/// `FormatFields` is primarily used in the context of [`FmtSubscriber`]. Each
/// time a span or event with fields is recorded, the subscriber will format
/// those fields with its associated `FormatFields` implementation.
///
/// [set of fields]: ../field/trait.RecordFields.html
/// [`FmtSubscriber`]: ../fmt/struct.Subscriber.html
pub trait FormatFields<'writer> {
    /// Format the provided `fields` to the provided `writer`, returning a result.
    fn format_fields<R: RecordFields>(
        &self,
        writer: &'writer mut dyn fmt::Write,
        fields: R,
    ) -> fmt::Result;
}

/// Returns a [`FormatFields`] implementation that formats fields using the
/// provided function or closure.
///
/// [`FormatFields`]: trait.FormatFields.html
pub fn debug_fn<F>(f: F) -> FieldFn<F>
where
    F: Fn(&mut dyn fmt::Write, &Field, &dyn fmt::Debug) -> fmt::Result + Clone,
{
    FieldFn(f)
}

/// A [`FormatFields`] implementation that formats fields by calling a function
/// or closure.
///
/// [`FormatFields`]: trait.FormatFields.html
#[derive(Debug, Clone)]
pub struct FieldFn<F>(F);
/// The [visitor] produced by [`FieldFn`]'s [`MakeVisitor`] implementation.
///
/// [visitor]: ../../field/trait.Visit.html
/// [`FieldFn`]: struct.FieldFn.html
/// [`MakeVisitor`]: ../../field/trait.MakeVisitor.html
pub struct FieldFnVisitor<'a, F> {
    f: F,
    writer: &'a mut dyn fmt::Write,
    result: fmt::Result,
}
/// Marker for `Format` that indicates that the compact log format should be used.
///
/// The compact format only includes the fields from the most recently entered span.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct Compact;

/// Marker for `Format` that indicates that the verbose log format should be used.
///
/// The full format includes fields from all entered spans.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct Full;

/// A pre-configured event formatter.
///
/// You will usually want to use this as the `FormatEvent` for a `FmtSubscriber`.
///
/// The default logging format, [`Full`] includes all fields in each event and its containing
/// spans. The [`Compact`] logging format includes only the fields from the most-recently-entered
/// span.
#[derive(Debug, Clone)]
pub struct Format<F = Full, T = SystemTime> {
    format: PhantomData<F>,
    timer: T,
    ansi: bool,
    display_target: bool,
}

impl Default for Format<Full, SystemTime> {
    fn default() -> Self {
        Format {
            format: PhantomData,
            timer: SystemTime,
            ansi: true,
            display_target: true,
        }
    }
}

impl<F, T> Format<F, T> {
    /// Use a less verbose output format.
    ///
    /// See [`Compact`].
    pub fn compact(self) -> Format<Compact, T> {
        Format {
            format: PhantomData,
            timer: self.timer,
            ansi: self.ansi,
            display_target: self.display_target,
        }
    }

    /// Use the full JSON format.
    ///
    /// See [`Json`].
    #[cfg(feature = "json")]
    pub fn json(self) -> Format<Json, T> {
        Format {
            format: PhantomData,
            timer: self.timer,
            ansi: self.ansi,
            display_target: self.display_target,
        }
    }

    /// Use the given [`timer`] for log message timestamps.
    ///
    /// See [`time`] for the provided timer implementations.
    ///
    /// Note that using the `chrono` feature flag enables the
    /// additional time formatters [`ChronoUtc`] and [`ChronoLocal`].
    ///
    /// [`time`]: ./time/index.html
    /// [`timer`]: ./time/trait.FormatTime.html
    /// [`ChronoUtc`]: ./time/struct.ChronoUtc.html
    /// [`ChronoLocal`]: ./time/struct.ChronoLocal.html
    pub fn with_timer<T2>(self, timer: T2) -> Format<F, T2> {
        Format {
            format: self.format,
            timer,
            ansi: self.ansi,
            display_target: self.display_target,
        }
    }

    /// Do not emit timestamps with log messages.
    pub fn without_time(self) -> Format<F, ()> {
        Format {
            format: self.format,
            timer: (),
            ansi: self.ansi,
            display_target: self.display_target,
        }
    }

    /// Enable ANSI terminal colors for formatted output.
    pub fn with_ansi(self, ansi: bool) -> Format<F, T> {
        Format { ansi, ..self }
    }

    /// Sets whether or not an event's target is displayed.
    pub fn with_target(self, display_target: bool) -> Format<F, T> {
        Format {
            display_target,
            ..self
        }
    }
}

impl<N, T> FormatEvent<N> for Format<Full, T>
where
    N: for<'writer> FormatFields<'writer>,
    T: FormatTime,
{
    fn format_event(
        &self,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        event: &Event<'_>,
    ) -> fmt::Result {
        #[cfg(feature = "tracing-log")]
        let normalized_meta = event.normalized_metadata();
        #[cfg(feature = "tracing-log")]
        let meta = normalized_meta.as_ref().unwrap_or_else(|| event.metadata());
        #[cfg(not(feature = "tracing-log"))]
        let meta = event.metadata();
        #[cfg(feature = "ansi")]
        time::write(&self.timer, writer, self.ansi)?;
        #[cfg(not(feature = "ansi"))]
        time::write(&self.timer, writer)?;

        let (fmt_level, full_ctx) = {
            #[cfg(feature = "ansi")]
            {
                (
                    FmtLevel::new(meta.level(), self.ansi),
                    FullCtx::new(&ctx, self.ansi),
                )
            }
            #[cfg(not(feature = "ansi"))]
            {
                (FmtLevel::new(meta.level()), FullCtx::new(&ctx))
            }
        };

        write!(
            writer,
            "{} {}{}: ",
            fmt_level,
            full_ctx,
            if self.display_target {
                meta.target()
            } else {
                ""
            }
        )?;
        ctx.format_fields(writer, event)?;
        writeln!(writer)
    }
}

impl<N, T> FormatEvent<N> for Format<Compact, T>
where
    N: for<'writer> FormatFields<'writer>,
    T: FormatTime,
{
    fn format_event(
        &self,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        event: &Event<'_>,
    ) -> fmt::Result {
        #[cfg(feature = "tracing-log")]
        let normalized_meta = event.normalized_metadata();
        #[cfg(feature = "tracing-log")]
        let meta = normalized_meta.as_ref().unwrap_or_else(|| event.metadata());
        #[cfg(not(feature = "tracing-log"))]
        let meta = event.metadata();
        #[cfg(feature = "ansi")]
        time::write(&self.timer, writer, self.ansi)?;
        #[cfg(not(feature = "ansi"))]
        time::write(&self.timer, writer)?;

        let (fmt_level, fmt_ctx) = {
            #[cfg(feature = "ansi")]
            {
                (
                    FmtLevel::new(meta.level(), self.ansi),
                    FmtCtx::new(&ctx, self.ansi),
                )
            }
            #[cfg(not(feature = "ansi"))]
            {
                (FmtLevel::new(meta.level()), FmtCtx::new(&ctx))
            }
        };
        write!(
            writer,
            "{} {}{}: ",
            fmt_level,
            fmt_ctx,
            if self.display_target {
                meta.target()
            } else {
                ""
            }
        )?;
        ctx.format_fields(writer, event)?;
        ctx.with_current(|(_, span)| write!(writer, " {}", span.fields()))
            .unwrap_or(Ok(()))?;
        writeln!(writer)
    }
}

// === impl FormatFields ===
impl<'writer, M> FormatFields<'writer> for M
where
    M: MakeOutput<&'writer mut dyn fmt::Write, fmt::Result>,
    M::Visitor: VisitFmt + VisitOutput<fmt::Result>,
{
    fn format_fields<R: RecordFields>(
        &self,
        writer: &'writer mut dyn fmt::Write,
        fields: R,
    ) -> fmt::Result {
        let mut v = self.make_visitor(writer);
        fields.record(&mut v);
        v.finish()
    }
}
/// The default [`FormatFields`] implementation.
///
/// [`FormatFields`]: trait.FormatFields.html
#[derive(Debug)]
pub struct DefaultFields {
    // reserve the ability to add fields to this without causing a breaking
    // change in the future.
    _private: (),
}

/// The [visitor] produced by [`DefaultFields`]'s [`MakeVisitor`] implementation.
///
/// [visitor]: ../../field/trait.Visit.html
/// [`DefaultFields`]: struct.DefaultFields.html
/// [`MakeVisitor`]: ../../field/trait.MakeVisitor.html
pub struct DefaultVisitor<'a> {
    writer: &'a mut dyn Write,
    is_empty: bool,
    result: fmt::Result,
}

impl DefaultFields {
    /// Returns a new default [`FormatFields`] implementation.
    ///
    /// [`FormatFields`]: trait.FormatFields.html
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for DefaultFields {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> MakeVisitor<&'a mut dyn Write> for DefaultFields {
    type Visitor = DefaultVisitor<'a>;

    #[inline]
    fn make_visitor(&self, target: &'a mut dyn Write) -> Self::Visitor {
        DefaultVisitor::new(target, true)
    }
}

// === impl DefaultVisitor ===

impl<'a> DefaultVisitor<'a> {
    /// Returns a new default visitor that formats to the provided `writer`.
    ///
    /// # Arguments
    /// - `writer`: the writer to format to.
    /// - `is_empty`: whether or not any fields have been previously written to
    ///   that writer.
    pub fn new(writer: &'a mut dyn Write, is_empty: bool) -> Self {
        Self {
            writer,
            is_empty,
            result: Ok(()),
        }
    }

    fn maybe_pad(&mut self) {
        if self.is_empty {
            self.is_empty = false;
        } else {
            self.result = write!(self.writer, " ");
        }
    }
}

impl<'a> field::Visit for DefaultVisitor<'a> {
    fn record_str(&mut self, field: &Field, value: &str) {
        if self.result.is_err() {
            return;
        }

        if field.name() == "message" {
            self.record_debug(field, &format_args!("{}", value))
        } else {
            self.record_debug(field, &value)
        }
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        if let Some(source) = value.source() {
            self.record_debug(
                field,
                &format_args!("{} {}.source={}", value, field, source),
            )
        } else {
            self.record_debug(field, &format_args!("{}", value))
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if self.result.is_err() {
            return;
        }

        self.maybe_pad();
        self.result = match field.name() {
            "message" => write!(self.writer, "{:?}", value),
            // Skip fields that are actually log metadata that have already been handled
            #[cfg(feature = "tracing-log")]
            name if name.starts_with("log.") => Ok(()),
            name if name.starts_with("r#") => write!(self.writer, "{}={:?}", &name[2..], value),
            name => write!(self.writer, "{}={:?}", name, value),
        };
    }
}

impl<'a> crate::field::VisitOutput<fmt::Result> for DefaultVisitor<'a> {
    fn finish(self) -> fmt::Result {
        self.result
    }
}

impl<'a> crate::field::VisitFmt for DefaultVisitor<'a> {
    fn writer(&mut self) -> &mut dyn fmt::Write {
        self.writer
    }
}

impl<'a> fmt::Debug for DefaultVisitor<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefaultVisitor")
            .field("writer", &format_args!("<dyn fmt::Write>"))
            .field("is_empty", &self.is_empty)
            .field("result", &self.result)
            .finish()
    }
}

struct FmtCtx<'a, N> {
    ctx: &'a span::Context<'a, N>,
    #[cfg(feature = "ansi")]
    ansi: bool,
}

impl<'a, N: 'a> FmtCtx<'a, N> {
    #[cfg(feature = "ansi")]
    pub(crate) fn new(ctx: &'a span::Context<'a, N>, ansi: bool) -> Self {
        Self { ctx, ansi }
    }

    #[cfg(not(feature = "ansi"))]
    pub(crate) fn new(ctx: &'a span::Context<'a, N>) -> Self {
        Self { ctx }
    }
}

#[cfg(feature = "ansi")]
impl<'a, N> fmt::Display for FmtCtx<'a, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut seen = false;
        self.ctx.visit_spans(|_, span| {
            if seen {
                f.pad(":")?;
            }
            seen = true;

            if self.ansi {
                write!(f, "{}", Style::new().bold().paint(span.name()))
            } else {
                write!(f, "{}", span.name())
            }
        })?;
        if seen {
            f.pad(" ")?;
        }
        Ok(())
    }
}

#[cfg(not(feature = "ansi"))]
impl<'a, N> fmt::Display for FmtCtx<'a, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut seen = false;
        self.ctx.visit_spans(|_, span| {
            if seen {
                f.pad(":")?;
            }
            seen = true;
            write!(f, "{}", span.name())
        })?;
        if seen {
            f.pad(" ")?;
        }
        Ok(())
    }
}

struct FullCtx<'a, N> {
    ctx: &'a span::Context<'a, N>,
    #[cfg(feature = "ansi")]
    ansi: bool,
}

impl<'a, N: 'a> FullCtx<'a, N> {
    #[cfg(feature = "ansi")]
    pub(crate) fn new(ctx: &'a span::Context<'a, N>, ansi: bool) -> Self {
        Self { ctx, ansi }
    }

    #[cfg(not(feature = "ansi"))]
    pub(crate) fn new(ctx: &'a span::Context<'a, N>) -> Self {
        Self { ctx }
    }
}

#[cfg(feature = "ansi")]
impl<'a, N> fmt::Display for FullCtx<'a, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut seen = false;
        let style = if self.ansi {
            Style::new().bold()
        } else {
            Style::new()
        };
        self.ctx.visit_spans(|_, span| {
            write!(f, "{}", style.paint(span.name()))?;

            seen = true;

            let fields = span.fields();
            if !fields.is_empty() {
                write!(f, "{}{}{}", style.paint("{"), fields, style.paint("}"))?;
            }
            ":".fmt(f)
        })?;
        if seen {
            f.pad(" ")?;
        }
        Ok(())
    }
}

#[cfg(not(feature = "ansi"))]
impl<'a, N> fmt::Display for FullCtx<'a, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut seen = false;
        self.ctx.visit_spans(|_, span| {
            write!(f, "{}", span.name())?;
            seen = true;

            let fields = span.fields();
            if !fields.is_empty() {
                write!(f, "{{{}}}", fields)?;
            }
            ":".fmt(f)
        })?;
        if seen {
            f.pad(" ")?;
        }
        Ok(())
    }
}

struct FmtLevel<'a> {
    level: &'a Level,
    #[cfg(feature = "ansi")]
    ansi: bool,
}

impl<'a> FmtLevel<'a> {
    #[cfg(feature = "ansi")]
    pub(crate) fn new(level: &'a Level, ansi: bool) -> Self {
        Self { level, ansi }
    }

    #[cfg(not(feature = "ansi"))]
    pub(crate) fn new(level: &'a Level) -> Self {
        Self { level }
    }
}

#[cfg(not(feature = "ansi"))]
impl<'a> fmt::Display for FmtLevel<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self.level {
            Level::TRACE => f.pad("TRACE"),
            Level::DEBUG => f.pad("DEBUG"),
            Level::INFO => f.pad("INFO"),
            Level::WARN => f.pad("WARN"),
            Level::ERROR => f.pad("ERROR"),
        }
    }
}

#[cfg(feature = "ansi")]
impl<'a> fmt::Display for FmtLevel<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ansi {
            match *self.level {
                Level::TRACE => write!(f, "{}", Colour::Purple.paint("TRACE")),
                Level::DEBUG => write!(f, "{}", Colour::Blue.paint("DEBUG")),
                Level::INFO => write!(f, "{}", Colour::Green.paint(" INFO")),
                Level::WARN => write!(f, "{}", Colour::Yellow.paint(" WARN")),
                Level::ERROR => write!(f, "{}", Colour::Red.paint("ERROR")),
            }
        } else {
            match *self.level {
                Level::TRACE => f.pad("TRACE"),
                Level::DEBUG => f.pad("DEBUG"),
                Level::INFO => f.pad("INFO"),
                Level::WARN => f.pad("WARN"),
                Level::ERROR => f.pad("ERROR"),
            }
        }
    }
}

// === impl FieldFn ===

impl<'a, F> MakeVisitor<&'a mut dyn fmt::Write> for FieldFn<F>
where
    F: Fn(&mut dyn fmt::Write, &Field, &dyn fmt::Debug) -> fmt::Result + Clone,
{
    type Visitor = FieldFnVisitor<'a, F>;

    fn make_visitor(&self, writer: &'a mut dyn fmt::Write) -> Self::Visitor {
        FieldFnVisitor {
            writer,
            f: self.0.clone(),
            result: Ok(()),
        }
    }
}

impl<'a, F> Visit for FieldFnVisitor<'a, F>
where
    F: Fn(&mut dyn fmt::Write, &Field, &dyn fmt::Debug) -> fmt::Result,
{
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if self.result.is_ok() {
            self.result = (self.f)(&mut self.writer, field, value)
        }
    }
}

impl<'a, F> VisitOutput<fmt::Result> for FieldFnVisitor<'a, F>
where
    F: Fn(&mut dyn fmt::Write, &Field, &dyn fmt::Debug) -> fmt::Result,
{
    fn finish(self) -> fmt::Result {
        self.result
    }
}

impl<'a, F> VisitFmt for FieldFnVisitor<'a, F>
where
    F: Fn(&mut dyn fmt::Write, &Field, &dyn fmt::Debug) -> fmt::Result,
{
    fn writer(&mut self) -> &mut dyn fmt::Write {
        &mut *self.writer
    }
}

impl<'a, F> fmt::Debug for FieldFnVisitor<'a, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FieldFnVisitor")
            .field("f", &format_args!("<Fn>"))
            .field("writer", &format_args!("<dyn fmt::Write>"))
            .field("result", &self.result)
            .finish()
    }
}

#[cfg(test)]
mod test {

    use crate::fmt::test::MockWriter;
    use crate::fmt::time::FormatTime;
    use lazy_static::lazy_static;
    use tracing::{self, subscriber::with_default};

    use std::fmt;
    use std::sync::Mutex;

    struct MockTime;
    impl FormatTime for MockTime {
        fn format_time(&self, w: &mut dyn fmt::Write) -> fmt::Result {
            write!(w, "fake time")
        }
    }

    #[cfg(feature = "ansi")]
    #[test]
    fn with_ansi_true() {
        lazy_static! {
            static ref BUF: Mutex<Vec<u8>> = Mutex::new(vec![]);
        }

        let make_writer = || MockWriter::new(&BUF);
        let expected = "\u{1b}[2mfake time\u{1b}[0m\u{1b}[32m INFO\u{1b}[0m tracing_subscriber::fmt::format::test: some ansi test\n";
        test_ansi(make_writer, expected, true, &BUF);
    }

    #[cfg(feature = "ansi")]
    #[test]
    fn with_ansi_false() {
        lazy_static! {
            static ref BUF: Mutex<Vec<u8>> = Mutex::new(vec![]);
        }

        let make_writer = || MockWriter::new(&BUF);
        let expected = "fake time INFO tracing_subscriber::fmt::format::test: some ansi test\n";

        test_ansi(make_writer, expected, false, &BUF);
    }

    #[cfg(not(feature = "ansi"))]
    #[test]
    fn without_ansi() {
        lazy_static! {
            static ref BUF: Mutex<Vec<u8>> = Mutex::new(vec![]);
        }

        let make_writer = || MockWriter::new(&BUF);
        let expected = "fake time INFO tracing_subscriber::fmt::format::test: some ansi test\n";
        let subscriber = crate::fmt::Subscriber::builder()
            .with_writer(make_writer)
            .with_timer(MockTime)
            .finish();

        with_default(subscriber, || {
            tracing::info!("some ansi test");
        });

        let actual = String::from_utf8(BUF.try_lock().unwrap().to_vec()).unwrap();
        assert_eq!(expected, actual.as_str());
    }

    #[cfg(feature = "ansi")]
    fn test_ansi<T>(make_writer: T, expected: &str, is_ansi: bool, buf: &Mutex<Vec<u8>>)
    where
        T: crate::fmt::MakeWriter + Send + Sync + 'static,
    {
        let subscriber = crate::fmt::Subscriber::builder()
            .with_writer(make_writer)
            .with_ansi(is_ansi)
            .with_timer(MockTime)
            .finish();

        with_default(subscriber, || {
            tracing::info!("some ansi test");
        });

        let actual = String::from_utf8(buf.try_lock().unwrap().to_vec()).unwrap();
        assert_eq!(expected, actual.as_str());
    }
}
