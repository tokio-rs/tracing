# tracing-fmt

**Warning: Until `tracing-fmt` has a 0.1.0 release on crates.io, please treat every release as potentially breaking.**

A (currently experimental) [`tracing`][tracing] subscriber
that formats, colors, and logs trace data.

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][azure-badge]][azure-url]
[![Gitter chat][gitter-badge]][gitter-url]

[Documentation][docs-url] |
[Chat][gitter-url]

[tracing]: https://crates.io/crates/tracing
[tracing-fmt]: https://github.com/tokio-rs/tracing/tree/master/tracing-fmt
[crates-badge]: https://img.shields.io/crates/v/tracing-fmt.svg
[crates-url]: https://crates.io/crates/tracing-fmt
[docs-badge]: https://docs.rs/tracing-fmt/badge.svg
[docs-url]: https://docs.rs/tracing-fmt
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[azure-badge]: https://dev.azure.com/tracing/tracing/_apis/build/status/tokio-rs.tracing?branchName=master
[azure-url]: https://dev.azure.com/tracing/tracing/_build/latest?definitionId=1&branchName=master
[gitter-badge]: https://img.shields.io/gitter/room/tokio-rs/tracing.svg
[gitter-url]: https://gitter.im/tokio-rs/tracing

## Overview

[`tracing`][tracing] is a framework for instrumenting Rust programs with context-aware,
structured, event-based diagnostic information. This crate provides an
implementation of the [`Subscriber`] trait that records `tracing`'s `Event`s
and `Span`s by formatting them as text and logging them to stdout.

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tracing by you, shall be licensed as MIT, without any additional
terms or conditions.
