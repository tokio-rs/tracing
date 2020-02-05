# tracing-error

Utilities for instrumenting errors with [`tracing`].

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![Documentation (master)][docs-master-badge]][docs-master-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]
![maintenance status][maint-badge]

[Documentation (release)][docs-url] | [Documentation (master)][docs-master-url] | [Chat][discord-url]

[crates-badge]: https://img.shields.io/crates/v/tracing-error.svg
[crates-url]: https://crates.io/crates/tracing-error/0.1.0
[docs-badge]: https://docs.rs/tracing-error/badge.svg
[docs-url]: https://docs.rs/tracing-error/0.1.0/tracing_error
[docs-master-badge]: https://img.shields.io/badge/docs-master-blue
[docs-master-url]: https://tracing-rs.netlify.com/tracing_error
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[actions-badge]: https://github.com/tokio-rs/tracing/workflows/CI/badge.svg
[actions-url]:https://github.com/tokio-rs/tracing/actions?query=workflow%3ACI
[discord-badge]: https://img.shields.io/discord/500028886025895936?logo=discord&label=discord&logoColor=white
[discord-url]: https://discord.gg/EeF3cQw
[maint-badge]: https://img.shields.io/badge/maintenance-experimental-blue.svg

## Overview

[`tracing`] is a framework for instrumenting Rust programs to collect
scoped, structured, and async-aware diagnostics. This crate provides
integrations between [`tracing`] instrumentation and Rust error handling. It
enables enriching error types with diagnostic information from `tracing`
[span] contexts, formatting those contexts when errors are displayed, and
automatically generate `tracing` [events] when errors occur.

The crate provides the following:

* [`SpanTrace`], a captured trace of the current `tracing` [span] context

* [`ErrorLayer`], a [subscriber layer] which enables capturing `SpanTrace`s

**Note**: This crate is currently experimental.

*Compiler support: requires `rustc` 1.39+*

## Usage

Currently, `tracing-error` provides the [`SpanTrace`] type, which captures
the current `tracing` span context when it is constructed and allows it to
be displayed at a later time.

This crate does not _currently_ provide any actual error types implementing
`std::error::Error`. Instead, user-constructed errors or libraries
implementing error types may capture a [`SpanTrace`] and include it as part
of their error types.

For example:

```rust
use std::{fmt, error::Error};
use tracing_error::SpanTrace;

#[derive(Debug)]
pub struct MyError {
    context: SpanTrace,
    // ...
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ... format other parts of the error ...
        self.context.fmt(f)?;
        // ... format other error context information, cause chain, etc ...
    }
}

impl Error for MyError {}

impl MyError {
    pub fn new() -> Self {
        Self {
            context: SpanTrace::capture(),
            // ... other error information ...
        }
    }
}
```

In the future, this crate may also provide its own `Error` types as well,
for users who do not wish to use other error-handling libraries.
Applications that wish to use `tracing-error`-enabled errors should
construct an [`ErrorLayer`] and add it to their [`Subscriber`] in order to
enable capturing [`SpanTrace`]s. For example:

```rust
use tracing_error::ErrorLayer;
use tracing_subscriber::prelude::*;

fn main() {
    let subscriber = tracing_subscriber::Registry::default()
        // any number of other subscriber layers may be added before or
        // after the `ErrorLayer`...
        .with(ErrorLayer::default());
    // set the subscriber as the default for the application
    tracing::subscriber::set_global_default(subscriber);
}
```

[`SpanTrace`]: https://docs.rs/tracing-error/0.1.0/tracing_error/struct.SpanTrace.html
[`ErrorLayer`]: https://docs.rs/tracing-error/0.1.0/tracing_error/struct.ErrorLayer.html
[span]: https://docs.rs/tracing/latest/tracing/span/index.html
[event]: https://docs.rs/tracing/latest/tracing/struct.Event.html
[subscriber layer]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html
[`tracing`]: https://crates.io/tracing

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tracing by you, shall be licensed as MIT, without any additional
terms or conditions.
