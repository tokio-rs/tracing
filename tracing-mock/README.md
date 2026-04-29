![Tracing — Structured, application-level diagnostics][splash]

[splash]: https://raw.githubusercontent.com/tokio-rs/tracing/main/assets/splash.svg

# tracing-mock

Utilities for testing [`tracing`] and crates that uses it.

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![Documentation (v0.2.x)][docs-v0.2.x-badge]][docs-v0.2.x-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[Documentation][docs-v0.2.x-url] | [Chat][discord-url]

[crates-badge]: https://img.shields.io/crates/v/tracing-mock.svg
[crates-url]: https://crates.io/crates/tracing-mock/0.1.0-beta.3
[docs-badge]: https://docs.rs/tracing-mock/badge.svg
[docs-url]: https://docs.rs/tracing-mock/0.1.0-beta.3
[docs-v0.2.x-badge]: https://img.shields.io/badge/docs-v0.2.x-blue
[docs-v0.2.x-url]: https://tracing.rs/tracing_mock
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/tokio-rs/tracing/blog/main/tracing-mock/LICENSE
[actions-badge]: https://github.com/tokio-rs/tracing/workflows/CI/badge.svg
[actions-url]:https://github.com/tokio-rs/tracing/actions?query=workflow%3ACI
[discord-badge]: https://img.shields.io/discord/500028886025895936?logo=discord&label=discord&logoColor=white
[discord-url]: https://discord.gg/EeF3cQw

## Overview

[`tracing`] is a framework for instrumenting Rust programs to collect
structured, event-based diagnostic information. `tracing-mock` provides
tools for making assertions about what `tracing` diagnostics are emitted
by code under test.

*Compiler support: [requires `rustc` 1.65+][msrv]*

[msrv]: #supported-rust-versions
[`tracing`]: https://github.com/tokio-rs/tracing

## Usage

The `tracing-mock` crate provides a mock [`Subscriber`][tracing-subscriber] that
allows asserting on the order and contents of [spans][tracing-spans] and
[events][tracing-events].

To get started with `tracing-mock`, check the documentation in the
[`subscriber`][mock-subscriber-mod] module and [`MockSubscriber`] struct.

While `tracing-mock` is in beta, it is recommended that an exact version is
specified in the cargo manifest. Otherwise, `cargo update` will take the latest
beta version, which may contain breaking changes compared to previous betas.

To do so, add the following to `Cargo.toml`:

```toml
[dependencies]
tracing-mock = "= 0.1.0-beta.3"
```

[tracing-spans]: https://docs.rs/tracing/0.1/tracing/#spans
[tracing-events]: https://docs.rs/tracing/0.1/tracing/#events
[tracing-subscriber]: https://docs.rs/tracing/0.1/tracing/trait.Subscriber.html
[mock-subscriber-mod]: https://docs.rs/tracing-mock/0.1.0-beta.3/tracing_mock/subscriber/index.html
[`MockSubscriber`]: https://docs.rs/tracing-mock/0.1.0-beta.3/tracing_mock/subscriber/struct.MockSubscriber.html

## Examples

Below is an example that checks that an event contains a message:

```rust
use tracing::subscriber::with_default;
use tracing_mock::{expect, subscriber};

fn yak_shaving() {
    tracing::info!("preparing to shave yaks");
}

let (subscriber, handle) = subscriber::mock()
    .event(expect::event().with_fields(expect::msg("preparing to shave yaks")))
    .only()
    .run_with_handle();

with_default(subscriber, || {
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
use tracing::subscriber::with_default;
use tracing_mock::{expect, subscriber};

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

let (subscriber, handle) = subscriber::mock()
    .new_span(
        span.clone()
            .with_fields(expect::field("number_of_yaks").with_value(&yak_count).only()),
    )
    .enter(span.clone())
    .event(
        expect::event().with_fields(
            expect::field("number_of_yaks")
                .with_value(&yak_count)
                .and(expect::msg("preparing to shave yaks"))
                .only(),
        ),
    )
    .event(
        expect::event().with_fields(
            expect::field("all_yaks_shaved")
                .with_value(&true)
                .and(expect::msg("yak shaving completed."))
                .only(),
        ),
    )
    .exit(span.clone())
    .only()
    .run_with_handle();

with_default(subscriber, || {
    yak_shaving(yak_count);
});

handle.assert_finished();
```

## Supported Rust Versions

Tracing is built against the latest stable release. The minimum supported
version is 1.65. The current Tracing version is not guaranteed to build on Rust
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
