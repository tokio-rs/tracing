use crate::span;
use crate::time::{self, FormatTime, SystemTime};
use crate::FormatEvent;

use std::fmt::{self, Write};
use std::marker::PhantomData;
use tracing_core::{
    field::{self, Field},
    Event, Level,
};

#[cfg(feature = "ansi")]
use ansi_term::{Colour, Style};

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
}

impl Default for Format<Full, SystemTime> {
    fn default() -> Self {
        Format {
            format: PhantomData,
            timer: SystemTime,
            ansi: true,
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
        }
    }

    /// Use the given `timer` for log message timestamps.
    pub fn with_timer<T2>(self, timer: T2) -> Format<F, T2> {
        Format {
            format: self.format,
            timer,
            ansi: self.ansi,
        }
    }

    /// Do not emit timestamps with log messages.
    pub fn without_time(self) -> Format<F, ()> {
        Format {
            format: self.format,
            timer: (),
            ansi: self.ansi,
        }
    }

    /// Enable ANSI encoding for formatted events.
    pub fn with_ansi(self, ansi: bool) -> Format<F, T> {
        Format { ansi, ..self }
    }
}

impl<N, T> FormatEvent<N> for Format<Full, T>
where
    N: for<'a> crate::NewVisitor<'a>,
    T: FormatTime,
{
    fn format_event(
        &self,
        ctx: &span::Context<N>,
        writer: &mut dyn fmt::Write,
        event: &Event,
    ) -> fmt::Result {
        let meta = event.metadata();
        time::write(&self.timer, writer)?;
        write!(
            writer,
            "{} {}{}: ",
            FmtLevel(meta.level(), self.ansi),
            FullCtx(&ctx, self.ansi),
            meta.target()
        )?;
        {
            let mut recorder = ctx.new_visitor(writer, true);
            event.record(&mut recorder);
        }
        writeln!(writer)
    }
}

impl<N, T> FormatEvent<N> for Format<Compact, T>
where
    N: for<'a> crate::NewVisitor<'a>,
    T: FormatTime,
{
    fn format_event(
        &self,
        ctx: &span::Context<N>,
        writer: &mut dyn fmt::Write,
        event: &Event,
    ) -> fmt::Result {
        let meta = event.metadata();
        time::write(&self.timer, writer)?;
        write!(
            writer,
            "{} {}{}: ",
            FmtLevel(meta.level(), self.ansi),
            FmtCtx(&ctx, self.ansi),
            meta.target()
        )?;
        {
            let mut recorder = ctx.new_visitor(writer, true);
            event.record(&mut recorder);
        }
        ctx.with_current(|(_, span)| write!(writer, " {}", span.fields()))
            .unwrap_or(Ok(()))?;
        writeln!(writer)
    }
}

pub struct NewRecorder;

pub struct Recorder<'a> {
    writer: &'a mut dyn Write,
    is_empty: bool,
}

impl<'a> Recorder<'a> {
    pub fn new(writer: &'a mut dyn Write, is_empty: bool) -> Self {
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

impl<'a> crate::NewVisitor<'a> for NewRecorder {
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

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.maybe_pad();
        let _ = match field.name() {
            "message" => write!(self.writer, "{:?}", value),
            name if name.starts_with("r#") => write!(self.writer, "{}={:?}", &name[2..], value),
            name => write!(self.writer, "{}={:?}", name, value),
        };
    }
}

struct FmtCtx<'a, N: 'a>(&'a span::Context<'a, N>, bool);

#[cfg(feature = "ansi")]
impl<'a, N> fmt::Display for FmtCtx<'a, N>
where
    N: crate::NewVisitor<'a>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut seen = false;
        self.0.visit_spans(|_, span| {
            if seen {
                f.pad(":")?;
            }
            seen = true;

            if self.1 {
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
        self.0.visit_spans(|_, span| {
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

struct FullCtx<'a, N: 'a>(&'a span::Context<'a, N>, bool);

#[cfg(feature = "ansi")]
impl<'a, N> fmt::Display for FullCtx<'a, N>
where
    N: crate::NewVisitor<'a>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut seen = false;
        let style = if self.1 {
            Some(Style::new().bold())
        } else {
            None
        };
        self.0.visit_spans(|_, span| {
            if let Some(style) = style {
                write!(f, "{}", style.paint(span.name()))?;
            } else {
                write!(f, "{}", span.name())?;
            }

            seen = true;

            let fields = span.fields();
            if !fields.is_empty() {
                if let Some(style) = style {
                    write!(f, "{}{}{}", style.paint("{"), fields, style.paint("}"))?;
                } else {
                    write!(f, "{{{}}}", fields)?;
                }
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
        self.0.visit_spans(|_, span| {
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

struct FmtLevel<'a>(&'a Level, bool);

#[cfg(not(feature = "ansi"))]
impl<'a> fmt::Display for FmtLevel<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self.0 {
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.1 {
            match *self.0 {
                Level::TRACE => write!(f, "{}", Colour::Purple.paint("TRACE")),
                Level::DEBUG => write!(f, "{}", Colour::Blue.paint("DEBUG")),
                Level::INFO => write!(f, "{}", Colour::Green.paint(" INFO")),
                Level::WARN => write!(f, "{}", Colour::Yellow.paint(" WARN")),
                Level::ERROR => write!(f, "{}", Colour::Red.paint("ERROR")),
            }
        } else {
            match *self.0 {
                Level::TRACE => f.pad("TRACE"),
                Level::DEBUG => f.pad("DEBUG"),
                Level::INFO => f.pad("INFO"),
                Level::WARN => f.pad("WARN"),
                Level::ERROR => f.pad("ERROR"),
            }
        }
    }
}
