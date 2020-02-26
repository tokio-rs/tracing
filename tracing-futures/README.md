# tracing-futures

Utilities for instrumenting futures-based code with [`tracing`].

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![Documentation (master)][docs-master-badge]][docs-master-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]
![maintenance status][maint-badge]

[Documentation][docs-url] | [Chat][discord-url]

[crates-badge]: https://img.shields.io/crates/v/tracing-futures.svg
[crates-url]: https://crates.io/crates/tracing-futures/0.2.3
[docs-badge]: https://docs.rs/tracing-futures/badge.svg
[docs-url]: https://docs.rs/tracing-futures/0.2.3/tracing_futures
[docs-master-badge]: https://img.shields.io/badge/docs-master-blue
[docs-master-url]: https://tracing-rs.netlify.com/tracing_futures
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[actions-badge]: https://github.com/tokio-rs/tracing/workflows/CI/badge.svg
[actions-url]:https://github.com/tokio-rs/tracing/actions?query=workflow%3ACI
[discord-badge]: https://img.shields.io/discord/500028886025895936?logo=discord&label=discord&logoColor=white
[discord-url]: https://discord.gg/EeF3cQw
[maint-badge]: https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg

## Overview

[`tracing`] is a framework for instrumenting Rust programs to collect
structured, event-based diagnostic information. This crate provides utilities
for using `tracing` to instrument asynchronous code written using futures and
async/await.

The crate provides the following traits:

* [`Instrument`] allows a `tracing` [span] to be attached to a future, sink,
  stream, or executor.

* [`WithSubscriber`] allows a `tracing` [`Subscriber`] to be attached to a
  future, sink, stream, or executor.

[`Instrument`]: https://docs.rs/tracing-futures/0.2.3/tracing_futures/trait.Instrument.html
[`WithSubscriber`]: https://docs.rs/tracing-futures/0.2.3/tracing_futures/trait.WithSubscriber.html
[span]: https://docs.rs/tracing/latest/tracing/span/index.html
[`Subscriber`]: https://docs.rs/tracing/latest/tracing/subscriber/index.html
[`tracing`]: https://crates.io/tracing

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tracing by you, shall be licensed as MIT, without any additional
terms or conditions.
