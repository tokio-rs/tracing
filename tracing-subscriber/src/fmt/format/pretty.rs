use super::*;
use crate::{
    field::{VisitFmt, VisitOutput},
    fmt::fmt_subscriber::{FmtContext, FormattedFields},
    registry::LookupSpan,
};

use std::{
    fmt::{self, Write},
    iter,
};
use tracing_core::{
    field::{self, Field},
    Collect, Event, Level,
};

#[cfg(feature = "tracing-log")]
use tracing_log::NormalizeEvent;

use ansi_term::{Colour, Style};

/// An excessively pretty, human-readable event formatter.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Pretty {
    display_location: bool,
}

/// The [visitor] produced by [`Pretty`]'s [`MakeVisitor`] implementation.
///
/// [visitor]: ../../field/trait.Visit.html
/// [`DefaultFields`]: struct.DefaultFields.html
/// [`MakeVisitor`]: ../../field/trait.MakeVisitor.html
pub struct PrettyVisitor<'a> {
    writer: &'a mut dyn Write,
    is_empty: bool,
    style: Style,
    result: fmt::Result,
}

// === impl Pretty ===

impl Default for Pretty {
    fn default() -> Self {
        Self {
            display_location: true,
        }
    }
}

impl Pretty {
    fn style_for(level: &Level) -> Style {
        match *level {
            Level::TRACE => Style::new().fg(Colour::Purple),
            Level::DEBUG => Style::new().fg(Colour::Blue),
            Level::INFO => Style::new().fg(Colour::Green),
            Level::WARN => Style::new().fg(Colour::Yellow),
            Level::ERROR => Style::new().fg(Colour::Red),
        }
    }

    /// Sets whether the event's source code location is displayed.
    ///
    /// This defaults to `true`.
    pub fn with_source_location(self, display_location: bool) -> Self {
        Self {
            display_location,
            ..self
        }
    }
}

impl<T> Format<Pretty, T> {
    /// Sets whether or not the source code location from which an event
    /// originated is displayed.
    ///
    /// This defaults to `true`.
    pub fn with_source_location(mut self, display_location: bool) -> Self {
        self.format = self.format.with_source_location(display_location);
        self
    }
}

impl<C, N, T> FormatEvent<C, N> for Format<Pretty, T>
where
    C: Collect + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
    T: FormatTime,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, C, N>,
        writer: &mut dyn fmt::Write,
        event: &Event<'_>,
    ) -> fmt::Result {
        #[cfg(feature = "tracing-log")]
        let normalized_meta = event.normalized_metadata();
        #[cfg(feature = "tracing-log")]
        let meta = normalized_meta.as_ref().unwrap_or_else(|| event.metadata());
        #[cfg(not(feature = "tracing-log"))]
        let meta = event.metadata();
        write!(writer, "  ")?;
        time::write(&self.timer, writer, true)?;

        let style = if self.display_level {
            Pretty::style_for(meta.level())
        } else {
            Style::new()
        };

        if self.display_target {
            let bold = style.bold();
            write!(
                writer,
                "{}{}{}: ",
                bold.prefix(),
                meta.target(),
                bold.infix(style)
            )?;
        }
        let mut v = PrettyVisitor::new(writer, true).with_style(style);
        event.record(&mut v);
        v.finish()?;
        writer.write_char('\n')?;

        let dimmed = Style::new().dimmed().italic();
        let thread = self.display_thread_name || self.display_thread_id;
        if let (true, Some(file), Some(line)) =
            (self.format.display_location, meta.file(), meta.line())
        {
            write!(
                writer,
                "    {} {}:{}{}",
                dimmed.paint("at"),
                file,
                line,
                dimmed.paint(if thread { " " } else { "\n" })
            )?;
        } else if thread {
            write!(writer, "    ")?;
        }

        if thread {
            write!(writer, "{} ", dimmed.paint("on"))?;
            let thread = std::thread::current();
            if self.display_thread_name {
                if let Some(name) = thread.name() {
                    write!(writer, "{}", name)?;
                    if self.display_thread_id {
                        write!(writer, " ({:?})", thread.id())?;
                    }
                } else if !self.display_thread_id {
                    write!(writer, " {:?}", thread.id())?;
                }
            } else if self.display_thread_id {
                write!(writer, " {:?}", thread.id())?;
            }
            writer.write_char('\n')?;
        }

        let bold = Style::new().bold();
        let span = event
            .parent()
            .and_then(|id| ctx.span(&id))
            .or_else(|| ctx.lookup_current());

        let scope = span.into_iter().flat_map(|span| {
            let parents = span.parents();
            iter::once(span).chain(parents)
        });

        for span in scope {
            let meta = span.metadata();
            if self.display_target {
                write!(
                    writer,
                    "    {} {}::{}",
                    dimmed.paint("in"),
                    meta.target(),
                    bold.paint(meta.name()),
                )?;
            } else {
                write!(
                    writer,
                    "    {} {}",
                    dimmed.paint("in"),
                    bold.paint(meta.name()),
                )?;
            }

            let ext = span.extensions();
            let fields = &ext
                .get::<FormattedFields<N>>()
                .expect("Unable to find FormattedFields in extensions; this is a bug");
            if !fields.is_empty() {
                write!(writer, " {} {}", dimmed.paint("with"), fields)?;
            }
            writer.write_char('\n')?;
        }

        writer.write_char('\n')
    }
}

