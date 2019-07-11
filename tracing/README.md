# tracing

Application-level tracing for Rust.

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][travis-badge]][travis-url]
[![Gitter chat][gitter-badge]][gitter-url]

[Documentation][docs-url] |
[Chat][gitter-url]

[crates-badge]: https://img.shields.io/crates/v/tracing.svg
[crates-url]: https://crates.io/crates/tracing
[docs-badge]: https://docs.rs/tracing/badge.svg
[docs-url]: https://docs.rs/tracing
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[travis-badge]: https://travis-ci.org/tokio-rs/tracing.svg?branch=master
[travis-url]: https://travis-ci.org/tokio-rs/tracing/branches
[gitter-badge]: https://img.shields.io/gitter/room/tokio-rs/tracing.svg
[gitter-url]: https://gitter.im/tokio-rs/tracing

## Overview

`tracing` is a framework for instrumenting Rust programs to collect
structured, event-based diagnostic information.

In asynchronous systems like Tokio, interpreting traditional log messages can
often be quite challenging. Since individual tasks are multiplexed on the same
thread, associated events and log lines are intermixed making it difficult to
trace the logic flow. `tracing` expands upon logging-style diagnostics by
allowing libraries and applications to record structured events with additional
information about *temporality* and *causality* — unlike a log message, a span
in `tracing` has a beginning and end time, may be entered and exited by the
flow of execution, and may exist within a nested tree of similar spans. In
addition, `tracing` spans are *structured*, with the ability to record typed
data as well as textual messages.

The `tracing` crate provides the APIs necessary for instrumenting libraries
and applications to emit trace data.

## Usage

First, add this to your `Cargo.toml`:

```toml
[dependencies]
tracing = "0.1.3"
```

This crate provides macros for creating `Span`s and `Event`s, which represent
periods of time and momentary events within the execution of a program,
respectively.

