# tracing-appender

Writers for logging events and spans

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![Documentation (master)][docs-master-badge]][docs-master-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[Documentation][docs-url] | [Chat][discord-url]

[crates-badge]: https://img.shields.io/crates/v/tracing-appender.svg
[crates-url]: https://crates.io/crates/tracing-appender/0.1.0
[docs-badge]: https://docs.rs/tracing-appender/badge.svg
[docs-url]: https://docs.rs/tracing-appender/0.1.0
[docs-master-badge]: https://img.shields.io/badge/docs-master-blue
[docs-master-url]: https://tracing-rs.netlify.com/tracing-appender
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[actions-badge]: https://github.com/tokio-rs/tracing/workflows/CI/badge.svg
[actions-url]:https://github.com/tokio-rs/tracing/actions?query=workflow%3ACI
[discord-badge]: https://img.shields.io/discord/500028886025895936?logo=discord&label=discord&logoColor=white
[discord-url]: https://discord.gg/EeF3cQw

## Overview

[`tracing`] is a framework for instrumenting Rust programs to collect
structured, event-based diagnostic information. This crate provides the ability
for [`tracing`] events and spans to be recorded in a non-blocking manner by using a
dedicated logging thread. It also provides a [`RollingFileAppender`]
that can be used with or without the non-blocking writer.

## Usage

First, add this to your `Cargo.toml`:
```toml
tracing-appender = "0.1"
```

This crate can be used in a few ways to record spans/events:
 - Using a [`RollingFileAppender`] to perform writes to a log file. This will block on writes.
 - Using *any* `std::io::Write` implementation in a non-blocking way
 - Using a combination of [`NonBlocking`] and [`RollingFileAppender`] to allow writes to a log file
without blocking.

## Rolling File Appender

```rust
fn main(){
    let file_appender = tracing_appender::rolling::hourly("/some/directory", "prefix.log");
}
```
This creates an hourly rotating file appender which outputs to `/some/directory/prefix.log.YYYY-MM-DD-HH`.
[`Rotation::DAILY`] and [`Rotation::NEVER`] are the other available options.

It implements the `std::io::Write` trait. To use with a subscriber, it must be combined with a
[`MakeWriter`] implementation to be able to record tracing spans/event.


The [rolling] module's documentation provides more detail on how to use this file appender.



## Non-Blocking Writer

```rust
fn main() {
    let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
    let subscriber = tracing_subscriber::fmt().with_writer(non_blocking);
    tracing::subscriber::set_global_default(subscriber.finish()).expect("Could not set global default");
}
```
**Note:** `_guard` is a [`WorkerGuard`] which is returned by `tracing_appender::non_blocking`
to ensure buffered logs are flushed to their output in the case of abrupt terminations of a process.
See [`WorkerGuard`] module for more details.

Alternatively, you can provide *any* type that implements [`std::io::Write`] to
[`tracing_appender::non_blocking`]

The [non_blocking] module's documentation provides more detail on how to use `non_blocking`.

## Non-Blocking Rolling File Appender

```rust
fn main() {
    let file_appender = tracing_appender::rolling::hourly("/some/directory", "prefix.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let subscriber = tracing_subscriber::fmt().with_writer(non_blocking);
    
    tracing::subscriber::set_global_default(subscriber.finish()).expect("Could not set global default");
}
```

[`tracing`]: https://docs.rs/tracing/latest/tracing/
[`MakeWriter`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/trait.MakeWriter.html
[write]: https://doc.rust-lang.org/std/io/trait.Write.html
[non_blocking]: https://docs.rs/tracing-appender/latest/tracing_appender/non_blocking/indexx.html
[rolling]: https://docs.rs/tracing-appender/latest/tracing_appender/rolling/index.html
[`WorkerGuard`]: https://docs.rs/tracing-appender/latest/tracing_appender/non_blocking/struct.WorkerGuard.html
[`RollingFileAppender`]: https://docs.rs/tracing-appender/latest/tracing_appender/rolling/struct.RollingFileAppender.html

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tokio by you, shall be licensed as MIT, without any additional
terms or conditions.
