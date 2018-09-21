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
#[macro_use]
extern crate tokio_trace;
extern crate ansi_term;

use ansi_term::{Color, Style};
use tokio_trace::Level;

use std::{
    fmt,
    io::{self, Write},
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

struct SloggishSubscriber {
    indent: AtomicUsize,
    indent_amount: usize,
    stderr: io::Stderr,
}

struct ColorLevel(Level);

impl fmt::Display for ColorLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Level::Trace => Color::Purple.paint("TRACE"),
            Level::Debug => Color::Blue.paint("DEBUG"),
            Level::Info => Color::Green.paint("INFO "),
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
        }
    }

    fn print_kvs<'a, I>(&self, writer: &mut impl Write, kvs: I, leading: &str) -> io::Result<()>
    where
        I: IntoIterator<Item = (&'static str, &'a dyn tokio_trace::Value)>,
    {
        let mut kvs = kvs.into_iter();
        if let Some((k, v)) = kvs.next() {
            write!(writer, "{}{}: {:?}", leading, Style::new().bold().paint(k), v)?;
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
        let indent = self.indent.load(Ordering::Acquire);
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
        self.print_meta(&mut stderr, event.static_meta).unwrap();
        self.print_indent(&mut stderr).unwrap();
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
        self.print_meta(&mut stderr, span.meta()).unwrap();
        let indent = self.print_indent(&mut stderr).unwrap();
        self.print_kvs(&mut stderr, span.fields(), "").unwrap();
        write!(&mut stderr, "\n").unwrap();
        self.indent
            .compare_and_swap(indent, indent + self.indent_amount, Ordering::Release);
    }

    #[inline]
    fn exit(&self, _span: &tokio_trace::Span, _at: Instant) {
        self.indent.fetch_sub(self.indent_amount, Ordering::Release);
    }
}

fn main() {
    tokio_trace::Dispatcher::builder()
        .add_subscriber(SloggishSubscriber::new(2))
        .init();

    let foo = 3;
    event!(Level::Info, { foo = foo, bar = "bar" }, "hello world");

    span!("", version = 5.0).enter(|| {
        span!("server", host = "localhost", port = 8080).enter(|| {
            event!(Level::Info, {}, "starting");
            event!(Level::Info, {}, "listening");
            let peer1 = span!("conn", peer_addr = "82.9.9.9", port = 42381);
            peer1.enter(|| {
                event!(Level::Debug, {}, "connected");
                event!(Level::Debug, { length = 2 }, "message received");
            });
            let peer2 = span!("conn", peer_addr = "8.8.8.8", port = 18230);
            peer2.enter(|| {
                event!(Level::Debug, {}, "connected");
            });
            peer1.enter(|| {
                event!(Level::Warn, { algo = "xor" }, "weak encryption requested");
                event!(Level::Debug, { length = 8 }, "response sent");
                event!(Level::Debug, { }, "disconnected");
            });
            peer2.enter(|| {
                event!(Level::Debug, { length = 5 }, "message received");
                event!(Level::Debug, { length = 8 }, "response sent");
                event!(Level::Debug, { }, "disconnected");
            });
            event!(Level::Error, {}, "internal error");
            event!(Level::Info, {}, "exit");
        })
    });
}
