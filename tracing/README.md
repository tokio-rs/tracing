# tracing

Application-level tracing for Rust.

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![Documentation (master)][docs-master-badge]][docs-master-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[Documentation][docs-url] | [Chat][discord-url]

[crates-badge]: https://img.shields.io/crates/v/tracing.svg
[crates-url]: https://crates.io/crates/tracing/0.1.11
[docs-badge]: https://docs.rs/tracing/badge.svg
[docs-url]: https://docs.rs/tracing/0.1.11
[docs-master-badge]: https://img.shields.io/badge/docs-master-blue
[docs-master-url]: https://tracing-rs.netlify.com/tracing
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[actions-badge]: https://github.com/tokio-rs/tracing/workflows/CI/badge.svg
[actions-url]:https://github.com/tokio-rs/tracing/actions?query=workflow%3ACI
[discord-badge]: https://img.shields.io/discord/500028886025895936?logo=discord&label=discord&logoColor=white
[discord-url]: https://discord.gg/EeF3cQw


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

*Compiler support: requires `rustc` 1.39+*

## Usage

(The below example is borrowed from the `log` crate's yak-shaving
[example](https://docs.rs/log/0.4.10/log/index.html#examples), modified to
idiomatic `tracing`)

In `Cargo.toml`:

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = "0.2.0-alpha.2"
```

In `src/main.rs`:

```rust
mod yak_shave;

use std::{error::Error, io};
use tracing::{debug, error, info, span, warn, Level};
use tracing_subscriber::FmtSubscriber;
use yak_shave;

fn main() {
    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than DEBUG (e.g, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::DEBUG)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting defualt subscriber failed");

    let number_of_yaks = 3;
    // this creates a new event, outside of any spans.
    info!(number_of_yaks, "preparing to shave yaks");

    let number_shaved = yak_shave::shave_all(number_of_yaks);
    info!(
        all_yaks_shaved = number_shaved == number_of_yaks,
        "yak shaving completed."
    );
}
```

In `src/yak_shave.rs`:

```rust
use std::{error::Error, io};
use tracing::{debug, error, info, span, warn, Level};

// the `#[tracing::instrument]` attribute creates a span with the name "shave"
// and a field (a key/value pair) whose key is "yak" and the value is the value of `yak`.
#[tracing::instrument]
pub fn shave(yak: usize) -> Result<(), Box<dyn Error + 'static>> {
    // this creates an event at the DEBUG log level with two fields:
    // - `excitement`, with the key "excitement" and the value "yay!"
    // - `message`, with the key "message" and the value "hello! I'm gonna shave a yak."
    //
    // unlike other fields, `message`'s shorthand initialization is just the string itself.
    debug!(excitement = "yay!", "hello! I'm gonna shave a yak.");
    if yak == 3 {
        warn!("could not locate yak!");
        // note that this is intended to demonstrate `tracing`'s features, not idiomatic
        // error handling! in a library or application, you should consider returning
        // a dedicated `YakError`. libraries like snafu or thiserror make this easy.
        return Err(io::Error::new(io::ErrorKind::Other, "shaving yak failed!").into());
    } else {
        debug!("yak shaved successfully");
    }
    Ok(())
}

pub fn shave_all(yaks: usize) -> usize {
    // Constructs a new span named "shaving_yaks" at the TRACE level,
    // and a field whose key is "yaks". This is equivalent to writing:
    //
    // let span = span!(Level::TRACE, "shaving_yaks", yaks = yaks);
    //
    // local variables (`yaks`) can be used as field values
    // without an assignment, similar to struct initializers.
    let span = span!(Level::TRACE, "shaving_yaks", yaks);
    let _enter = span.enter();

    info!("shaving yaks");

    let mut yaks_shaved = 0;
    for yak in 1..=yaks {
        let res = shave(yak);
        debug!(yak, shaved = res.is_ok());

        if let Err(ref error) = res {
            // Like spans, events can also use the field initialization shorthand.
            // In this instance, `yak` is the field being initalized.
            error!(yak, error = error.as_ref(), "failed to shave yak!");
        } else {
            yaks_shaved += 1;
        }
        debug!(yaks_shaved);
    }

    yaks_shaved
}
```

### In Applications

In order to record trace events, executables have to use a `Subscriber`
implementation compatible with `tracing`. A `Subscriber` implements a way of
collecting trace data, such as by logging it to standard output. [`tracing_subscriber`](https://docs.rs/tracing-subscriber/)'s
[`fmt` module](https://docs.rs/tracing-subscriber/0.2.0-alpha.2/tracing_subscriber/fmt/index.html) provides reasonable defaults.
Additionally, `tracing-subscriber` is able to consume messages emitted by `log`-instrumented libraries and modules.

The simplest way to use a subscriber is to call the `set_global_default` function:

```rust
use tracing_subscruber::FmtSubscriber;

fn main() {
    let subscriber = tracing_subscruber::FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // builds the subscriber.
        .finish();   

    tracing::subscriber::set_global_default(subscriber)
      .expect("setting tracing default failed");
}
```

This subscriber will be used as the default in all threads for the remainder of the duration
of the program, similar to how loggers work in the `log` crate.

In addition, you can locally override the default subscriber. For example:

```rust
use tracing_subscruber::FmtSubscriber;
use tracing::info;

fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // builds the subscriber.
        .finish();   

    tracing::subscriber::with_default(subscriber, || {
        info!("This will be logged to stdout");
    });
    info!("This will _not_ be logged to stdout");
}
```

Any trace events generated outside the context of a subscriber will not be collected.

Once a subscriber has been set, instrumentation points may be added to the
executable using the `tracing` crate's macros.

### In Libraries

Libraries should only rely on the `tracing` crate and use the provided macros
and types to collect whatever information might be useful to downstream consumers.
The `shave` and `shave_all` functions above is a good example of library-like usage.

Note: Libraries should *NOT* call `set_global_default()`, as this will cause conflicts when
executables try to set the default later.

### In Asynchronous Code

If you are instrumenting code that make use of
[`std::future::Future`](https://doc.rust-lang.org/stable/std/future/trait.Future.html)
or async/await, be sure to use the
[`tracing-futures`](https://docs.rs/tracing-futures) crate. This is needed
because the following example _will not_ work:

```rust
async {
    let _s = span.enter();
    // ...
}
```

The span `_s` will be dropped at the end of the `async` block, _not_ when the
future created by the async block is complete. In practice, this means that
the span does not live long enough to instrument the future for its entire
lifetime.

There are two ways to instrument asynchronous code. The first is through the
[`Future::instrument`](https://docs.rs/tracing-futures/0.2.0/tracing_futures/trait.Instrument.html#method.instrument) combinator:

```rust
use tracing_futures::Instrument;

