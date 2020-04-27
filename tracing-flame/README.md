# tracing-flame

A Tracing [Layer][`FlameLayer`] for generating a folded stack trace for generating flamegraphs
and flamecharts with [`inferno`]

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![Documentation (master)][docs-master-badge]][docs-master-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]
![maintenance status][maint-badge]

[Documentation][docs-url] | [Chat][discord-url]

# Overview

[`tracing`] is a framework for instrumenting Rust programs to collect
scoped, structured, and async-aware diagnostics. `tracing-flame` provides helpers
for consuming `tracing` instrumentation that can later be visualized as a
flamegraph/flamechart. Flamegraphs/flamecharts are useful for identifying performance
issues bottlenecks in an application. For more details, see Brendan Gregg's [post]
on flamegraphs.

[post]: http://www.brendangregg.com/flamegraphs.html

## Usage

This crate is meant to be used in a two step process:

1. A textual representation of the spans that are entered and exited are
  captured with [`FlameLayer`].
2. Feed the textual representation into `inferno-flamegraph` to generate the
   flamegraph or flamechart.

*Note*: when using a buffered writer as the writer for a `FlameLayer`, it is necessary to
ensure that the buffer has been flushed before the data is passed into
[`inferno-flamegraph`]. For more details on how to flush the internal writer
of the `FlameLayer`, see the docs for [`FlushGuard`].

## Layer Setup

```rust
use std::{fs::File, io::BufWriter};
use tracing_flame::FlameLayer;
use tracing_subscriber::{registry::Registry, prelude::*, fmt};

fn setup_global_subscriber() -> impl Drop {
    let fmt_layer = fmt::Layer::default();

    let (flame_layer, _guard) = FlameLayer::with_file("./tracing.folded").unwrap();

    let subscriber = Registry::default()
        .with(fmt_layer)
        .with(flame_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");
    _guard
}

// your code here ..
```

As an alternative, you can provide _any_ type that implements `std::io::Write` to
`FlameLayer::new`.

## Generating the Image

To convert the textual representation of a flamegraph to a visual one, first install `inferno`:

```console
cargo install inferno
```

Then, pass the file created by `FlameLayer` into `inferno-flamegraph`:

```console
# flamegraph
cat tracing.folded | inferno-flamegraph > tracing-flamegraph.svg

#flamechart
cat tracing.folded | inferno-flamegraph --flamechart > tracing-flamechart.svg
```

## Differences between `flamegraph`s and `flamechart`s

By default, `inferno-flamegraph` creates flamegraphs. Flamegraphs operate by
that collapsing identical stack frames and sorting them on the frame's names.

This behavior is great for multithreaded programs and long-running programs
where the same frames occur _many_ times, for short durations, because it reduces
noise in the graph and gives the reader a better idea of the
overall time spent in each part of the application.

However, it is sometimes desirable to preserve the _exact_ ordering of events
as they were emitted by `tracing-flame`, so that it is clear when each
span is entered relative to others and get an accurate visual trace of
the execution of your program. This representation is best created with a
_flamechart_, which _does not_ sort or collapse identical stack frames.

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tracing by you, shall be licensed as MIT, without any additional
terms or conditions.

[`inferno`]: https://docs.rs/inferno
[`FlameLayer`]: https://docs.rs/tracing-flame/*/tracing_flame/struct.FlameLayer.html
[`FlushGuard`]: https://docs.rs/tracing-flame/*/tracing_flame/struct.FlushGuard.html
[`inferno-flamegraph`]: https://docs.rs/inferno/0.9.5/inferno/index.html#producing-a-flame-graph
[tracing]: https://github.com/tokio-rs/tracing/tree/master/tracing
[crates-badge]: https://img.shields.io/crates/v/tracing-flame.svg
[crates-url]: https://crates.io/crates/tracing-flame
[docs-badge]: https://docs.rs/tracing-flame/badge.svg
[docs-url]: https://docs.rs/tracing-flame/0.2.4
[docs-master-badge]: https://img.shields.io/badge/docs-master-blue
[docs-master-url]: https://tracing-rs.netlify.com/tracing_flame
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[actions-badge]: https://github.com/tokio-rs/tracing/workflows/CI/badge.svg
[actions-url]:https://github.com/tokio-rs/tracing/actions?query=workflow%3ACI
[discord-badge]: https://img.shields.io/discord/500028886025895936?logo=discord&label=discord&logoColor=white
[discord-url]: https://discord.gg/EeF3cQw
[maint-badge]: https://img.shields.io/badge/maintenance-experimental-blue.svg
