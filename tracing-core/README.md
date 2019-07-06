# tracing-core

Core primitives for application-level tracing.

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][travis-badge]][travis-url]
[![Gitter chat][gitter-badge]][gitter-url]

[Documentation][docs-url] |
[Chat][gitter-url]

[crates-badge]: https://img.shields.io/crates/v/tracing-core.svg
[crates-url]: https://crates.io/crates/tracing-core
[docs-badge]: https://docs.rs/tracing-core/badge.svg
[docs-url]: https://docs.rs/tracing-core
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[travis-badge]: https://travis-ci.org/tokio-rs/tracing.svg?branch=master
[travis-url]: https://travis-ci.org/tokio-rs/tracing/branches
[gitter-badge]: https://img.shields.io/gitter/room/tokio-rs/tracing.svg
[gitter-url]: https://gitter.im/tokio-rs/tracing

## Overview

[`tracing`] is a framework for instrumenting Rust programs to collect
structured, event-based diagnostic information. This crate defines the core
primitives of `tracing`.

The crate provides:

* [`Span`] identifies a span within the execution of a program.

* [`Event`] represents a single event within a trace.

* [`Subscriber`], the trait implemented to collect trace data.

* [`Metadata`] and [`Callsite`] provide information describing `Span`s.

* [`Field`], [`FieldSet`], [`Value`], and [`ValueSet`] represent the
  structured data attached to a `Span`.

* [`Dispatch`] allows span events to be dispatched to `Subscriber`s.

In addition, it defines the global callsite registry and per-thread current
dispatcher which other components of the tracing system rely on.

Application authors will typically not use this crate directly. Instead, they
will use the [`tracing`] crate, which provides a much more fully-featured
API. However, this crate's API will change very infrequently, so it may be used
when dependencies must be very stable.

[`tracing`]: ../tracing
[`Span`]: https://docs.rs/tracing-core/0.1.1/tracing_core/span/struct.Span.html
[`Event`]: https://docs.rs/tracing-core/0.1.1/tracing_core/event/struct.Event.html
[`Subscriber`]: https://docs.rs/tracing-core/0.1.1/tracing_core/subscriber/trait.Subscriber.html
[`Metadata`]: https://docs.rs/tracing-core/0.1.1/tracing_core/metadata/struct.Metadata.html
[`Callsite`]: https://docs.rs/tracing-core/0.1.1/tracing_core/callsite/trait.Callsite.html
[`Field`]: https://docs.rs/tracing-core/0.1.1/tracing_core/field/struct.Field.html
[`FieldSet`]: https://docs.rs/tracing-core/0.1.1/tracing_core/field/struct.FieldSet.html
[`Value`]: https://docs.rs/tracing-core/0.1.1/tracing_core/field/trait.Value.html
[`ValueSet`]: https://docs.rs/tracing-core/0.1.1/tracing_core/field/struct.ValueSet.html
[`Dispatch`]: https://docs.rs/tracing-core/0.1.1/tracing_core/dispatcher/struct.Dispatch.html

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tokio by you, shall be licensed as MIT, without any additional
terms or conditions.
