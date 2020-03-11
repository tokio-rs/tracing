//! A Tracing [Layer][`FlameLayer`] for generating a folded stack trace for generating flamegraphs
//! and flamecharts with [`inferno`]
//!
//! # Overview
//!
//! [`tracing`] is a framework for instrumenting Rust programs to collect
//! scoped, structured, and async-aware diagnostics. This crate provides helpers
//! for consuming tracing instrumentation for visualization as a
//! flamegraph/flamechart which can be useful for finding performance
//! bottlenecks.
//!
//! ## Usage
//!
//! This crate is meant to be used as a two step process. First, you capture a
//! textual representation of the spans that are entered and exited with the
//! [`FlameLayer`] then you feed these into `inferno-flamegraph` to generate the
//! flamegraph/flamechart image.
//!
//! ## Layer Setup
//!
//! ```rust
//! fn setup_global_subscriber() {
//!     use tracing_flame::FlameLayer;
//!     use tracing_subscriber::{registry::Registry, prelude::*};
//!
//!     let subscriber = Registry::default()
//!         .with(FlameLayer::write_to_file("./tracing2.folded").unwrap());
//!
//!     tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");
//! }
//!
//! // your code here ..
//! ```
//!
//! Alternatively you can provide any type that implements `std::io::Write` to
//! `FlameLayer::new`.
//!
//! ## Generating The Image
//!
//! To convert your text representation to a visual one first install inferno
//!
//! ```console
//! cargo install inferno
//! ```
//!
//! Then pass the file your error layer created into `inferno-flamegraph`
//!
//! ```console
//! # flamegraph
//! cat tracing.folded | inferno-flamegraph > tracing-flamegraph.svg
//!
//! #flamechart
//! cat tracing.folded | inferno-flamegraph --flamechart > tracing-flamechart.svg
//! ```
//!
//! ## Differences between `flamegraph`s and `flamechart`s
//!
//! By default `inferno-flamegraph` creates `flamegraph`s. Flamegraphs is that
//! they collapse identical stack frames and sort them based on their names.
//! This is great for multithreaded programs and long running programs where the
//! same frames happen many times for short durations because it simplifies the
//! graph and gives you a broad idea of overall time spent in each part of the
//! stack.
//!
//! However it is sometimes desirable to preserve the exact ordering of events
//! as they were emitted by `tracing-flame`, so you can see exactly when each
//! span is entered relative to the other and get an accurate visual trace of
//! the execution of your program. To get this style of representation you
//! should instead generate a `flamechart`, which does not sort or collapse
//! identical stack frames.
//!
//! [`tracing`]: https://docs.rs/tracing
//! [`inferno`]: https://docs.rs/inferno
//! [`FlameLayer`]: struct.FlameLayer.html
mod error;

use error::Error;
use std::fmt;
use std::fmt::Write as _;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::span::Attributes;
use tracing::Id;
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::registry::SpanRef;
use tracing_subscriber::Layer;

pub struct FlameLayer<S, W>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: Write + 'static,
{
    out: Mutex<W>,
    last_event: Mutex<Instant>,
    _inner: PhantomData<S>,
}

impl<S, W> FlameLayer<S, W>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: Write + 'static,
{
    pub fn new(writer: W) -> Self {
        Self {
            out: Mutex::new(writer),
            last_event: Mutex::new(Instant::now()),
            _inner: PhantomData,
        }
    }
}

impl<S> FlameLayer<S, File>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    pub fn write_to_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let file = File::create(path).map_err(|source| Error::IO {
            path: path.into(),
            source,
        })?;
        // let writer = BufWriter::new(file);
        Ok(Self::new(file))
    }
}

impl<S, W> Layer<S> for FlameLayer<S, W>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: Write + 'static,
{
    fn new_span(&self, _: &Attributes, id: &Id, ctx: Context<S>) {
        let samples = self.time_since_last_event();

        let mut spans = SpanIter::new(id.clone(), &ctx);
        let _ = spans.next();
        let spans = spans.rev();
        let mut stack = String::new();

        stack
            .write_str("tracing")
            .expect("expected: write to String never fails");

        for parent in spans {
            stack
                .write_str("; ")
                .expect("expected: write to String never fails");
            write(&mut stack, parent).expect("expected: write to String never fails");
        }

        write!(&mut stack, " {}", samples.as_nanos())
            .expect("expected: write to String never fails");
        writeln!(
            *self.out.lock().expect("expected: lock is never poisoned"),
            "{}",
            stack
        )
        .expect("expected: write to String never fails");
    }

    fn on_close(&self, id: Id, ctx: Context<S>) {
        let samples = self.time_since_last_event();
        let mut spans = SpanIter::new(id, &ctx);
        let first = spans.next();

        let mut stack = String::new();
        stack
            .write_str("tracing; ")
            .expect("expected: write to String never fails");

        for parent in spans.rev() {
            write(&mut stack, parent).expect("expected: write to String never fails");
            stack
                .write_str("; ")
                .expect("expected: write to String never fails");
        }

        write(&mut stack, first.expect("expected: always at least 1 span"))
            .expect("expected: write to String never fails");
        write!(&mut stack, " {}", samples.as_nanos())
            .expect("expected: write to String never fails");
        writeln!(
            *self.out.lock().expect("expected: lock is never poisoned"),
            "{}",
            stack
        )
        .expect("expected: write to String never fails");
    }
}

impl<S, W> FlameLayer<S, W>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: Write + 'static,
{
    fn time_since_last_event(&self) -> Duration {
        let now = Instant::now();
        let mut guard = self
            .last_event
            .lock()
            .expect("expected: lock is never poisoned");
        let prev = *guard;
        let diff = now - prev;
        *guard = now;
        diff
    }
}

fn write<S>(dest: &mut String, span: SpanRef<'_, S>) -> fmt::Result
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    write!(dest, "{}", span.name())?;

    if let Some(file) = span.metadata().file() {
        write!(dest, ":{}", file)?;
    }

    if let Some(line) = span.metadata().line() {
        write!(dest, ":{}", line)?;
    }

    Ok(())
}

struct SpanIter<'a, S>
where
    S: for<'span> LookupSpan<'span>,
{
    spans: std::vec::IntoIter<SpanRef<'a, S>>,
}

impl<'a, S> SpanIter<'a, S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn new(id: Id, ctx: &'a Context<'a, S>) -> SpanIter<'a, S> {
        let mut spans = Vec::new();
        let mut curr_span = ctx.span(&id);

        while let Some(span) = curr_span {
            curr_span = span.parent();
            spans.push(span);
        }

        Self {
            spans: spans.into_iter(),
        }
    }
}

impl<'a, S> DoubleEndedIterator for SpanIter<'a, S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.spans.next_back()
    }
}

impl<'a, S> Iterator for SpanIter<'a, S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    type Item = SpanRef<'a, S>;

    fn next(&mut self) -> Option<Self::Item> {
        self.spans.next()
    }
}
