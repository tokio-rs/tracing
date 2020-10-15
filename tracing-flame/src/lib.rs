//! A Tracing [Subscriber][`FlameSubscriber`] for generating a folded stack trace for generating flamegraphs
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
//! *Compiler support: [requires `rustc` 1.42+][msrv]*
//!
//! [msrv]: #supported-rust-versions
//! [post]: http://www.brendangregg.com/flamegraphs.html
//!
//! ## Usage
//!
//! This crate is meant to be used in a two step process:
//!
//! 1. Capture textual representation of the spans that are entered and exited
//!    with [`FlameSubscriber`].
//! 2. Feed the textual representation into `inferno-flamegraph` to generate the
//!    flamegraph or flamechart.
//!
//! *Note*: when using a buffered writer as the writer for a `FlameSubscriber`, it is necessary to
//! ensure that the buffer has been flushed before the data is passed into
//! [`inferno-flamegraph`]. For more details on how to flush the internal writer
//! of the `FlameSubscriber`, see the docs for [`FlushGuard`].
//!
//! ## Subscriber Setup
//!
//! ```rust
//! use std::{fs::File, io::BufWriter};
//! use tracing_flame::FlameSubscriber;
//! use tracing_subscriber::{registry::Registry, prelude::*, fmt};
//!
//! fn setup_global_subscriber() -> impl Drop {
//!     let fmt_subscriber = fmt::Subscriber::default();
//!
//!     let (flame_subscriber, _guard) = FlameSubscriber::with_file("./tracing.folded").unwrap();
//!
//!     let collector = Registry::default()
//!         .with(fmt_subscriber)
//!         .with(flame_subscriber);
//!
//!     tracing::collector::set_global_default(collector).expect("Could not set global default");
//!     _guard
//! }
//!
//! // your code here ..
//! ```
//!
//! As an alternative, you can provide _any_ type that implements `std::io::Write` to
//! `FlameSubscriber::new`.
//!
//! ## Generating the Image
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
//! # flamechart
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
//! _flamechart_, which _does not_ sort or collapse identical stack frames.
//!
//! [`tracing`]: https://docs.rs/tracing
//! [`inferno`]: https://docs.rs/inferno
//! [`FlameLayer`]: struct.FlameLayer.html
//! [`FlushGuard`]: struct.FlushGuard.html
//! [`inferno-flamegraph`]: https://docs.rs/inferno/0.9.5/inferno/index.html#producing-a-flame-graph
//!
//! ## Supported Rust Versions
//!
//! Tracing is built against the latest stable release. The minimum supported
//! version is 1.42. The current Tracing version is not guaranteed to build on
//! Rust versions earlier than the minimum supported version.
//!
//! Tracing follows the same compiler support policies as the rest of the Tokio
//! project. The current stable Rust compiler and the three most recent minor
//! versions before it will always be supported. For example, if the current
//! stable compiler version is 1.45, the minimum supported version will not be
//! increased past 1.42, three minor versions prior. Increasing the minimum
//! supported compiler version is not considered a semver breaking change as
//! long as doing so complies with this policy.
//!
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/logo-type.png",
    html_favicon_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/favicon.ico",
    issue_tracker_base_url = "https://github.com/tokio-rs/tracing/issues/"
)]
#![cfg_attr(docsrs, deny(broken_intra_doc_links))]
#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    unreachable_pub,
    bad_style,
    const_err,
    dead_code,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    private_in_public,
    unconditional_recursion,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true
)]

pub use error::Error;

use error::Kind;
use lazy_static::lazy_static;
use std::cell::Cell;
use std::fmt;
use std::fmt::Write as _;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::span;
use tracing::Collect;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::registry::SpanRef;
use tracing_subscriber::subscribe::Context;
use tracing_subscriber::Subscribe;

mod error;

lazy_static! {
    static ref START: Instant = Instant::now();
}

thread_local! {
    static LAST_EVENT: Cell<Instant> = Cell::new(*START);

    static THREAD_NAME: String = {
        let thread = std::thread::current();
        let mut thread_name = format!("{:?}", thread.id());
        if let Some(name) = thread.name() {
            thread_name += "-";
            thread_name += name;
        }
        thread_name
    };
}

