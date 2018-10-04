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
use super::tokio_trace::{self, Level};


use std::{
    collections::hash_map::RandomState,
    hash::{BuildHasher, Hash, Hasher},
    fmt,
    io::{self, Write},
    sync::{Mutex, atomic::{AtomicUsize, Ordering}},
    time::SystemTime,
};

pub struct SloggishSubscriber {
    indent_amount: usize,
    stderr: io::Stderr,
    hash_builder: RandomState,
    stack: Mutex<Vec<u64>>,
    ids: AtomicUsize,
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
            indent_amount,
            stderr: io::stderr(),
            hash_builder: RandomState::new(),
            stack: Mutex::new(vec![]),
            ids: AtomicUsize::new(0),
        }
    }

    fn print_kvs<'a, I>(&self, writer: &mut impl Write, kvs: I, leading: &str) -> io::Result<()>
    where
        I: IntoIterator<Item = (&'a str, &'a dyn tokio_trace::Value)>,
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
        meta: &tokio_trace::Meta,
    ) -> io::Result<()> {
        write!(
            writer,
            "{level} {target} ",
            level = ColorLevel(meta.level),
            target = meta.target.unwrap_or(meta.module_path),
        )
    }

    fn print_indent(&self, writer: &mut impl Write, indent: usize,) -> io::Result<()> {
        for _ in 0..(indent * self.indent_amount) {
            write!(writer, " ")?;
        }
        Ok(())
    }

    fn hash_span(&self, span: &tokio_trace::SpanData) -> u64 {
        let mut hasher = self.hash_builder.build_hasher();
        span.hash(&mut hasher);
        hasher.finish()
    }
}

impl tokio_trace::Subscriber for SloggishSubscriber {
    fn enabled(&self, _metadata: &tokio_trace::Meta) -> bool {
        true
    }

    fn new_span(&self, _: &tokio_trace::span::NewSpan) -> tokio_trace::span::Id {
        let next = self.ids.fetch_add(1, Ordering::SeqCst) as u64;
        tokio_trace::span::Id::from_u64(next)
    }

    #[inline]
    fn observe_event<'event, 'meta: 'event>(&self, event: &'event tokio_trace::Event<'event, 'meta>) {
        let mut stderr = self.stderr.lock();
        let parent_hash = event.parent
            .as_ref()
            .map(|span| self.hash_span(span));

        let stack = self.stack.lock().unwrap();
        if let Some(idx) = stack.iter()
            .position(|hash| parent_hash.as_ref()
                .map(|p| p == hash)
                .unwrap_or(false)
            )
        {
            self.print_indent(&mut stderr, idx + 1).unwrap();
        }
        write!(
            &mut stderr,
            "{} ",
            humantime::format_rfc3339_seconds(SystemTime::now())
        ).unwrap();
        self.print_meta(&mut stderr, event.meta).unwrap();
        write!(
            &mut stderr,
            "{}",
            Style::new().bold().paint(format!("{}", event.message))
        ).unwrap();
        self.print_kvs(&mut stderr, event.fields(), ", ").unwrap();
        write!(&mut stderr, "\n").unwrap();
    }

    #[inline]
    fn enter(&self, span: &tokio_trace::SpanData) {
        let mut stderr = self.stderr.lock();

        let span_hash = self.hash_span(span);
        let parent_hash = span.parent()
            .map(|parent| self.hash_span(parent));

        let mut stack = self.stack.lock().unwrap();
        if stack.iter().any(|hash| hash == &span_hash) {
            // We are already in this span, do nothing.
            return;
        } else {
            let indent = if let Some(idx) = stack
                .iter()
                .position(|hash| parent_hash
                    .map(|p| hash == &p)
                    .unwrap_or(false))
            {
                let idx = idx + 1;
                stack.truncate(idx);
                idx
            } else {
                stack.clear();
                0
            };
            self.print_indent(&mut stderr, indent).unwrap();
            stack.push(span_hash);
            self.print_kvs(&mut stderr, span.fields(), "").unwrap();
            write!(&mut stderr, "\n").unwrap();
        }
    }

    #[inline]
    fn exit(&self, _span: &tokio_trace::SpanData) {}
}
