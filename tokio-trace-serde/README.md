# tokio-trace-serde

An adapter for serializing `tokio-trace` types using `serde`.

[Documentation](https://docs.rs/tokio-trace-serde/0.1.0/tokio_trace_serde/index.html)

## Overview

`tokio-trace-serde` enables serializing `tokio-trace` types using
`serde`. `tokio-trace` is a framework for instrumenting Rust programs
to collect structured, event-based diagnostic information.

Traditional logging is based on human-readable text messages.
`tokio-trace` gives us machine-readable structured diagnostic
information. This lets us interact with diagnostic data
programmatically. With `tokio-trace-serde`, you can implement a
`Subscriber` to serialize your `tokio-trace` types and make use of the
existing ecosystem of `serde` serializers to talk with distributed
tracing systems.

Serializing diagnostic information allows us to do more with our logged
values. For instance, when working with logging data in JSON gives us
pretty-print when we're debugging in development and you can emit JSON
and tracing data to monitor your services in production.

The `tokio-trace` crate provides the APIs necessary for instrumenting
libraries and applications to emit trace data.

## Usage

First, add this to your `Cargo.toml`:

```toml
[dependencies]
tokio-trace = "0.1"
tokio-trace-serde = "0.1"
```

Next, add this to your crate:

```rust
#[macro_use]
extern crate tokio_trace;
extern crate tokio_trace_serde;

use tokio_trace_serde::AsSerde;
```

Please read the [`tokio-trace` documentation](https://docs.rs/tokio-trace/0.1.0/tokio_trace/index.html)
for more information on how to create trace data.

This crate provides the `as_serde` function, via the `AsSerde` trait,
which enables serializing the `Attributes`, `Event`, `Id`, `Metadata`,
and `Record` `tokio-trace` values.

For the full example, please see the [examples](../examples) folder.

Implement a `Subscriber` to format the serialization of `tokio-trace`
types how you'd like.

```rust
pub struct JsonSubscriber {
    next_id: AtomicUsize, // you need to assign span IDs, so you need a counter
}

impl Subscriber for JsonSubscriber {

    fn new_span(&self, attrs: &Attributes) -> Id {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let id = Id::from_u64(id as u64);
        let json = json!({
        "new_span": {
            "attributes": attrs.as_serde(),
            "id": id.as_serde(),
        }});
        println!("{}", json);
        id
    }
    // ...
}
```

After you implement your `Subscriber`, you can use your `tokio-trace`
subscriber, `JsonSubscriber` in the above example, to record serialized
trace data.

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tokio by you, shall be licensed as MIT, without any additional
terms or conditions.