/// A `Subscriber` that records span open/close events as folded flamegraph stack
/// samples.
///
/// The output of `FlameLayer` emulates the output of commands like `perf` once
/// they've been collapsed by `inferno-flamegraph`. The output of this layer
/// should look similar to the output of the following commands:
///
/// ```sh
/// perf record --call-graph dwarf target/release/mybin
/// perf script | inferno-collapse-perf > stacks.folded
/// ```
///
/// # Sample Counts
///
/// Because `tracing-flame` doesn't use sampling, the number at the end of each
/// folded stack trace does not represent a number of samples of that stack.
/// Instead, the numbers on each line are the number of nanoseconds since the
/// last event in the same thread.
///
/// # Dropping and Flushing
///
/// If you use a global subscriber the drop implementations on your various
/// layers will not get called when your program exits. This means that if
/// you're using a buffered writer as the inner writer for the `FlameLayer`
/// you're not guaranteed to see all the events that have been emitted in the
/// file by default.
///
/// To ensure all data is flushed when the program exits, `FlameLayer` exposes
/// the [`flush_on_drop`] function, which returns a [`FlushGuard`]. The `FlushGuard`
/// will flush the writer when it is dropped. If necessary, it can also be used to manually
/// flush the writer.
///
/// [`flush_on_drop`]: struct.FlameLayer.html#method.flush_on_drop
/// [`FlushGuard`]: struct.FlushGuard.html
#[derive(Debug)]
pub struct FlameSubscriber<S, W> {
    out: Arc<Mutex<W>>,
    config: Config,
    _inner: PhantomData<S>,
}

#[derive(Debug)]
struct Config {
    /// Don't include samples where no spans are open
    empty_samples: bool,

    /// Don't include thread_id
    threads_collapsed: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            empty_samples: true,
            threads_collapsed: false,
        }
    }
}

/// An RAII guard for managing flushing a global writer that is
/// otherwise inaccessible.
///
/// This type is only needed when using
/// `tracing::subscriber::set_global_default`, which prevents the drop
/// implementation of layers from running when the program exits.
#[must_use]
#[derive(Debug)]
pub struct FlushGuard<W>
where
    W: Write + 'static,
{
    out: Arc<Mutex<W>>,
}

impl<S, W> FlameSubscriber<S, W>
where
    S: Collect + for<'span> LookupSpan<'span>,
    W: Write + 'static,
{
    /// Returns a new `FlameSubscriber` that outputs all folded stack samples to the
    /// provided writer.
    pub fn new(writer: W) -> Self {
        // Initialize the start used by all threads when initializing the
        // LAST_EVENT when constructing the layer
        let _unused = *START;
        Self {
            out: Arc::new(Mutex::new(writer)),
            config: Default::default(),
            _inner: PhantomData,
        }
    }

    /// Returns a `FlushGuard` which will flush the `FlameSubscriber`'s writer when
    /// it is dropped, or when `flush` is manually invoked on the guard.
    pub fn flush_on_drop(&self) -> FlushGuard<W> {
        FlushGuard {
            out: self.out.clone(),
        }
    }

    /// Configures whether or not periods of time where no spans are entered
    /// should be included in the output.
    ///
    /// Defaults to `true`.
    ///
    /// Setting this feature to false can help with situations where no span is
    /// active for large periods of time. This can include time spent idling, or
    /// doing uninteresting work that isn't being measured.
    /// When a large number of empty samples are recorded, the flamegraph
    /// may be harder to interpret and navigate, since the recorded spans will
    /// take up a correspondingly smaller percentage of the graph. In some
    /// cases, a large number of empty samples may even hide spans which
    /// would otherwise appear in the flamegraph.
    pub fn with_empty_samples(mut self, enabled: bool) -> Self {
        self.config.empty_samples = enabled;
        self
    }

    /// Configures whether or not spans from different threads should be
    /// collapsed into one pool of events.
    ///
    /// Defaults to `false`.
    ///
    /// Setting this feature to true can help with applications that distribute
    /// work evenly across many threads, such as thread pools. In such
    /// cases it can be difficult to get an overview of where the application
    /// as a whole spent most of its time, because work done in the same
    /// span may be split up across many threads.
    pub fn with_threads_collapsed(mut self, enabled: bool) -> Self {
        self.config.threads_collapsed = enabled;
        self
    }
}

