![Tracing — Structured, application-level diagnostics][splash]

[splash]: https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/splash.svg

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![Documentation (master)][docs-master-badge]][docs-master-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/tracing.svg
[crates-url]: https://crates.io/crates/tracing
[docs-badge]: https://docs.rs/tracing/badge.svg
[docs-url]: https://docs.rs/tracing
[docs-master-badge]: https://img.shields.io/badge/docs-master-blue
[docs-master-url]: https://tracing-rs.netlify.com
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[actions-badge]: https://github.com/tokio-rs/tracing/workflows/CI/badge.svg
[actions-url]:https://github.com/tokio-rs/tracing/actions?query=workflow%3ACI
[discord-badge]: https://img.shields.io/discord/500028886025895936?logo=discord&label=discord&logoColor=white
[discord-url]: https://discord.gg/EeF3cQw

[Website](https://tokio.rs) |
[Chat](https://discord.gg/EeF3cQw) | [Documentation (master branch)](https://tracing-rs.netlify.com/)

## Overview

`tracing` is a framework for instrumenting Rust programs to collect
structured, event-based diagnostic information. `tracing` is maintained by the
Tokio project, but does _not_ require the `tokio` runtime to be used.

## Usage

### In Applications

In order to record trace events, executables have to use a collector
implementation compatible with `tracing`. A collector implements a way of
collecting trace data, such as by logging it to standard output.
[`tracing-subscriber`][tracing-subscriber-docs]'s [`fmt` module][fmt] provides
a collector for logging traces with reasonable defaults. Additionally,
`tracing-subscriber` is able to consume messages emitted by `log`-instrumented
libraries and modules.

To use `tracing-subscriber`, add the following to your `Cargo.toml`:

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = "0.2"
```

Then create and install a collector, for example using [`init()`]:

```rust
use tracing::info;
use tracing_subscriber;

fn main() {
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

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

Using `init()` calls [`set_global_default()`] so this collector will be used
as the default in all threads for the remainder of the duration of the
program, similar to how loggers work in the `log` crate.

[tracing-subscriber-docs]: https://docs.rs/tracing-subscriber/
[fmt]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/index.html
[`set_global_default`]: https://docs.rs/tracing/latest/tracing/subscriber/fn.set_global_default.html


For more control, a collector can be built in stages and not set globally,
but instead used to locally override the default collector. For example:

```rust
use tracing::{info, Level};
use tracing_subscriber;

fn main() {
    let collector = tracing_subscriber::fmt()
        // filter spans/events with level TRACE or higher.
        .with_max_level(Level::TRACE)
        // build but do not install the subscriber.
        .finish();

    tracing::collector::with_default(collector, || {
        info!("This will be logged to stdout");
    });
    info!("This will _not_ be logged to stdout");
}
```

Any trace events generated outside the context of a collector will not be collected.

This approach allows trace data to be collected by multiple collectors
within different contexts in the program. Note that the override only applies to the
currently executing thread; other threads will not see the change from with_default.

Once a collector has been set, instrumentation points may be added to the
executable using the `tracing` crate's macros.

[`tracing-subscriber`]: https://docs.rs/tracing-subscriber/
[fmt]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/index.html
[`init()`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/fn.init.html
[`set_global_default()`]: https://docs.rs/tracing/latest/tracing/subscriber/fn.set_global_default.html

### In Libraries

Libraries should only rely on the `tracing` crate and use the provided macros
and types to collect whatever information might be useful to downstream consumers.

```rust
use std::{error::Error, io};
use tracing::{debug, error, info, span, warn, Level};

// the `#[tracing::instrument]` attribute creates and enters a span
// every time the instrumented function is called. The span is named after the
// the function or method. Parameters passed to the function are recorded as fields.
#[tracing::instrument]
pub fn shave(yak: usize) -> Result<(), Box<dyn Error + 'static>> {
    // this creates an event at the DEBUG level with two fields:
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
            // In this instance, `yak` is the field being initialized.
            error!(yak, error = error.as_ref(), "failed to shave yak!");
        } else {
            yaks_shaved += 1;
        }
        debug!(yaks_shaved);
    }

    yaks_shaved
}
```

```toml
[dependencies]
tracing = "0.1"
```

Note: Libraries should *NOT* install a collector by using a method that calls
[`set_global_default()`], as this will cause conflicts when executables try to
set the default later.

### In Asynchronous Code

To trace `async fn`s, the preferred method is using the [`#[instrument]`][instrument] attribute:

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

The [`tracing-futures`] crate must be specified as a dependency to enable
`async` support.

Special handling is needed for the general case of code using
[`std::future::Future`][std-future] or blocks with `async`/`await`, as the
following example _will not_ work:

```rust
async {
    let _s = span.enter();
    // ...
}
```

The span guard `_s` will not exit until the future generated by the `async` block is complete.
Since futures and spans can be entered and exited _multiple_ times without them completing,
the span remains entered for as long as the future exists, rather than being entered only when
it is polled, leading to very confusing and incorrect output.
For more details, see [the documentation on closing spans][closing].

This problem can be solved using the [`Future::instrument`] combinator:

```rust
use tracing::Instrument;

let my_future = async {
    // ...
};

my_future
    .instrument(tracing::info_span!("my_future"))
    .await
```

`Future::instrument` attaches a span to the future, ensuring that the span's lifetime
is as long as the future's.

Under the hood, the [`#[instrument]`][instrument] macro performs same the explicit span
attachment that `Future::instrument` does.

[std-future]: https://doc.rust-lang.org/stable/std/future/trait.Future.html
[`tracing-futures`]: https://docs.rs/tracing-futures
[closing]: https://docs.rs/tracing/latest/tracing/span/index.html#closing-spans
[`Future::instrument`]: https://docs.rs/tracing/latest/tracing/trait.Instrument.html#method.instrument
[instrument]: https://docs.rs/tracing/0.1.11/tracing/attr.instrument.html

## Supported Rust Versions

Tracing is built against the latest stable release. The minimum supported
version is 1.42. The current Tracing version is not guaranteed to build on Rust
versions earlier than the minimum supported version.

Tracing follows the same compiler support policies as the rest of the Tokio
project. The current stable Rust compiler and the three most recent minor
versions before it will always be supported. For example, if the current stable
compiler version is 1.45, the minimum supported version will not be increased
past 1.42, three minor versions prior. Increasing the minimum supported compiler
version is not considered a semver breaking change as long as doing so complies
with this policy.

## Getting Help

First, see if the answer to your question can be found in the API documentation.
If the answer is not there, there is an active community in
the [Tracing Discord channel][chat]. We would be happy to try to answer your
question. Last, if that doesn't work, try opening an [issue] with the question.

[chat]: https://discord.gg/EeF3cQw
[issue]: https://github.com/tokio-rs/tracing/issues/new

## Contributing

:balloon: Thanks for your help improving the project! We are so happy to have
you! We have a [contributing guide][guide] to help you get involved in the Tracing
project.

[guide]: CONTRIBUTING.md

## Project layout

The [`tracing`] crate contains the primary _instrumentation_ API, used for
instrumenting libraries and applications to emit trace data. The [`tracing-core`]
crate contains the _core_ API primitives on which the rest of `tracing` is
instrumented. Authors of trace subscribers may depend on `tracing-core`, which
guarantees a higher level of stability.

Additionally, this repository contains several compatibility and utility
libraries built on top of `tracing`. Some of these crates are in a pre-release
state, and are less stable than the `tracing` and `tracing-core` crates.

The crates included as part of Tracing are:

* [`tracing-futures`]: Utilities for instrumenting `futures`.
  ([crates.io][fut-crates]|[docs][fut-docs])

* [`tracing-macros`]: Experimental macros for emitting trace events (unstable).

* [`tracing-attributes`]: Procedural macro attributes for automatically
    instrumenting functions. ([crates.io][attr-crates]|[docs][attr-docs])

* [`tracing-log`]: Compatibility with the `log` crate (unstable).

* [`tracing-opentelemetry`]: Provides a layer that connects spans from multiple
  systems into a trace and emits them to [OpenTelemetry]-compatible distributed
  tracing systems for processing and visualization.
  ([crates.io][otel-crates]|[docs][otel-docs])

* [`tracing-serde`]: A compatibility layer for serializing trace data with
    `serde` (unstable).

* [`tracing-subscriber`]: Collector implementations, and utilities for
  implementing and composing `Collector`s.
  ([crates.io][sub-crates]|[docs][sub-docs])

* [`tracing-tower`]: Compatibility with the `tower` ecosystem (unstable).

* [`tracing-appender`]: Utilities for outputting tracing data, including a file appender
   and non-blocking writer. ([crates.io][app-crates]|[docs][app-docs])

* [`tracing-error`]: Provides `SpanTrace`, a type for instrumenting errors with
  tracing spans

* [`tracing-flame`]; Provides a layer for generating flame graphs based on
  tracing span entry / exit events.

* [`tracing-journald`]: Provides a layer for recording events to the
  Linux `journald` service, preserving structured data.

[`tracing`]: tracing
[`tracing-core`]: tracing-core
[`tracing-futures`]: tracing-futures
[`tracing-macros`]: tracing-macros
[`tracing-attributes`]: tracing-attributes
[`tracing-log`]: tracing-log
[`tracing-opentelemetry`]: tracing-opentelemetry
[`tracing-serde`]: tracing-serde
[`tracing-subscriber`]: tracing-subscriber
[`tracing-tower`]: tracing-tower
[`tracing-appender`]: tracing-appender
[`tracing-error`]: tracing-error
[`tracing-flame`]: tracing-flame
[`tracing-journald`]: tracing-journald

[fut-crates]: https://crates.io/crates/tracing-futures
[fut-docs]: https://docs.rs/tracing-futures

[attr-crates]: https://crates.io/crates/tracing-attributes
[attr-docs]: https://docs.rs/tracing-attributes

[sub-crates]: https://crates.io/crates/tracing-subscriber
[sub-docs]: https://docs.rs/tracing-subscriber

[otel-crates]: https://crates.io/crates/tracing-opentelemetry
[otel-docs]: https://docs.rs/tracing-opentelemetry
[OpenTelemetry]: https://opentelemetry.io/

[app-crates]: https://crates.io/crates/tracing-appender
[app-docs]: https://docs.rs/tracing-appender

## Related Crates

In addition to this repository, here are also several third-party crates which
are not maintained by the `tokio` project. These include:

- [`tracing-timing`] implements inter-event timing metrics on top of `tracing`.
  It provides a subscriber that records the time elapsed between pairs of
  `tracing` events and generates histograms.
- [`tracing-honeycomb`] Provides a layer that reports traces spanning multiple machines to [honeycomb.io]. Backed by [`tracing-distributed`].
- [`tracing-distributed`] Provides a generic implementation of a layer that reports traces spanning multiple machines to some backend.
- [`tracing-actix-web`] provides `tracing` integration for the `actix-web` web framework.
- [`tracing-actix`] provides `tracing` integration for the `actix` actor
  framework.
- [`tracing-gelf`] implements a subscriber for exporting traces in Greylog
  GELF format.
- [`tracing-coz`] provides integration with the [coz] causal profiler
  (Linux-only).
- [`tracing-bunyan-formatter`] provides a layer implementation that reports events and spans in [bunyan] format, enriched with timing information.
- [`tide-tracing`] provides a [tide] middleware to trace all incoming requests and responses.
- [`color-spantrace`] provides a formatter for rendering span traces in the
  style of `color-backtrace`
- [`color-eyre`] provides customized panic and eyre report handlers for
  `eyre::Report` for capturing span traces and backtraces with new errors and
  pretty printing them.
- [`spandoc`] provides a proc macro for constructing spans from doc comments
  _inside_ of functions.
- [`tracing-wasm`] provides a `Collector`/`Subscriber` implementation that reports
  events and spans via browser `console.log` and [User Timing API (`window.performance`)].
- [`test-env-log`] takes care of initializing `tracing` for tests, based on
  environment variables with an `env_logger` compatible syntax.
- [`tracing-unwrap`] provides convenience methods to report failed unwraps on `Result` or `Option` types to a `Collector`.
- [`diesel-tracing`] provides integration with [`diesel`] database connections.
- [`tracing-tracy`] provides a way to collect [Tracy] profiles in instrumented
  applications.
- [`tracing-elastic-apm`] provides a layer for reporting traces to [Elastic APM].

(if you're the maintainer of a `tracing` ecosystem crate not in this list,
please let us know!)

[`tracing-timing`]: https://crates.io/crates/tracing-timing
[`tracing-honeycomb`]: https://crates.io/crates/tracing-honeycomb
[`tracing-distributed`]: https://crates.io/crates/tracing-distributed
[honeycomb.io]: https://www.honeycomb.io/
[`tracing-actix`]: https://crates.io/crates/tracing-actix
[`tracing-actix-web`]: https://crates.io/crates/tracing-actix-web
[`tracing-gelf`]: https://crates.io/crates/tracing-gelf
[`tracing-coz`]: https://crates.io/crates/tracing-coz
[coz]: https://github.com/plasma-umass/coz
[`tracing-bunyan-formatter`]: https://crates.io/crates/tracing-bunyan-formatter
[`tide-tracing`]: https://crates.io/crates/tide-tracing
[tide]: https://crates.io/crates/tide
[bunyan]: https://github.com/trentm/node-bunyan
[`color-spantrace`]: https://docs.rs/color-spantrace
[`color-eyre`]: https://docs.rs/color-eyre
[`spandoc`]: https://docs.rs/spandoc
[`tracing-wasm`]: https://docs.rs/tracing-wasm
[`test-env-log`]: https://crates.io/crates/test-env-log
[User Timing API (`window.performance`)]: https://developer.mozilla.org/en-US/docs/Web/API/User_Timing_API
[`tracing-unwrap`]: https://docs.rs/tracing-unwrap
[`diesel`]: https://crates.io/crates/diesel
[`diesel-tracing`]: https://crates.io/crates/diesel-tracing
[`tracing-tracy`]: https://crates.io/crates/tracing-tracy
[Tracy]: https://github.com/wolfpld/tracy
[`tracing-elastic-apm`]: https://crates.io/crates/tracing-elastic-apm
[Elastic APM]: https://www.elastic.co/apm

**Note:** that some of the ecosystem crates are currently unreleased and
undergoing active development. They may be less stable than `tracing` and
`tracing-core`.

## External Resources

This is a list of links to blog posts, conference talks, and tutorials about
Tracing.

#### Blog Posts

* [Diagnostics with Tracing][tokio-blog-2019-08] on the Tokio blog, August 2019
* [Production-Grade Logging in Rust Applications][production-logging-2020], November 2020

[tokio-blog-2019-08]: https://tokio.rs/blog/2019-08-tracing/

#### Talks

* [Bay Area Rust Meetup talk and Q&A][bay-rust-2018-03], March 2018
* [RustConf 2019 talk][rust-conf-2019-08-video] and [slides][rust-conf-2019-08-slides], August 2019
* [Are we observable yet? @ RustyDays talk][rusty-days-2020-08-video] and [slides][rusty-days-2020-08-slides], August 2020

[bay-rust-2018-03]: https://www.youtube.com/watch?v=j_kXRg3zlec
[rust-conf-2019-08-video]: https://www.youtube.com/watch?v=JjItsfqFIdo
[rust-conf-2019-08-slides]: https://www.elizas.website/slides/rustconf-8-2019.pdf
[rusty-days-2020-08-video]: https://youtu.be/HtKnLiFwHJM
[rusty-days-2020-08-slides]: https://docs.google.com/presentation/d/1zrxJs7fJgQ29bKfnAll1bYTo9cYZxsCZUwDDtyp5Fak/edit?usp=sharing
[production-logging-2020]: https://medium.com/better-programming/production-grade-logging-in-rust-applications-2c7fffd108a6

Help us expand this list! If you've written or spoken about Tracing, or
know of resources that aren't listed, please open a pull request adding them.

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tracing by you, shall be licensed as MIT, without any additional
terms or conditions.
