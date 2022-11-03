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
[mit-url]: LICENSE
[actions-badge]: https://github.com/tokio-rs/tracing/workflows/CI/badge.svg
[actions-url]:https://github.com/tokio-rs/tracing/actions?query=workflow%3ACI
[discord-badge]: https://img.shields.io/discord/500028886025895936?logo=discord&label=discord&logoColor=white
[discord-url]: https://discord.gg/EeF3cQw

## Overview

[`tracing`] is a framework for instrumenting Rust programs to collect
structured, event-based diagnostic information. `tracing-mock` provides
mock `tracing` objects that are useful in testing `tracing`  itself 
and crates that use `tracing`.

*Compiler support: [requires `rustc` 1.49+][msrv]*

[msrv]: #supported-rust-versions

## Usage

`tracing-mock` crate provides a mock `Subscriber` that allows asserting on 
the order and contents of spans and events.

As `tracing-mock` isn't available on [crates.io](https://crates.io/)
yet, you must import it via git. It is important that you also override
the source of any `tracing` crates that are transient dependencies. For
example, the `Cargo.toml` for your test crate could contain:

```toml
[dependencies]
lib-under-test = 1.0 # depends on `tracing`

[dev-dependencies]
tracing-mock = { git = "https://github.com/tokio-rs/tracing", branch = "v0.1.x", version = "0.1" }
tracing = { git = "https://github.com/tokio-rs/tracing", branch = "v0.1.x", version = "0.1" }

[patch.crates-io]
tracing = { git = "https://github.com/tokio-rs/tracing", branch = "v0.1.x" }
```
## Examples

Below is a simple example of checking for an event with only a
message.

```rust
use tracing::subscriber::with_default;
use tracing_mock::{event, field, subscriber};

fn yak_shaving() {
    tracing::info!("preparing to shave yaks");
}

#[test]
fn traced_event() {
    let (subscriber, handle) = subscriber::mock()
        .event(event::mock().with_fields(field::msg("preparing to shave yaks")))
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        yak_shaving();
    });

    handle.assert_finished();
}
```

The following is a more complex example taking the full yak shaving example
from the `tracing` README and additionally instrumenting the called function
with a span.

```rust
use tracing::subscriber::with_default;
use tracing_mock::{event, field, span, subscriber};

#[tracing::instrument]
fn yak_shaving(number_of_yaks: u32) {
    tracing::info!(number_of_yaks, "preparing to shave yaks");

    let number_shaved = yak_shave::shave_all(number_of_yaks);
    tracing::info!(
        all_yaks_shaved = number_shaved == number_of_yaks,
        "yak shaving completed."
    );
}

// If this test gets updated, the `tracing-mock` README.md must be updated too.
#[test]
fn yak_shaving_traced() {
    let yak_count: u32 = 3;
    let span = span::mock().named("yak_shaving");

    let (subscriber, handle) = subscriber::mock()
        .new_span(
            span.clone()
                .with_field(field::mock("number_of_yaks").with_value(&yak_count).only()),
        )
        .enter(span.clone())
        .event(
            event::mock().with_fields(
                field::mock("number_of_yaks")
                    .with_value(&yak_count)
                    .and(field::msg("preparing to shave yaks"))
                    .only(),
            ),
        )
        .event(
            event::mock().with_fields(
                field::mock("all_yaks_shaved")
                    .with_value(&true)
                    .and(field::msg("yak shaving completed."))
                    .only(),
            ),
        )
        .exit(span.clone())
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        yak_shaving(yak_count);
    });

    handle.assert_finished();
}
```

The full code for both examples can be found in the [readme.rs](tests/readme.rs)
tests.

## Supported Rust Versions

Tracing is built against the latest stable release. The minimum supported
version is 1.49. The current Tracing version is not guaranteed to build on Rust
versions earlier than the minimum supported version.

Tracing follows the same compiler support policies as the rest of the Tokio
project. The current stable Rust compiler and the three most recent minor
versions before it will always be supported. For example, if the current stable
compiler version is 1.45, the minimum supported version will not be increased
past 1.42, three minor versions prior. Increasing the minimum supported compiler
version is not considered a semver breaking change as long as doing so complies
with this policy.

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tracing by you, shall be licensed as MIT, without any additional
terms or conditions.