impl<W> FlushGuard<W>
where
    W: Write + 'static,
{
    /// Flush the internal writer of the `FlameSubscriber`, ensuring that all
    /// intermediately buffered contents reach their destination.
    pub fn flush(&self) -> Result<(), Error> {
        let mut guard = match self.out.lock() {
            Ok(guard) => guard,
            Err(e) => {
                if !std::thread::panicking() {
                    panic!("{}", e);
                } else {
                    return Ok(());
                }
            }
        };

        guard.flush().map_err(Kind::FlushFile).map_err(Error)
    }
}

impl<W> Drop for FlushGuard<W>
where
    W: Write + 'static,
{
    fn drop(&mut self) {
        match self.flush() {
            Ok(_) => (),
            Err(e) => e.report(),
        }
    }
}

impl<S> FlameSubscriber<S, BufWriter<File>>
where
    S: Collect + for<'span> LookupSpan<'span>,
{
    /// Constructs a `FlameSubscriber` that outputs to a `BufWriter` to the given path, and a
    /// `FlushGuard` to ensure the writer is flushed.
    pub fn with_file(path: impl AsRef<Path>) -> Result<(Self, FlushGuard<BufWriter<File>>), Error> {
        let path = path.as_ref();
        let file = File::create(path)
            .map_err(|source| Kind::CreateFile {
                path: path.into(),
                source,
            })
            .map_err(Error)?;
        let writer = BufWriter::new(file);
        let layer = Self::new(writer);
        let guard = layer.flush_on_drop();
        Ok((layer, guard))
    }
}

impl<S, W> Subscribe<S> for FlameSubscriber<S, W>
where
    S: Collect + for<'span> LookupSpan<'span>,
    W: Write + 'static,
{
    fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
        let samples = self.time_since_last_event();

        let first = ctx.span(id).expect("expected: span id exists in registry");

        if !self.config.empty_samples && first.from_root().count() == 0 {
            return;
        }

        let parents = first.from_root();

        let mut stack = String::new();

        if !self.config.threads_collapsed {
            THREAD_NAME.with(|name| stack += name.as_str());
        } else {
            stack += "all-threads";
        }

        for parent in parents {
            stack += "; ";
            write(&mut stack, parent).expect("expected: write to String never fails");
        }

        write!(&mut stack, " {}", samples.as_nanos())
            .expect("expected: write to String never fails");

        let _ = writeln!(*self.out.lock().unwrap(), "{}", stack);
    }

    fn on_exit(&self, id: &span::Id, ctx: Context<'_, S>) {
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

        let samples = self.time_since_last_event();
        let first = expect!(ctx.span(&id), "expected: span id exists in registry");
        let parents = first.from_root();

        let mut stack = String::new();
        if !self.config.threads_collapsed {
            THREAD_NAME.with(|name| stack += name.as_str());
        } else {
            stack += "all-threads";
        }
        stack += "; ";

        for parent in parents {
            expect!(
                write(&mut stack, parent),
                "expected: write to String never fails"
            );
            stack += "; ";
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

impl<S, W> FlameSubscriber<S, W>
where
    S: Collect + for<'span> LookupSpan<'span>,
    W: Write + 'static,
{
    fn time_since_last_event(&self) -> Duration {
        let now = Instant::now();

        let prev = LAST_EVENT.with(|e| {
            let prev = e.get();
            e.set(now);
            prev
        });

        now - prev
    }
}

fn write<S>(dest: &mut String, span: SpanRef<'_, S>) -> fmt::Result
where
    S: Collect + for<'span> LookupSpan<'span>,
{
    if let Some(module_path) = span.metadata().module_path() {
        write!(dest, "{}::", module_path)?;
    }

    write!(dest, "{}", span.name())?;

    if let Some(file) = span.metadata().file() {
        write!(dest, ":{}", file)?;
    }

    if let Some(line) = span.metadata().line() {
        write!(dest, ":{}", line)?;
    }

    Ok(())
}
