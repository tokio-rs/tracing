![Tracing — Structured, application-level diagnostics][splash]

[splash]: https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/splash.svg

# tracing-mock

Utilities for testing [`tracing`][tracing] and crates that uses it.

[![Documentation (master)][docs-master-badge]][docs-master-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[Documentation][docs-master-url] | [Chat][discord-url]

[docs-master-badge]: https://img.shields.io/badge/docs-master-blue
[docs-master-url]: https://tracing-rs.netlify.com/tracing_mock
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/tokio-rs/tracing/blob/master/tracing-mock/LICENSE
[actions-badge]: https://github.com/tokio-rs/tracing/workflows/CI/badge.svg
[actions-url]:https://github.com/tokio-rs/tracing/actions?query=workflow%3ACI
[discord-badge]: https://img.shields.io/discord/500028886025895936?logo=discord&label=discord&logoColor=white
[discord-url]: https://discord.gg/EeF3cQw

## Overview

[`tracing`] is a framework for instrumenting Rust programs to collect
structured, event-based diagnostic information. `tracing-mock` provides
tools for making assertions about what `tracing` diagnostics are emitted
by code under test.

*Compiler support: [requires `rustc` 1.63+][msrv]*

[msrv]: #supported-rust-versions

## Usage

`tracing-mock` crate provides a mock
[`Collector`](https://tracing-rs.netlify.app/tracing/#collectors)
that allows asserting on the order and contents of
[spans](https://tracing-rs.netlify.app/tracing/#spans) and
[events](https://tracing-rs.netlify.app/tracing/#events).

As `tracing-mock` isn't available on [crates.io](https://crates.io/)
yet, you must import it via git. When using `tracing-mock` with the
`tracing` `0.1` ecosystem, it is important that you also override the
source of any `tracing` crates that are transient dependencies. For
example, the `Cargo.toml` for your test crate could contain:

```toml
[dependencies]
lib-under-test = "1.0" # depends on `tracing`

[dev-dependencies]
tracing-mock = { git = "https://github.com/tokio-rs/tracing", branch = "v0.1.x", version = "0.1" }
tracing = { git = "https://github.com/tokio-rs/tracing", branch = "v0.1.x", version = "0.1" }

[patch.crates-io]
tracing = { git = "https://github.com/tokio-rs/tracing", branch = "v0.1.x" }
tracing-core = { git = "https://github.com/tokio-rs/tracing", branch = "v0.1.x" }
```

## Examples

The following examples are for the `master` branch. For examples that
will work with `tracing` from [crates.io], please check the
[v0.1.x](https://github.com/tokio-rs/tracing/tree/v0.1.x/tracing-mock)
branch.

Below is an example that checks that an event contains a message:

```rust
use tracing::collect::with_default;
use tracing_mock::{collector, expect};

fn yak_shaving() {
    tracing::info!("preparing to shave yaks");
}

let (collector, handle) = collector::mock()
    .event(expect::event().with_fields(expect::message("preparing to shave yaks")))
    .only()
    .run_with_handle();

with_default(collector, || {
    yak_shaving();
});

handle.assert_finished();

```

Below is a slightly more complex example. `tracing-mock` asserts that, in order:
- a span is created with a single field/value pair
- the span is entered
- an event is created with the field `number_of_yaks`, a corresponding
  value of 3, and the message "preparing to shave yaks", and nothing else
- an event is created with the field `all_yaks_shaved`, a corresponding value
  of `true`, and the message "yak shaving completed"
- the span is exited
- no further traces are received

```rust
use tracing::collect::with_default;
use tracing_mock::{collector, expect};

#[tracing::instrument]
fn yak_shaving(number_of_yaks: u32) {
    tracing::info!(number_of_yaks, "preparing to shave yaks");

    let number_shaved = number_of_yaks; // shave_all
    tracing::info!(
        all_yaks_shaved = number_shaved == number_of_yaks,
        "yak shaving completed."
    );
}

let yak_count: u32 = 3;
let span = expect::span().named("yak_shaving");

let (collector, handle) = collector::mock()
    .new_span(
        span.clone()
            .with_fields(expect::field("number_of_yaks").with_value(&yak_count).only()),
    )
    .enter(span.clone())
    .event(
        expect::event().with_fields(
            expect::field("number_of_yaks")
                .with_value(&yak_count)
                .and(expect::message("preparing to shave yaks"))
                .only(),
        ),
    )
    .event(
        expect::event().with_fields(
            expect::field("all_yaks_shaved")
                .with_value(&true)
                .and(expect::message("yak shaving completed."))
                .only(),
        ),
    )
    .exit(span.clone())
    .only()
    .run_with_handle();

with_default(collector, || {
    yak_shaving(yak_count);
});

handle.assert_finished();
```

## Supported Rust Versions

Tracing is built against the latest stable release. The minimum supported
version is 1.63. The current Tracing version is not guaranteed to build on Rust
versions earlier than the minimum supported version.

Tracing follows the same compiler support policies as the rest of the Tokio
project. The current stable Rust compiler and the three most recent minor
versions before it will always be supported. For example, if the current stable
compiler version is 1.69, the minimum supported version will not be increased
past 1.66, three minor versions prior. Increasing the minimum supported compiler
version is not considered a semver breaking change as long as doing so complies
with this policy.

## License

This project is licensed under the [MIT license][mit-url].

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tracing by you, shall be licensed as MIT, without any additional
terms or conditions.