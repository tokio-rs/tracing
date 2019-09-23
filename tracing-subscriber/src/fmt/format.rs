//! Formatters for logging `tracing` events.
use super::span;
use super::time::{self, FormatTime, SystemTime};
use super::NewVisitor;
#[cfg(feature = "tracing-log")]
use tracing_log::NormalizeEvent;

use std::fmt::{self, Write};
use std::marker::PhantomData;
use tracing_core::{
    field::{self, Field},
    Event, Level, Metadata,
};

#[cfg(feature = "ansi")]
use ansi_term::{Colour, Style};

/// A type that can format a tracing `Event` for a `fmt::Write`.
///
/// `FormatEvent` is primarily used in the context of [`FmtSubscriber`]. Each time an event is
/// dispatched to [`FmtSubscriber`], the subscriber forwards it to its associated `FormatEvent` to
/// emit a log message.
///
/// This trait is already implemented for function pointers with the same signature as `format`.
pub trait FormatEvent<N> {
    /// Write a log message for `Event` in `Context` to the given `Write`.
    fn format_event(
        &self,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        event: &Event<'_>,
    ) -> fmt::Result;

    /// Write a log message for [creating a new span][new_span] with the given
    /// `Attributes` in the given `Context` to the provided `Write`.
    ///
    /// This method is optional; the default impl is a no-op. Implementations
    /// which wish to log calls to `new_span` can override this.
    ///
    /// [new_span]: https://docs.rs/tracing/0.1.9/tracing/subscriber/trait.Subscriber.html#tymethod.new_span
    fn format_new_span(
        &self,
        _ctx: &span::Context<'_, N>,
        _writer: &mut dyn fmt::Write,
        _attrs: &span::Attributes<'_>,
    ) -> fmt::Result {
        Ok(())
    }

    /// Write a log message for [closing a span][close] with the given
    /// `Id` in the given `Context` to the provided `Write`.
    ///
    /// This method is optional; the default impl is a no-op. Implementations
    /// which wish to log calls to `new_span` can override this.
    ///
    /// [close]: https://docs.rs/tracing/0.1.9/tracing/subscriber/trait.Subscriber.html#method.try_close
    fn format_close(
        &self,
        _ctx: &span::Context<'_, N>,
        _writer: &mut dyn fmt::Write,
        _id: &span::Id,
    ) -> fmt::Result {
        Ok(())
    }

    /// Write a log message for [entering a span][enter] with the given
    /// `Id` in the given `Context` to the provided `Write`.
    ///
    /// This method is optional; the default impl is a no-op. Implementations
    /// which wish to log calls to `new_span` can override this.
    ///
    /// [enter]: https://docs.rs/tracing/0.1.9/tracing/subscriber/trait.Subscriber.html#tymethod.enter
    fn format_enter(
        &self,
        _ctx: &span::Context<'_, N>,
        _writer: &mut dyn fmt::Write,
        _attrs: &span::Id,
    ) -> fmt::Result {
        Ok(())
    }

    /// Write a log message for [exiting a span][close] with the given
    /// `Id` in the given `Context` to the provided `Write`.
    ///
    /// This method is optional; the default impl is a no-op. Implementations
    /// which wish to log calls to `new_span` can override this.
    ///
    /// [close]: https://docs.rs/tracing/0.1.9/tracing/subscriber/trait.Subscriber.html#tymethod.exit
    fn format_exit(
        &self,
        _ctx: &span::Context<'_, N>,
        _writer: &mut dyn fmt::Write,
        _id: &span::Id,
    ) -> fmt::Result {
        Ok(())
    }
}

/// A type that can format the context for a log line.
///
/// This includes the current span context, a timestamp, and the metadata of the
/// span or event being logged.
pub trait FormatCtx {
    /// Formats the context portion of a log line for a span or event with the
    /// provided metadata.
    fn format_ctx<N, T>(
        &self,
        ctx: &span::Context<'_, N>,
        timer: &T,
        writer: &mut dyn fmt::Write,
        metadata: &Metadata<'_>,
    ) -> fmt::Result
    where
        N: for<'a> NewVisitor<'a>,
        T: FormatTime;

    /// Sets whether ANSI formatting should be enabled, returning a new
    /// `FormatCtx`.
    #[cfg(feature = "ansi")]
    fn with_ansi(self, with_ansi: bool) -> Self;

    /// Returns true if ANSI formatting is enabled.
    #[cfg(feature = "ansi")]
    fn has_ansi(&self) -> bool;

