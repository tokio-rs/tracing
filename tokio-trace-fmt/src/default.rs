use ::span;

use tokio_trace_core::{
    Event, Level, field::{self, Field},
};

use std::{
    fmt,
    io::{self, Write},
};

#[cfg(feature = "ansi")]
use ansi_term::{Colour, Style};

pub fn fmt_event(ctx: &span::Context, f: &mut Write, event: &Event) -> io::Result<()> {
    let meta = event.metadata();
    write!(f, "{} {}{}:", FmtLevel(meta.level()), FmtCtx(&ctx), meta.target())?;
    {
        let mut recorder = Recorder(f);
        event.record(&mut recorder);
    }
    ctx.with_current(|(_, span)| {
        write!(f, "{}", span.fields)
    }).unwrap_or(Ok(()))?;
    writeln!(f, "")
}

pub struct NewRecorder;

pub struct Recorder<'a>(&'a mut Write);

impl<'a> ::NewRecorder<'a> for NewRecorder {
    type Recorder = Recorder<'a>;

    #[inline]
    fn make(&self, writer: &'a mut Write) -> Self::Recorder {
        Recorder(writer)
    }
}

impl<'a> field::Record for Recorder<'a> {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.record_debug(field, &format_args!("{}", value))
        } else {
            self.record_debug(field, &value)
        }
    }

    fn record_debug(&mut self, field: &Field, value: &fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.0, " {:?}", value);
        } else {
            let _ = write!(self.0, " {}={:?}", field, value);
        }
    }
}

struct FmtCtx<'a>(&'a span::Context<'a>);

#[cfg(feature = "ansi")]
impl<'a> fmt::Display for FmtCtx<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut seen = false;
        self.0.with_spans(|(_, span)| {
            if seen {
                write!(f, ":{}", Style::new().bold().paint(span.name()))?;
            } else {
                write!(f, "{}", Style::new().bold().paint(span.name()))?;
                seen = true;
            }
            Ok(())
        })?;
        if seen {
            f.pad(" ")?;
        }
        Ok(())
    }
}

#[cfg(not(feature = "ansi"))]
impl<'a> fmt::Display for FmtCtx<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut seen = false;
        self.0.fmt_spans(|(_, span)| {
            if seen {
                write!(f, ":{}", span.name())?;
            } else {
                write!(f, "{}", span.name())?;
                seen = true;
            }
            Ok(())
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
        match self.0 {
            &Level::TRACE => f.pad("TRACE"),
            &Level::DEBUG => f.pad("DEBUG"),
            &Level::INFO => f.pad("INFO"),
            &Level::WARN => f.pad("WARN"),
            &Level::ERROR => f.pad("ERROR"),
        }
    }
}

#[cfg(feature = "ansi")]
impl<'a> fmt::Display for FmtLevel<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            &Level::TRACE => write!(f, "{}", Colour::Purple.paint("TRACE")),
            &Level::DEBUG => write!(f, "{}", Colour::Blue.paint("DEBUG")),
            &Level::INFO => write!(f, "{}", Colour::Green.paint(" INFO")),
            &Level::WARN => write!(f, "{}", Colour::Yellow.paint(" WARN")),
            &Level::ERROR => write!(f, "{}", Colour::Red.paint("ERROR")),
        }
    }
}
