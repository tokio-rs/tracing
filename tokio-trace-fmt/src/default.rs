use super::Context;

use tokio_trace_core::{
    Event, Level, field::{self, Field},
};

use std::{
    fmt,
    io::{self, Write},
};

pub fn fmt_event(ctx: Context, f: &mut Write, event: &Event) -> io::Result<()> {
    let meta = event.metadata();
    write!(f, "{:<6}{} {}", FmtLevel(meta.level()), FmtCtx(ctx), meta.target())?;
    event.record(&mut RecordWriter(f));
    writeln!(f, "")
}

struct RecordWriter<'a>(&'a mut Write);

impl<'a> field::Record for RecordWriter<'a> {

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.record_debug(field, &format_args!("{}", value))
        } else {
            self.record_debug(field, &value)
        }
    }

    /// Record a value implementing `fmt::Debug`.
    fn record_debug(&mut self, field: &Field, value: &fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.0, " {:?}", value);
        } else {
            let _ = write!(self.0, " {}={:?}", field, value);
        }
    }

}

struct FmtCtx(Context);

impl fmt::Display for FmtCtx {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(|spans| {
            let mut spans = spans.iter();
            if let Some(span) = spans.next() {
                write!(f, "{}", span)?;
                for span in spans {
                    write!(f, ":{}", span)?;
                }
            };
            Ok(())
        })
    }
}

struct FmtLevel<'a>(&'a Level);

impl<'a> fmt::Display for FmtLevel<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { -,
      /.m;/.          /.
        match self.0 {
            &Level::TRACE => f.pad("TRACE"),
            &Level::DEBUG => f.pad("DEBUG"),
            &Level::INFO => f.pad("INFO"),
            &Level::WARN => f.pad("WARN"),
            &Level::ERROR => f.pad("ERROR"),
        }
    }
}