    /// Sets whether the target should be included in the formatted output,
    /// returning a new `FormatCtx`.
    fn with_target(self, with_target: bool) -> Self;

    /// Returns true if targets are included in the formatted output.
    fn has_target(&self) -> bool;
}

/// A type that can format span lifecycle events to a `tracing::Write`.
pub trait SpanLifecycle {
    /// Write a log message for [creating a new span][new_span] with the given
    /// `Attributes` in the given `Context` to the provided `Write`.
    ///
    /// This method is optional; the default impl is a no-op. Implementations
    /// which wish to log calls to `new_span` can override this.
    ///
    /// [new_span]: https://docs.rs/tracing/0.1.9/tracing/subscriber/trait.Subscriber.html#tymethod.new_span
    fn format_new_span<N, F, T>(
        formatter: &F,
        timer: &T,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        attrs: &span::Attributes<'_>,
    ) -> fmt::Result
    where
        N: for<'a> NewVisitor<'a>,
        F: FormatCtx,
        T: FormatTime;

    /// Write a log message for [closing a span][close] with the given
    /// `Id` in the given `Context` to the provided `Write`.
    ///
    /// This method is optional; the default impl is a no-op. Implementations
    /// which wish to log calls to `new_span` can override this.
    ///
    /// [close]: https://docs.rs/tracing/0.1.9/tracing/subscriber/trait.Subscriber.html#method.try_close
    fn format_close<N, F, T>(
        formatter: &F,
        timer: &T,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        id: &span::Id,
    ) -> fmt::Result
    where
        N: for<'a> NewVisitor<'a>,
        F: FormatCtx,
        T: FormatTime;
}

/// A type that formats log lines for enter/exit events.
pub trait SpanEntry {
    /// Write a log message for [entering a span][enter] with the given
    /// `Id` in the given `Context` to the provided `Write`.
    ///
    /// This method is optional; the default impl is a no-op. Implementations
    /// which wish to log calls to `new_span` can override this.
    ///
    /// [enter]: https://docs.rs/tracing/0.1.9/tracing/subscriber/trait.Subscriber.html#tymethod.enter
    fn format_enter<N, F, T>(
        formatter: &F,
        timer: &T,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        attrs: &span::Id,
    ) -> fmt::Result
    where
        N: for<'a> NewVisitor<'a>,
        F: FormatCtx,
        T: FormatTime;

    /// Write a log message for [exiting a span][close] with the given
    /// `Id` in the given `Context` to the provided `Write`.
    ///
    /// This method is optional; the default impl is a no-op. Implementations
    /// which wish to log calls to `new_span` can override this.
    ///
    /// [close]: https://docs.rs/tracing/0.1.9/tracing/subscriber/trait.Subscriber.html#tymethod.exit
    fn format_exit<N, F, T>(
        formatter: &F,
        timer: &T,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        id: &span::Id,
    ) -> fmt::Result
    where
        N: for<'a> NewVisitor<'a>,
        F: FormatCtx,
        T: FormatTime;
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

/// Marker for `Format` that indicates that the compact log format should be used.
///
/// The compact format only includes the fields from the most recently entered span.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct Compact {
    ansi: bool,
    display_target: bool,
}

/// Marker for `Format` that indicates that the verbose log format should be used.
///
/// The full format includes fields from all entered spans.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct Full {
    ansi: bool,
    display_target: bool,
}

/// Marker for `Format` that indicates that span enters and exits or span
/// opens and closes should be logged.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct WithSpans;

/// A pre-configured event formatter.
///
/// You will usually want to use this as the `FormatEvent` for a `FmtSubscriber`.
///
/// The default logging format, [`Full`] includes all fields in each event and its containing
/// spans. The [`Compact`] logging format includes only the fields from the most-recently-entered
/// span.
#[derive(Debug, Clone)]
pub struct Format<F = Full, T = SystemTime, L = (), E = ()> {
    format: PhantomData<(L, E)>,
    fmt_ctx: F,
    timer: T,
    ansi: bool,
}

impl Default for Format<Full, SystemTime> {
    fn default() -> Self {
        Format {
            format: PhantomData,
            fmt_ctx: Full {
                ansi: true,
                display_target: true,
            },
            timer: SystemTime,
            ansi: true,
        }
    }
}

