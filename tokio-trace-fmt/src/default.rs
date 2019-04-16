use span;

use tokio_trace_core::{
    field::{self, Field},
    Event, Level,
};

use std::fmt::{self, Write};

#[cfg(feature = "ansi")]
use ansi_term::{Colour, Style};

pub fn fmt_event<N>(ctx: &span::Context<N>, f: &mut Write, event: &Event) -> fmt::Result
where
    N: for<'a> ::NewVisitor<'a>,
{
    let meta = event.metadata();
    write!(
        f,
        "{} {}{}: ",
        FmtLevel(meta.level()),
        FmtCtx(&ctx),
        meta.target()
    )?;
    {
        let mut recorder = ctx.new_visitor(f);
        event.record(&mut recorder);
    }
    ctx.with_current(|(_, span)| {
        span.fields()
            .try_for_each(|(_, value)| write!(f, " {}", value))
    })
    .unwrap_or(Ok(()))?;
    writeln!(f)
}

pub fn fmt_verbose<N>(ctx: &span::Context<N>, f: &mut Write, event: &Event) -> fmt::Result
where
    N: for<'a> ::NewVisitor<'a>,
{
    let meta = event.metadata();
    write!(
        f,
        "{} {}{}: ",
        FmtLevel(meta.level()),
        FullCtx(&ctx),
        meta.target()
    )?;
    {
        let mut recorder = ctx.new_visitor(f);
        event.record(&mut recorder);
    }
    writeln!(f)
}

pub struct NewRecorder;

pub struct Recorder<'a> {
    writer: &'a mut Write,
}

impl<'a> Recorder<'a> {
    pub fn new(writer: &'a mut Write) -> Self {
        Self { writer }
    }
}

impl<'a> ::NewVisitor<'a> for NewRecorder {
    type Visitor = Recorder<'a>;

    #[inline]
    fn make(&self, writer: &'a mut fmt::Write) -> Self::Visitor {
        Recorder::new(writer)
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

    fn record_debug(&mut self, field: &Field, value: &fmt::Debug) {
        let _ = match field.name() {
            "message" => write!(self.writer, "{:?}", value),
            name if name.starts_with("r#") => write!(self.writer, "{}={:?}", &name[2..], value),
            name => write!(self.writer, "{}={:?}", name, value),
        };
    }
}

struct FmtCtx<'a, N: 'a>(&'a span::Context<'a, N>);

#[cfg(feature = "ansi")]
impl<'a, N> fmt::Display for FmtCtx<'a, N>
where
    N: ::NewVisitor<'a>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut seen = false;
        self.0.visit_spans(|_, span| {
            if seen {
                f.pad(":")?;
            }
            seen = true;
            write!(f, "{}", Style::new().bold().paint(span.name()))
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

struct FullCtx<'a, N: 'a>(&'a span::Context<'a, N>);

#[cfg(feature = "ansi")]
impl<'a, N> fmt::Display for FullCtx<'a, N>
where
    N: ::NewVisitor<'a>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut seen = false;
        let style = Style::new().bold();
        self.0.visit_spans(|_, span| {
            write!(f, "{}", style.paint(span.name()))?;
            seen = true;

            span.fields().try_for_each(|(_, value)| {
                write!(f, " {}{}{}", style.paint("{"), value, style.paint("}"))
            })?;
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

struct FmtLevel<'a>(&'a Level);

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
        match *self.0 {
            Level::TRACE => write!(f, "{}", Colour::Purple.paint("TRACE")),
            Level::DEBUG => write!(f, "{}", Colour::Blue.paint("DEBUG")),
            Level::INFO => write!(f, "{}", Colour::Green.paint(" INFO")),
            Level::WARN => write!(f, "{}", Colour::Yellow.paint(" WARN")),
            Level::ERROR => write!(f, "{}", Colour::Red.paint("ERROR")),
        }
    }
}