As a rule of thumb, _spans_ should be used to represent discrete units of work
(e.g., a given request's lifetime in a server) or periods of time spent in a
given context (e.g., time spent interacting with an instance of an external
system, such as a database). In contrast, _events_ should be used to represent
points in time within a span — a request returned with a given status code,
_n_ new items were taken from a queue, and so on.

`Span`s are constructed using the `span!` macro, and then _entered_
to indicate that some code takes place within the context of that `Span`:

```rust
use tracing::{span, Level};

// Construct a new span named "my span".
let mut span = span!(Level::INFO, "my span");
span.in_scope(|| {
    // Any trace events in this closure or code called by it will occur within
    // the span.
});
// Dropping the span will close it, indicating that it has ended.
```

The `Event` type represent an event that occurs instantaneously, and is
essentially a `Span` that cannot be entered. They are created using the `event!`
macro:

```rust
use tracing::{event, Level};

event!(Level::INFO, "something has happened!");
```

Users of the [`log`] crate should note that `tracing` exposes a set of macros for
creating `Event`s (`trace!`, `debug!`, `info!`, `warn!`, and `error!`) which may
be invoked with the same syntax as the similarly-named macros from the `log`
crate. Often, the process of converting a project to use `tracing` can begin
with a simple drop-in replacement.

Let's consider the `log` crate's yak-shaving
[example](https://docs.rs/log/0.4.6/log/index.html#examples), modified to use
`tracing`:

```rust
// Import `tracing`'s macros rather than `log`'s
use tracing::{span, info, warn, Level};

// unchanged from here forward
pub fn shave_the_yak(yak: &mut Yak) {
    info!(target: "yak_events", "Commencing yak shaving for {:?}", yak);

    loop {
        match find_a_razor() {
            Ok(razor) => {
                info!("Razor located: {}", razor);
                yak.shave(razor);
                break;
            }
            Err(err) => {
                warn!("Unable to locate a razor: {}, retrying", err);
            }
        }
    }
}

// Dummy impls to make the example compile
#[derive(Debug)] pub struct Yak(String);
impl Yak { fn shave(&mut self, _: u32) {} }
fn find_a_razor() -> Result<u32, u32> { Ok(1) }
```

We can change it even further to better utilize features in tracing.

```rust
use tracing::{span, info, warn, Level};

pub fn shave_the_yak(yak: &mut Yak) {
    // create and enter a span to represent the scope
    let span = span!(Level::TRACE, "shave_the_yak", ?yak);
    let _enter = span.enter();

    // Since the span is annotated with the yak, it is part of the context
    // for everything happening inside the span. Therefore, we don't need
    // to add it to the message for this event, as the `log` crate does.
    info!(target: "yak_events", "Commencing yak shaving");
    loop {
        match find_a_razor() {
            Ok(razor) => {
                // We can add the razor as a field rather than formatting it
                // as part of the message, allowing subscribers to consume it
                // in a more structured manner:
                info!({ %razor }, "Razor located");
                yak.shave(razor);
                break;
            }
            Err(err) => {
                // However, we can also create events with formatted messages,
                // just as we would for log records.
                warn!("Unable to locate a razor: {}, retrying", err);
            }
        }
    }
}

#[derive(Debug)] pub struct Yak(String);
impl Yak { fn shave(&mut self, _: u32) {} }
fn find_a_razor() -> Result<u32, u32> { Ok(1) }
```

You can find further examples showing how to use this crate in the examples
directory.

### In libraries

Libraries should link only to the `tracing` crate, and use the provided
macros to record whatever information will be useful to downstream consumers.

### In executables

In order to record trace events, executables have to use a `Subscriber`
implementation compatible with `tracing`. A `Subscriber` implements a way of
collecting trace data, such as by logging it to standard output.

There currently aren't too many subscribers to choose from. The best one to use right now
is probably [`tracing-fmt`], which logs to the terminal. It is not currently
published to crates.io so you will need to add [`tracing-fmt`] as a git
dependency to use it.

The simplest way to use a subscriber is to call the `set_global_default` function:

```rust
use tracing::{span::{Id, Attributes, Record}, Metadata};

pub struct FooSubscriber;

impl tracing::Subscriber for FooSubscriber {
  fn new_span(&self, _: &Attributes) -> Id { Id::from_u64(0) }
  fn record(&self, _: &Id, _: &Record) {}
  fn event(&self, _: &tracing::Event) {}
  fn record_follows_from(&self, _: &Id, _: &Id) {}
  fn enabled(&self, _: &Metadata) -> bool { false }
  fn enter(&self, _: &Id) {}
  fn exit(&self, _: &Id) {}
}

impl FooSubscriber {
  fn new() -> Self { FooSubscriber }
}

let my_subscriber = FooSubscriber::new();

tracing::subscriber::set_global_default(my_subscriber)
    .expect("setting tracing default failed");
```

This subscriber will be used as the default in all threads for the remainder of the duration
of the program, similar to how loggers work in the `log` crate.

Note: Libraries should *NOT* call `set_global_default()`! That will cause conflicts when
executables try to set the default later.

In addition, you can locally override the default subscriber, using the `tokio` pattern
of executing code in a context. For example:

```rust
use tracing::{span::{Id, Attributes, Record}, Metadata};

pub struct FooSubscriber;

impl tracing::Subscriber for FooSubscriber {
  fn new_span(&self, _: &Attributes) -> Id { Id::from_u64(0) }
  fn record(&self, _: &Id, _: &Record) {}
  fn event(&self, _: &tracing::Event) {}
  fn record_follows_from(&self, _: &Id, _: &Id) {}
  fn enabled(&self, _: &Metadata) -> bool { false }
  fn enter(&self, _: &Id) {}
  fn exit(&self, _: &Id) {}
}

impl FooSubscriber {
  fn new() -> Self { FooSubscriber }
}

let my_subscriber = FooSubscriber::new();

tracing::subscriber::with_default(my_subscriber, || {
    // Any trace events generated in this closure or by functions it calls
    // will be collected by `my_subscriber`.
})
```

Any trace events generated outside the context of a subscriber will not be collected.

Once a subscriber has been set, instrumentation points may be added to the
executable using the `tracing` crate's macros.

In addition to `tracing` and `tracing-core`, the [`tokio-rs/tracing`] repository
contains several additional crates designed to be used with the `tracing` ecosystem.
This includes a collection of `Subscriber` implementations, as well as utility
and adapter crates to assist in writing `Subscriber`s and instrumenting
applications.

In particular, the following crates are likely to be of interest:

 - [`tracing-futures`] provides a compatibility layer with the `futures`
   crate, allowing spans to be attached to `Future`s, `Stream`s, and `Executor`s.
 - [`tracing-fmt`] provides a `Subscriber` implementation for
   logging formatted trace data to stdout, with similar filtering and
   formatting to the `env-logger` crate.
 - [`tracing-log`] provides a compatibility layer with the `log` crate,
   allowing log messages to be recorded as `tracing` `Event`s within the
   trace tree. This is useful when a project using `tracing` have
   dependencies which use `log`.
 - [`tracing-timing`] implements inter-event timing metrics on top of `tracing`.
   It provides a subscriber that records the time elapsed between pairs of
   `tracing` events and generates histograms.

**Note:** that some of the ecosystem crates are currently unreleased and
undergoing active development. They may be less stable than `tracing` and
`tracing-core`.


[`log`]: https://docs.rs/log/0.4.6/log/
[`tokio-rs/tracing`]: https://github.com/tokio-rs/tracing
[`tracing-futures`]: https://github.com/tokio-rs/tracing/tree/master/tracing-futures
[`tracing-fmt`]: https://github.com/tokio-rs/tracing/tree/master/tracing-fmt
[`tracing-log`]: https://github.com/tokio-rs/tracing/tree/master/tracing-log
[`tracing-timing`]: https://crates.io/crates/tracing-timing

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tokio by you, shall be licensed as MIT, without any additional
terms or conditions.