impl<F, T, L, E> Format<F, T, L, E>
where
    F: FormatCtx,
{
    /// Use a less verbose output format.
    ///
    /// See [`Compact`].
    pub fn compact(self) -> Format<Compact, T, L, E> {
        Format {
            format: PhantomData,
            fmt_ctx: Compact {
                ansi: self.ansi,
                display_target: self.fmt_ctx.has_target(),
            },
            timer: self.timer,
            ansi: self.ansi,
        }
    }

    /// Use the given `timer` for log message timestamps.
    pub fn with_timer<T2>(self, timer: T2) -> Format<F, T2, L, E> {
        Format {
            format: self.format,
            fmt_ctx: self.fmt_ctx,
            timer,
            ansi: self.ansi,
        }
    }

    /// Configures the formatter to log when spans are created and closed.
    pub fn with_spans(self) -> Format<F, T, WithSpans, E> {
        Format {
            format: PhantomData,
            fmt_ctx: self.fmt_ctx,
            timer: self.timer,
            ansi: self.ansi,
        }
    }

    /// Configures the formatter to log when spans are entered and exited.
    pub fn with_entry(self) -> Format<F, T, L, WithSpans> {
        Format {
            format: PhantomData,
            fmt_ctx: self.fmt_ctx,
            timer: self.timer,
            ansi: self.ansi,
        }
    }

    /// Do not emit timestamps with log messages.
    pub fn without_time(self) -> Format<F, (), L, E> {
        Format {
            format: self.format,
            fmt_ctx: self.fmt_ctx,
            timer: (),
            ansi: self.ansi,
        }
    }

    /// Enable ANSI terminal colors for formatted output.
    pub fn with_ansi(self, ansi: bool) -> Format<F, T, L, E> {
        Format {
            ansi,
            fmt_ctx: self.fmt_ctx.with_ansi(ansi),
            ..self
        }
    }

    /// Sets whether or not an event's target is displayed.
    pub fn with_target(self, display_target: bool) -> Format<F, T, L, E> {
        Format {
            fmt_ctx: self.fmt_ctx.with_target(display_target),
            ..self
        }
    }
}

impl<N, T, L, E> FormatEvent<N> for Format<Full, T, L, E>
where
    N: for<'a> super::NewVisitor<'a>,
    T: FormatTime,
    L: SpanLifecycle,
    E: SpanEntry,
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
        self.fmt_ctx.format_ctx(ctx, &self.timer, writer, meta)?;
        {
            let mut recorder = ctx.new_visitor(writer, true);
            event.record(&mut recorder);
        }
        writeln!(writer)
    }

    #[inline]
    fn format_new_span(
        &self,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        attrs: &span::Attributes<'_>,
    ) -> fmt::Result {
        L::format_new_span(&self.fmt_ctx, &self.timer, ctx, writer, attrs)
    }

    #[inline]
    fn format_close(
        &self,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        id: &span::Id,
    ) -> fmt::Result {
        L::format_close(&self.fmt_ctx, &self.timer, ctx, writer, id)
    }

    #[inline]
    fn format_enter(
        &self,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        id: &span::Id,
    ) -> fmt::Result {
        E::format_enter(&self.fmt_ctx, &self.timer, ctx, writer, id)
    }

    #[inline]
    fn format_exit(
        &self,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        id: &span::Id,
    ) -> fmt::Result {
        E::format_exit(&self.fmt_ctx, &self.timer, ctx, writer, id)
    }
}

impl<N, T, L, E> FormatEvent<N> for Format<Compact, T, L, E>
where
    N: for<'a> super::NewVisitor<'a>,
    T: FormatTime,
    L: SpanLifecycle,
    E: SpanEntry,
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
        self.fmt_ctx.format_ctx(ctx, &self.timer, writer, meta)?;
        {
            let mut recorder = ctx.new_visitor(writer, true);
            event.record(&mut recorder);
        }
        ctx.with_current(|(_, span)| write!(writer, " {}", span.fields()))
            .unwrap_or(Ok(()))?;
        writeln!(writer)
    }

    #[inline]
    fn format_new_span(
        &self,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        attrs: &span::Attributes<'_>,
    ) -> fmt::Result {
        L::format_new_span(&self.fmt_ctx, &self.timer, ctx, writer, attrs)
    }

    #[inline]
    fn format_close(
        &self,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        id: &span::Id,
    ) -> fmt::Result {
        L::format_close(&self.fmt_ctx, &self.timer, ctx, writer, id)
    }

    #[inline]
    fn format_enter(
        &self,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        id: &span::Id,
    ) -> fmt::Result {
        E::format_enter(&self.fmt_ctx, &self.timer, ctx, writer, id)
    }

    #[inline]
    fn format_exit(
        &self,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        id: &span::Id,
    ) -> fmt::Result {
        E::format_exit(&self.fmt_ctx, &self.timer, ctx, writer, id)
    }
}

