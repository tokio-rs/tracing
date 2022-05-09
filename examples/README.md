# Tracing Examples

This directory contains a collection of examples that demonstrate the use of the
`tracing` ecosystem:

- **tracing**:
  + `counters`: Implements a very simple metrics system to demonstrate how
    collectors can consume field values as typed data.
  + `sloggish`: A demo `Collect` implementation that mimics the output of
    `slog-term`'s `Compact` formatter.
- **tracing-attributes**:
  + `attrs-basic`: A simple example of the `#[instrument]` attribute.
  + `attrs-literal-field-names`: Demonstrates using literal field names rather
    than rust tokens..
  + `attrs-args`: An example implementing a simple recursive calculation of
    Fibonacci numbers, to demonstrate how the `#[instrument]` attribute can
    record function arguments.
- **tracing-subscriber**:
  + `fmt`: Demonstrates the use of the `fmt` module in `tracing-subscriber`,
    which provides a collector implementation that logs traces to the console.
  + `fmt-stderr`: Demonstrates overriding the output stream used by the `fmt`
    collector.
  + `fmt-custom-field`: Demonstrates overriding how the `fmt` collector formats
    fields on spans and events.
  + `fmt-custom-event`: Demonstrates overriding how the `fmt` collector formats
    events.
  + `fmt-multiple-writers.rs`: demonstrates how `fmt::Subcriber` can write to multiple
    destinations (in this instance, stdout and a file) simultaneously.
  + `fmt-source-locations.rs`: demonstrates displaying source code locations
    with `fmt::Subscriber`.
  + `subscriber-filter`: Demonstrates the `tracing-subscriber::filter` module,
    which provides a subscriber which adds configurable filtering to a collector
    implementation.
  + `tower-load`: Demonstrates how dynamically reloadable filters can be used to
    debug a server under load in production.
  + `journald`: Demonstrates how to use `fmt` and `journald` subscribers to output to
    both the terminal and the system journal.
  + `toggle-subscribers` : Demonstrates how subscribers can be wrapped with an `Option` allowing
    them to be dynamically toggled.
- **tracing-futures**:
  + `spawny-thing`: Demonstrates the use of the `#[instrument]` attribute macro
    asynchronous functions.
  + `tokio-spawny-thing.rs`: Similar to `spawny-thingy`, but with the additional
    demonstration instrumenting [concurrent tasks][tasks] created with 
    `tokio::spawn`.
  + `futures-proxy-server`: Demonstrates the use of `tracing-futures` by
    implementing a simple proxy server, based on [this example][tokio-proxy]
    from `tokio`.
  + `async_fn`: Demonstrates how asynchronous functions can be
     instrumented.
  + `echo`: Demonstrates a `tracing`-instrumented variant of Tokio's `echo` example.
- **tracing-flame**:
  + `infero-flame`: Demonstrates the use of `tracing-flame` to generate a flamegraph
     from spans.
- **tracing-tower**:
  + `tower-client`: Demonstrates the use of `tracing-tower` to instrument a
    simple `tower` HTTP/1.1 client.
  + `tower-server`: Demonstrates the use of `tracing-tower` to instrument a
    simple `tower` HTTP/1.1 server.
- **tracing-serde**:
  + `serde-yak-shave`: Demonstrates the use of `tracing-serde` by implementing a
    collector that emits trace output as JSON.
- **tracing-log**:
  + `hyper-echo`: Demonstrates how `tracing-log` can be used to record
    unstructured logs from dependencies as `tracing` events, by instrumenting
    [this example][echo] from `hyper`, and using `tracing-log` to record logs
    emitted by `hyper`.
- **tracing-opentelemetry**:
  + `opentelemetry`: Demonstrates how `tracing-opentelemetry` can be used to
    export and visualize `tracing` span data.
  + `opentelemetry-remote-context`: Demonstrates how `tracing-opentelemetry`
    can be used to extract and inject remote context when traces span multiple
    systems.

[tasks]: (https://docs.rs/tokio/0.2.21/tokio/task/index.html)
[tokio-proxy]: https://github.com/tokio-rs/tokio/blob/v0.1.x/tokio/examples/proxy.rs
[echo]: https://github.com/hyperium/hyper/blob/0.12.x/examples/echo.rs
