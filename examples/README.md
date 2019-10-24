# Tracing Examples

This directory contains a collection of examples that demonstrate the use of the
`tracing` ecosystem:

- **tracing**:
  + `counters`: Implements a very simple metrics system to demonstrate how
    subscribers can consume field values as typed data.
  + `sloggish`: A demo `Subscriber` implementation that mimics the output of
    `slog-term`'s `Compact` formatter.
- **tracing-attributes**:
  + `attrs-basic`: A simple example of the `#[instrument]` attribute.
  + `attrs-args`: An example implementing a simple recursive calculation of
    Fibonacci numbers, to demonstrate how the `#[instrument]` attribute can
    record function arguments.
- **tracing-subscriber**:
  + `fmt`: Demonstrates the use of the `fmt` module in `tracing-subscriber`,
    which provides a subscriber implementation that logs traces to the console.
  + `fmt-stderr`: Demonstrates overriding the output stream used by the `fmt`
    subscriber.
  + `subscriber-filter`: Demonstrates the `tracing-subscriber::filter` module,
    which provides a layer which adds configurable filtering to a subscriber
    implementation.
  + `tower-load`: Demonstrates how dynamically reloadable filters can be used to
    debug a server under load in production.
- **tracing-futures**:
  + `futures-proxy-server`: Demonstrates the use of `tracing-futures` by
    implementing a simple proxy server, based on [this example][tokio-proxy]
    from `tokio`.
  + `futures-spawn`: A simple demonstration of the relationship between the
    spans representing spawned futures.
- **tracing-tower**:
  + `tower-h2-client`: Demonstrates the use of `tracing-tower` to instrument a
    simple `tower-h2` HTTP/2 client (based on [this example][h2-client] from
    `tower-h2`).
  + `tower-h2-server`: Demonstrates the use of `tracing-tower` to instrument a
    simple `tower-h2` HTTP/2 server (based on [this example][h2-server] from
    `tower-h2`).
- **tracing-serde**:
  + `serde-yak-shave`: Demonstrates the use of `tracing-serde` by implementing a
    subscriber that emits trace output as JSON.
- **tracing-log**:
  + `hyper-echo`: Demonstrates how `tracing-log` can be used to record
    unstructured logs from dependencies as `tracing` events, by instrumenting
    [this example][echo] from `hyper`, and using `tracing-log` to record logs
    emitted by `hyper`.

The [nightly-examples] directory contains examples of how `tracing` can be used
with async/await. These are kept separate as async/await is currently only
available on nightly Rust toolchains.

[tokio-proxy]: https://github.com/tokio-rs/tokio/blob/v0.1.x/tokio/examples/proxy.rs
[echo]: https://github.com/hyperium/hyper/blob/0.12.x/examples/echo.rs
[h2-client]: https://github.com/tower-rs/tower-h2/blob/0865040d699697bbaf1c3b77b3f256b72f98cdf4/examples/client.rs
[h2-server]: https://github.com/tower-rs/tower-h2/blob/0865040d699697bbaf1c3b77b3f256b72f98cdf4/examples/server.rs
[nightly-examples]: ../nightly-examples
