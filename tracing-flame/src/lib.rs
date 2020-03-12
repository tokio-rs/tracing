//! A Tracing [Layer][`FlameLayer`] for generating a folded stack trace for generating flamegraphs
//! and flamecharts with [`inferno`]
//!
//! # Overview
//!
//! [`tracing`] is a framework for instrumenting Rust programs to collect
//! scoped, structured, and async-aware diagnostics. `tracing-flame` provides helpers
//! for consuming `tracing` instrumentation that can later be visualized as a
//! flamegraph/flamechart. Flamegraphs/flamecharts are useful for identifying performance
//! issues bottlenecks in an application. For more details, see Brendan Gregg's [post]
//! on flamegraphs.
//!
//! [post]: http://www.brendangregg.com/flamegraphs.html
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
//! use std::{fs::File, io::BufWriter};
//! use tracing_flame::{FlameGuard, FlameLayer};
//! use tracing_subscriber::{registry::Registry, prelude::*, fmt};
//!
//! fn setup_global_subscriber() -> FlameGuard<BufWriter<File>> {
//!     let fmt_layer = fmt::Layer::default();
//!
//!     let flame_layer = FlameLayer::with_file("./tracing.folded").unwrap();
//!     let _guard = flame_layer.flush_on_drop();
//!
//!     let subscriber = Registry::default()
//!         .with(fmt_layer)
//!         .with(flame_layer);
//!
//!     tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");
//!     _guard
//! }
//!
//! // your code here ..
//! ```
//!
//! As an alternative, you can provide _any_ type that implements `std::io::Write` to
//! `FlameLayer::new`.
//!
//! ## Generating The Image
//!
//! To convert the textual representation of a flamegraph to a visual one, first install `inferno`:
//!
//! ```console
//! cargo install inferno
//! ```
//!
//! Then, pass the file created by `FlameLayer` into `inferno-flamegraph`:
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
//! By default, `inferno-flamegraph` creates flamegraphs. Flamegraphs operate by
//! that collapsing identical stack frames and sorting them on the frame's names.
//!
//! This behavior is great for multithreaded programs and long-running programs
//! where the same frames occur _many_ times, for short durations, because it reduces
//! noise in the graph and gives the reader a better idea of the
//! overall time spent in each part of the application.
//!
//! However, it is sometimes desirable to preserve the _exact_ ordering of events
//! as they were emitted by `tracing-flame`, so that it is clear when each
//! span is entered relative to others and get an accurate visual trace of
//! the execution of your program. This representation is best created with a
//! `flamechart`, which _does not_ sort or collapse identical stack frames.
//!
//! [`tracing`]: https://docs.rs/tracing
//! [`inferno`]: https://docs.rs/inferno
//! [`FlameLayer`]: struct.FlameLayer.html
mod error;

use error::{Kind, Error};
use std::fmt;
use std::fmt::Write as _;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;
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
    S: Subscriber,
    W: Write + 'static,
{
    out: Arc<Mutex<W>>,
    last_event: Mutex<Instant>,
    _inner: PhantomData<S>,
}

pub struct FlameGuard<W>
where
    W: Write + 'static,
{
    out: Arc<Mutex<W>>,
}

impl<S, W> FlameLayer<S, W>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: Write + 'static,
{
    pub fn new(writer: W) -> Self {
        Self {
            out: Arc::new(Mutex::new(writer)),
            last_event: Mutex::new(Instant::now()),
            _inner: PhantomData,
        }
    }

    pub fn flush_on_drop(&self) -> FlameGuard<W> {
        FlameGuard {
            out: self.out.clone(),
        }
    }
}

impl<W> Drop for FlameGuard<W>
where
    W: Write + 'static,
{
    fn drop(&mut self) {
        match self.out.lock().unwrap().flush() {
            Ok(_) => (),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
}

impl<S> FlameLayer<S, BufWriter<File>>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    pub fn with_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let file = File::create(path).map_err(|source| Kind::IO {
            path: path.into(),
            source,
        })?;
        let writer = BufWriter::new(file);
        Ok(Self::new(writer))
    }
}

impl<S, W> Layer<S> for FlameLayer<S, W>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: Write + 'static,
{
    fn new_span(&self, _: &Attributes, id: &Id, ctx: Context<S>) {
        let samples = self.time_since_last_event().unwrap();

        let first = ctx.span(id).expect("expected: span id exists in registry");
        let parents = first.from_root();

        let mut stack = String::new();

        stack.push_str("tracing");

        for parent in parents {
            stack.push_str("; ");
            write(&mut stack, parent).expect("expected: write to String never fails");
        }

        write!(&mut stack, " {}", samples.as_nanos())
            .expect("expected: write to String never fails");

        let _ = writeln!(*self.out.lock().unwrap(), "{}", stack);
    }

    #[allow(clippy::needless_return)]
    fn on_close(&self, id: Id, ctx: Context<S>) {
        let panicking = std::thread::panicking();
        macro_rules! expect {
            ($e:expr, $msg:literal) => {
                if panicking {
                    return;
                } else {
                    $e.expect($msg)
                }
            };
            ($e:expr) => {
                if panicking {
                    return;
                } else {
                    $e.unwrap()
                }
            };
        }

        let samples = expect!(self.time_since_last_event());
        let first = expect!(ctx.span(&id), "expected: span id exists in registry");
        let parents = first.from_root();

        let mut stack = String::new();
        stack.push_str("tracing; ");

        for parent in parents {
            expect!(
                write(&mut stack, parent),
                "expected: write to String never fails"
            );
            stack.push_str("; ");
        }

        expect!(
            write(&mut stack, first),
            "expected: write to String never fails"
        );
        expect!(
            write!(&mut stack, " {}", samples.as_nanos()),
            "expected: write to String never fails"
        );

        let _ = writeln!(*expect!(self.out.lock()), "{}", stack);
    }
}

impl<S, W> FlameLayer<S, W>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: Write + 'static,
{
    fn time_since_last_event(&self) -> Result<Duration, PoisonError<MutexGuard<'_, Instant>>> {
        let now = Instant::now();
        let mut guard = self.last_event.lock()?;
        let prev = *guard;
        let diff = now - prev;
        *guard = now;
        Ok(diff)
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
