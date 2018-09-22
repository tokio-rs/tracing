//! A simple example demonstrating how one might implement a custom
//! subscriber.
//!
//! This subscriber implements a tree-structured logger similar to
//! the "compact" formatter in [`slog-term`]. The demo mimicks the
//! example output in the screenshot in the [`slog` README].
//!
//! Note that this logger isn't ready for actual production use.
//! Several corners were cut to make the example simple.
//!
//! [`slog-term`]: https://docs.rs/slog-term/2.4.0/slog_term/
//! [`slog` README]: https://github.com/slog-rs/slog#terminal-output-example
extern crate humantime;
extern crate ansi_term;
use self::ansi_term::{Color, Style};
use tokio_trace::{self, Level};


use std::{
    fmt,
    io::{self, Write},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Instant, SystemTime},
};

pub struct SloggishSubscriber {
    indent: AtomicUsize,
    indent_amount: usize,
    stderr: io::Stderr,
    t0_instant: Instant,
    t0_sys: SystemTime,
}

struct ColorLevel(Level);

impl fmt::Display for ColorLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Level::Trace => Color::Purple.paint("TRACE"),
            Level::Debug => Color::Blue.paint("DEBUG"),
            Level::Info => Color::Green.paint("INFO"),
            Level::Warn => Color::Yellow.paint("WARN "),
            Level::Error => Color::Red.paint("ERROR"),
        }.fmt(f)
    }
}

impl SloggishSubscriber {
    pub fn new(indent_amount: usize) -> Self {
        Self {
            indent: AtomicUsize::new(0),
            indent_amount,
            stderr: io::stderr(),
            t0_instant: Instant::now(),
            t0_sys: SystemTime::now(),
        }
    }

    fn anchor_instant(&self, t1: Instant) -> SystemTime {
        let diff = t1 - self.t0_instant;
        self.t0_sys + diff
    }

    fn print_kvs<'a, I>(&self, writer: &mut impl Write, kvs: I, leading: &str) -> io::Result<()>
    where
        I: IntoIterator<Item = (&'static str, &'a dyn tokio_trace::Value)>,
    {
        let mut kvs = kvs.into_iter();
        if let Some((k, v)) = kvs.next() {
            write!(
                writer,
                "{}{}: {:?}",
                leading,
                Style::new().bold().paint(k),
                v
            )?;
        }
        for (k, v) in kvs {
            write!(writer, ", {}: {:?}", Style::new().bold().paint(k), v)?;
        }
        Ok(())
    }

    fn print_meta(
        &self,
        writer: &mut impl Write,
        meta: &tokio_trace::StaticMeta,
    ) -> io::Result<()> {
        write!(
            writer,
            "{level} {target} ",
            level = ColorLevel(meta.level),
            target = meta.target.unwrap_or(meta.module_path),
        )
    }

    fn print_indent(&self, writer: &mut impl Write) -> io::Result<usize> {
        let indent = self.indent.load(Ordering::SeqCst);
        for _ in 0..indent {
            write!(writer, " ")?;
        }
        Ok(indent)
    }
}

impl tokio_trace::Subscriber for SloggishSubscriber {
    #[inline]
    fn observe_event<'event>(&self, event: &'event tokio_trace::Event<'event>) {
        let mut stderr = self.stderr.lock();
        self.print_indent(&mut stderr).unwrap();
        let t1 = self.anchor_instant(event.timestamp);
        write!(
            &mut stderr,
            "{} ",
            humantime::format_rfc3339_seconds(t1)
        ).unwrap();
        self.print_meta(&mut stderr, event.static_meta).unwrap();
        write!(
            &mut stderr,
            "{}",
            Style::new().bold().paint(format!("{}", event.message))
        ).unwrap();
        self.print_kvs(&mut stderr, event.fields(), ", ").unwrap();
        write!(&mut stderr, "\n").unwrap();
    }

    #[inline]
    fn enter(&self, span: &tokio_trace::Span, _at: Instant) {
        let mut stderr = self.stderr.lock();
        let indent = self.print_indent(&mut stderr).unwrap();
        self.print_kvs(&mut stderr, span.fields(), "").unwrap();
        write!(&mut stderr, "\n").unwrap();
        self.indent
            .compare_and_swap(indent, indent + self.indent_amount, Ordering::SeqCst);
    }

    #[inline]
    fn exit(&self, _span: &tokio_trace::Span, _at: Instant) {
        self.indent.fetch_sub(self.indent_amount, Ordering::SeqCst);
    }
}