let my_future = async {
    // ...
};

my_future
    .instrument(tracing::info_span!("my_future"))
    .await
```

`Future::instrument` attaches a span to the future, ensuring that the span's lifetime 
is as long as the future's.

The second, and preferred, option is through the
[`#[instrument]`](https://docs.rs/tracing/0.1.11/tracing/attr.instrument.html)
attribute:

```rust
use tracing::{info, instrument};
use tokio::{io::AsyncWriteExt, net::TcpStream};
use std::io;

#[instrument]
async fn write(stream: &mut TcpStream) -> io::Result<usize> {
    let result = stream.write(b"hello world\n").await;
    info!("wrote to stream; success={:?}", result.is_ok());
    result
}
```

Under the hood, the `#[instrument]` macro performs same the explicit span
attachment that `Future::instrument` does.

### Concepts

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

The [`#[instrument]`](https://docs.rs/tracing/0.1.11/tracing/attr.instrument.html) attribute macro
can reduce some of this boilerplate:

```rust
use tracing::{instrument};

#[instrument]
pub fn my_function(my_arg: usize) {
    // This event will be recorded inside a span named `my_function` with the
    // field `my_arg`.
    tracing::info!("inside my_function!");
    // ...
}
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

### Ecosystem

In addition to `tracing` and `tracing-core`, the [`tokio-rs/tracing`] repository
contains several additional crates designed to be used with the `tracing` ecosystem.
This includes a collection of `Subscriber` implementations, as well as utility
and adapter crates to assist in writing `Subscriber`s and instrumenting
applications.

In particular, the following crates are likely to be of interest:

 - [`tracing-futures`] provides a compatibility layer with the `futures`
   crate, allowing spans to be attached to `Future`s, `Stream`s, and `Executor`s.
 - [`tracing-subscriber`] provides `Subscriber` implementations and
   utilities for working with `Subscriber`s. This includes a [`FmtSubscriber`]
   `FmtSubscriber` for logging formatted trace data to stdout, with similar
   filtering and formatting to the [`env_logger`] crate.
 - [`tracing-log`] provides a compatibility layer with the [`log`] crate,
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
[`tracing-subscriber`]: https://github.com/tokio-rs/tracing/tree/master/tracing-subscriber
[`tracing-log`]: https://github.com/tokio-rs/tracing/tree/master/tracing-log
[`tracing-timing`]: https://crates.io/crates/tracing-timing
[`env_logger`]: https://crates.io/crates/env_logger
[`FmtSubscriber`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/struct.Subscriber.html
[`examples`]: https://github.com/tokio-rs/tracing/tree/master/examples

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tokio by you, shall be licensed as MIT, without any additional
terms or conditions.