impl SpanLifecycle for () {
    fn format_new_span<N, F, T>(
        _formatter: &F,
        _timer: &T,
        _ctx: &span::Context<'_, N>,
        _writer: &mut dyn fmt::Write,
        _attrs: &span::Attributes<'_>,
    ) -> fmt::Result
    where
        N: for<'a> NewVisitor<'a>,
        F: FormatCtx,
        T: FormatTime,
    {
        Ok(())
    }

    fn format_close<N, F, T>(
        _formatter: &F,
        _timer: &T,
        _ctx: &span::Context<'_, N>,
        _writer: &mut dyn fmt::Write,
        _id: &span::Id,
    ) -> fmt::Result
    where
        N: for<'a> NewVisitor<'a>,
        F: FormatCtx,
        T: FormatTime,
    {
        Ok(())
    }
}

impl SpanEntry for () {
    fn format_enter<N, F, T>(
        _formatter: &F,
        _timer: &T,
        _ctx: &span::Context<'_, N>,
        _writer: &mut dyn fmt::Write,
        _id: &span::Id,
    ) -> fmt::Result {
        Ok(())
    }

    fn format_exit<N, F, T>(
        _formatter: &F,
        _timer: &T,
        _ctx: &span::Context<'_, N>,
        _writer: &mut dyn fmt::Write,
        _id: &span::Id,
    ) -> fmt::Result
    where
        N: for<'a> NewVisitor<'a>,
        F: FormatCtx,
        T: FormatTime,
    {
        Ok(())
    }
}

#[cfg(not(feature = "ansi"))]
impl SpanLifecycle for WithSpans {
    fn format_new_span<N, F, T>(
        formatter: &F,
        timer: &T,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        attrs: &span::Attributes<'_>,
    ) -> fmt::Result
    where
        N: for<'a> NewVisitor<'a>,
        F: FormatCtx,
        T: FormatTime,
    {
        let meta = attrs.metadata();
        formatter.format_ctx(ctx, timer, writer, meta)?;
        write!(writer, "{}", meta.name())?;
        if !attrs.is_empty() {
            writer.write_char('{')?;
            attrs.record(&mut ctx.new_visitor(writer, true));
            writer.write_char('}')?;
        }
        writeln!(writer)
    }

    fn format_close<N, F, T>(
        formatter: &F,
        timer: &T,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        id: &span::Id,
    ) -> fmt::Result
    where
        N: for<'a> NewVisitor<'a>,
        F: FormatCtx,
        T: FormatTime,
    {
        if let Some(span) = ctx.span(id) {
            formatter.format_ctx(ctx, timer, writer, span.metadata())?;
            write!(writer, "close {}", span.name())?;
            let fields = span.fields();
            if !fields.is_empty() {
                write!(writer, "{{{}}}", fields)?;
            }
            writeln!(writer)?;
        }
        Ok(())
    }
}

#[cfg(not(feature = "ansi"))]
impl SpanEntry for WithSpans {
    fn format_enter<N, F, T>(
        formatter: &F,
        timer: &T,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        id: &span::Id,
    ) -> fmt::Result
    where
        N: for<'a> NewVisitor<'a>,
        F: FormatCtx,
        T: FormatTime,
    {
        if let Some(span) = ctx.span(id) {
            formatter.format_ctx(ctx, timer, writer, span.metadata())?;
            write!(writer, "enter {}", span.name())?;
            let fields = span.fields();
            if !fields.is_empty() {
                write!(writer, "{{{}}}", fields)?;
            }
            writeln!(writer)?;
        }
        Ok(())
    }

    fn format_exit<N, F, T>(
        formatter: &F,
        timer: &T,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        id: &span::Id,
    ) -> fmt::Result
    where
        N: for<'a> NewVisitor<'a>,
        F: FormatCtx,
        T: FormatTime,
    {
        if let Some(span) = ctx.span(id) {
            formatter.format_ctx(ctx, timer, writer, span.metadata())?;
            write!(writer, "exit {}", span.name())?;
            let fields = span.fields();
            if !fields.is_empty() {
                write!(writer, "{{{}}}", fields)?;
            }
            writeln!(writer)?;
        }
        Ok(())
    }
}