impl<'writer> FormatFields<'writer> for Pretty {
    fn format_fields<R: RecordFields>(
        &self,
        writer: &'writer mut dyn fmt::Write,
        fields: R,
    ) -> fmt::Result {
        let mut v = PrettyVisitor::new(writer, true);
        fields.record(&mut v);
        v.finish()
    }

    fn add_fields(&self, current: &'writer mut String, fields: &span::Record<'_>) -> fmt::Result {
        let empty = current.is_empty();
        let mut v = PrettyVisitor::new(current, empty);
        fields.record(&mut v);
        v.finish()
    }
}

// === impl PrettyVisitor ===

impl<'a> PrettyVisitor<'a> {
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
            style: Style::default(),
            result: Ok(()),
        }
    }

    pub(crate) fn with_style(self, style: Style) -> Self {
        Self { style, ..self }
    }

    fn maybe_pad(&mut self) {
        if self.is_empty {
            self.is_empty = false;
        } else {
            self.result = write!(self.writer, ", ");
        }
    }
}

impl<'a> field::Visit for PrettyVisitor<'a> {
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
            let bold = self.style.bold();
            self.record_debug(
                field,
                &format_args!(
                    "{}, {}{}.source{}: {}",
                    value,
                    bold.prefix(),
                    field,
                    bold.infix(self.style),
                    source,
                ),
            )
        } else {
            self.record_debug(field, &format_args!("{}", value))
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if self.result.is_err() {
            return;
        }
        let bold = self.style.bold();
        self.maybe_pad();
        self.result = match field.name() {
            "message" => write!(self.writer, "{}{:?}", self.style.prefix(), value,),
            // Skip fields that are actually log metadata that have already been handled
            #[cfg(feature = "tracing-log")]
            name if name.starts_with("log.") => Ok(()),
            name if name.starts_with("r#") => write!(
                self.writer,
                "{}{}{}: {:?}",
                bold.prefix(),
                &name[2..],
                bold.infix(self.style),
                value
            ),
            name => write!(
                self.writer,
                "{}{}{}: {:?}",
                bold.prefix(),
                name,
                bold.infix(self.style),
                value
            ),
        };
    }
}

impl<'a> VisitOutput<fmt::Result> for PrettyVisitor<'a> {
    fn finish(self) -> fmt::Result {
        write!(self.writer, "{}", self.style.suffix())?;
        self.result
    }
}

impl<'a> VisitFmt for PrettyVisitor<'a> {
    fn writer(&mut self) -> &mut dyn fmt::Write {
        self.writer
    }
}

impl<'a> fmt::Debug for PrettyVisitor<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PrettyVisitor")
            .field("writer", &format_args!("<dyn fmt::Write>"))
            .field("is_empty", &self.is_empty)
            .field("result", &self.result)
            .field("style", &self.style)
            .finish()
    }
}
