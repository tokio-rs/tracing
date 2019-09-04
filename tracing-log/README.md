# tracing-log

[`log`] compatibility for [`tracing`].

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![Documentation (master)][docs-master-badge]][docs-master-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][azure-badge]][azure-url]
[![Gitter chat][gitter-badge]][gitter-url]
![maintenance status][maint-badge]

[Documentation][docs-url] |
[Chat][gitter-url]

[crates-badge]: https://img.shields.io/crates/v/tracing-log.svg
[crates-url]: https://crates.io/crates/tracing-log
[docs-badge]: https://docs.rs/tracing-log/badge.svg
[docs-url]: https://docs.rs/tracing-log
[docs-master-badge]: https://img.shields.io/badge/docs-master-blue
[docs-master-url]: https://tracing-rs.netlify.com/tracing_log
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[azure-badge]: https://dev.azure.com/tracing/tracing/_apis/build/status/tokio-rs.tracing?branchName=master
[azure-url]: https://dev.azure.com/tracing/tracing/_build/latest?definitionId=1&branchName=master
[gitter-badge]: https://img.shields.io/gitter/room/tokio-rs/tracing.svg
[gitter-url]: https://gitter.im/tokio-rs/tracing
[maint-badge]: https://img.shields.io/badge/maintenance-experimental-blue.svg

## Overview

[`tracing`] is a framework for instrumenting Rust programs with context-aware,
structured, event-based diagnostic information. This crate provides
compatibility layers for using `tracing` alongside the logging facade provided
by the [`log`] crate.

This crate provides:

- [`LogTracer`], a [`log::Log`] implementation that consumes [`log::Record`]s
  and outputs them as [`tracing::Event`].
- [`TraceLogger`], a [`tracing::Subscriber`] implementation that consumes
  [`tracing::Event`]s and outputs [`log::Record`], allowing an existing logger
  implementation to be used to record trace events.

[`tracing`]: https://crates.io/crates/tracing
[`log`]: https://crates.io/crates/log
[`LogTracer`]: https://docs.rs/tracing-log/latest/tracing_log/struct.LogTracer.html
[`TraceLogger`]: https://docs.rs/tracing-log/latest/tracing_log/struct.TraceLogger.html
[`log::Log`]: https://docs.rs/log/latest/log/trait.Log.html
[`log::Record`]: https://docs.rs/log/latest/log/struct.Record.html
[`tracing::Subscriber`]: https://docs.rs/tracing/latest/tracing/trait.Subscriber.html
[`tracing::Event`]: https://docs.rs/tracing/latest/tracing/struct.Event.html

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tracing by you, shall be licensed as MIT, without any additional
terms or conditions.
