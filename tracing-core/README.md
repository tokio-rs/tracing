# tracing-core

Core primitives for `tracing`.

[Documentation](https://docs.rs/tracing-core/0.2.0/tracing_core/index.html)

## Overview

`tracing` is a framework for instrumenting Rust programs to collect
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

[`tracing`]: ../
[`Span`]: https://docs.rs/tracing-core/0.2.0/tracing_core/span/struct.Span.html
[`Event`]: https://docs.rs/tracing-core/0.2.0/tracing_core/event/struct.Event.html
[`Subscriber`]: https://docs.rs/tracing-core/0.2.0/tracing_core/subscriber/trait.Subscriber.html
[`Metadata`]: https://docs.rs/tracing-core/0.2.0/tracing_core/metadata/struct.Metadata.html
[`Callsite`]: https://docs.rs/tracing-core/0.2.0/tracing_core/callsite/trait.Callsite.html
[`Field`]: https://docs.rs/tracing-core/0.2.0/tracing_core/field/struct.Field.html
[`FieldSet`]: https://docs.rs/tracing-core/0.2.0/tracing_core/field/struct.FieldSet.html
[`Value`]: https://docs.rs/tracing-core/0.2.0/tracing_core/field/trait.Value.html
[`ValueSet`]: https://docs.rs/tracing-core/0.2.0/tracing_core/field/struct.ValueSet.html
[`Dispatch`]: https://docs.rs/tracing-core/0.2.0/tracing_core/dispatcher/struct.Dispatch.html

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tokio by you, shall be licensed as MIT, without any additional
terms or conditions.