#[cfg(feature = "ansi")]
impl SpanEntry for WithSpans {
    fn format_enter<N, F, T>(
        formatter: &F,
        timer: &T,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        id: &span::Id,
    ) -> fmt::Result
    where
        N: for<'a> NewVisitor<'a>,
        F: FormatCtx,
        T: FormatTime,
    {
        if let Some(span) = ctx.span(id) {
            formatter.format_ctx(ctx, timer, writer, span.metadata())?;
            let style = if formatter.has_ansi() {
                Style::new().bold()
            } else {
                Style::new()
            };
            write!(writer, "enter {}", style.paint(span.name()))?;
            let fields = span.fields();
            if !fields.is_empty() {
                write!(writer, "{}{}{}", style.paint("{"), fields, style.paint("}"))?;
            }
            writeln!(writer)?;
        }
        Ok(())
    }

    fn format_exit<N, F, T>(
        formatter: &F,
        timer: &T,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        id: &span::Id,
    ) -> fmt::Result
    where
        N: for<'a> NewVisitor<'a>,
        F: FormatCtx,
        T: FormatTime,
    {
        if let Some(span) = ctx.span(id) {
            formatter.format_ctx(ctx, timer, writer, span.metadata())?;
            let style = if formatter.has_ansi() {
                Style::new().bold()
            } else {
                Style::new()
            };
            write!(writer, "exit {}", style.paint(span.name()))?;
            let fields = span.fields();
            if !fields.is_empty() {
                write!(writer, "{}{}{}", style.paint("{"), fields, style.paint("}"))?;
            }
            writeln!(writer)?;
        }
        Ok(())
    }
}

#[cfg(feature = "ansi")]
impl SpanLifecycle for WithSpans {
    fn format_new_span<N, F, T>(
        formatter: &F,
        timer: &T,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        attrs: &span::Attributes<'_>,
    ) -> fmt::Result
    where
        N: for<'a> NewVisitor<'a>,
        F: FormatCtx,
        T: FormatTime,
    {
        let meta = attrs.metadata();
        formatter.format_ctx(ctx, timer, writer, meta)?;
        let style = if formatter.has_ansi() {
            Style::new().bold()
        } else {
            Style::new()
        };
        write!(writer, "{}", style.paint(meta.name()))?;
        if !attrs.is_empty() {
            write!(writer, "{}", style.paint("{"))?;
            attrs.record(&mut ctx.new_visitor(writer, true));
            write!(writer, "{}", style.paint("}"))?;
        }
        writeln!(writer)
    }

    fn format_close<N, F, T>(
        formatter: &F,
        timer: &T,
        ctx: &span::Context<'_, N>,
        writer: &mut dyn fmt::Write,
        id: &span::Id,
    ) -> fmt::Result
    where
        N: for<'a> NewVisitor<'a>,
        F: FormatCtx,
        T: FormatTime,
    {
        if let Some(span) = ctx.span(id) {
            formatter.format_ctx(ctx, timer, writer, span.metadata())?;
            let style = if formatter.has_ansi() {
                Style::new().bold()
            } else {
                Style::new()
            };
            write!(writer, "close {}", style.paint(span.name()))?;
            let fields = span.fields();
            if !fields.is_empty() {
                write!(writer, "{}{}{}", style.paint("{"), fields, style.paint("}"))?;
            }
            writeln!(writer)?;
        }
        Ok(())
    }
}

impl FormatCtx for Full {
    fn format_ctx<N, T>(
        &self,
        ctx: &span::Context<'_, N>,
        timer: &T,
        writer: &mut dyn fmt::Write,
        meta: &Metadata<'_>,
    ) -> fmt::Result
    where
        N: for<'a> NewVisitor<'a>,
        T: FormatTime,
    {
        time::write(timer, writer)?;
        write!(
            writer,
            "{} {}{}: ",
            FmtLevel::new(meta.level(), self.ansi),
            FullCtx::new(&ctx, self.ansi),
            if self.display_target {
                meta.target()
            } else {
                ""
            }
        )
    }

    #[cfg(feature = "ansi")]
    fn with_ansi(self, ansi: bool) -> Self {
        Self { ansi, ..self }
    }

    #[cfg(feature = "ansi")]
    fn has_ansi(&self) -> bool {
        self.ansi
    }

    fn with_target(self, display_target: bool) -> Self {
        Self {
            display_target,
            ..self
        }
    }

    fn has_target(&self) -> bool {
        self.display_target
    }
}

impl FormatCtx for Compact {
    fn format_ctx<N, T>(
        &self,
        ctx: &span::Context<'_, N>,
        timer: &T,
        writer: &mut dyn fmt::Write,
        meta: &Metadata<'_>,
    ) -> fmt::Result
    where
        N: for<'a> NewVisitor<'a>,
        T: FormatTime,
    {
        time::write(timer, writer)?;
        write!(
            writer,
            "{} {}{}: ",
            FmtLevel::new(meta.level(), self.ansi),
            FmtCtx::new(&ctx, self.ansi),
            if self.display_target {
                meta.target()
            } else {
                ""
            }
        )
    }

    #[cfg(feature = "ansi")]
    fn with_ansi(self, ansi: bool) -> Self {
        Self { ansi, ..self }
    }

    #[cfg(feature = "ansi")]
    fn has_ansi(&self) -> bool {
        self.ansi
    }

    fn with_target(self, display_target: bool) -> Self {
        Self {
            display_target,
            ..self
        }
    }

    fn has_target(&self) -> bool {
        self.display_target
    }
}

/// The default implementation of `NewVisitor` that records fields using the
/// default format.
#[derive(Debug)]
pub struct NewRecorder {
    _p: (),
}

impl NewRecorder {
    pub(crate) fn new() -> Self {
        Self { _p: () }
    }
}

/// A visitor that records fields using the default format.
pub struct Recorder<'a> {
    writer: &'a mut dyn Write,
    is_empty: bool,
}

impl<'a> Recorder<'a> {
    pub(crate) fn new(writer: &'a mut dyn Write, is_empty: bool) -> Self {
        Self { writer, is_empty }
    }

    fn maybe_pad(&mut self) {
        if self.is_empty {
            self.is_empty = false;
        } else {
            let _ = write!(self.writer, " ");
        }
    }
}

impl<'a> super::NewVisitor<'a> for NewRecorder {
    type Visitor = Recorder<'a>;

    #[inline]
    fn make(&self, writer: &'a mut dyn Write, is_empty: bool) -> Self::Visitor {
        Recorder::new(writer, is_empty)
    }
}

impl<'a> field::Visit for Recorder<'a> {
    fn record_str(&mut self, field: &Field, value: &str) {
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
        self.maybe_pad();
        let _ = match field.name() {
            "message" => write!(self.writer, "{:?}", value),
            // Skip fields that are actually log metadata that have already been handled
            #[cfg(feature = "tracing-log")]
            name if name.starts_with("log.") => Ok(()),
            name if name.starts_with("r#") => write!(self.writer, "{}={:?}", &name[2..], value),
            name => write!(self.writer, "{}={:?}", name, value),
        };
    }
}

// This has to be a manual impl, as `&mut dyn Writer` doesn't implement `Debug`.
impl<'a> fmt::Debug for Recorder<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Recorder")
            .field("writer", &format_args!("<dyn fmt::Write>"))
            .field("is_empty", &self.is_empty)
            .finish()
    }
}

struct FmtCtx<'a, N> {
    ctx: &'a span::Context<'a, N>,
    ansi: bool,
}

impl<'a, N: 'a> FmtCtx<'a, N> {
    pub(crate) fn new(ctx: &'a span::Context<'a, N>, ansi: bool) -> Self {
        Self { ctx, ansi }
    }
}

#[cfg(feature = "ansi")]
impl<'a, N> fmt::Display for FmtCtx<'a, N>
where
    N: super::NewVisitor<'a>,
{
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
    ansi: bool,
}

impl<'a, N: 'a> FullCtx<'a, N> {
    pub(crate) fn new(ctx: &'a span::Context<'a, N>, ansi: bool) -> Self {
        Self { ctx, ansi }
    }
}

#[cfg(feature = "ansi")]
impl<'a, N> fmt::Display for FullCtx<'a, N>
where
    N: super::NewVisitor<'a>,
{
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
    ansi: bool,
}

impl<'a> FmtLevel<'a> {
    pub(crate) fn new(level: &'a Level, ansi: bool) -> Self {
        Self { level, ansi }
    }
}

#[cfg(not(feature = "ansi"))]
impl<'a> fmt::Display for FmtLevel<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
