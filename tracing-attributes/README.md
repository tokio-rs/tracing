# tracing-attributes

Macro attributes for application-level tracing.

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![Documentation (master)][docs-master-badge]][docs-master-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[Documentation][docs-url] | [Chat][discord-url]

[crates-badge]: https://img.shields.io/crates/v/tracing-attributes.svg
[crates-url]: https://crates.io/crates/tracing-attributes
[docs-badge]: https://docs.rs/tracing-attributes/badge.svg
[docs-url]: https://docs.rs/tracing-attributes/0.1.6
[docs-master-badge]: https://img.shields.io/badge/docs-master-blue
[docs-master-url]: https://tracing-rs.netlify.com/tracing_attributes
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[actions-badge]: https://github.com/tokio-rs/tracing/workflows/CI/badge.svg
[actions-url]:https://github.com/tokio-rs/tracing/actions?query=workflow%3ACI
[discord-badge]: https://img.shields.io/discord/500028886025895936?logo=discord&label=discord&logoColor=white
[discord-url]: https://discord.gg/EeF3cQw

## Overview

[`tracing`] is a framework for instrumenting Rust programs to collect
structured, event-based diagnostic information. This crate provides the
`#[instrument]` attribute for automatically instrumenting functions using
`tracing`.

Note that this macro is also re-exported by the main `tracing` crate.

## Usage

First, add this to your `Cargo.toml`:

```toml
[dependencies]
tracing-attributes = "0.1.6"
```

*Compiler support: requires rustc 1.39+*

This crate provides the `#[instrument]` attribute for instrumenting a function
with a `tracing` [span]. For example:

```rust
use tracing_attributes::instrument;

#[instrument]
pub fn my_function(my_arg: usize) {
    // ...
}
```


[`tracing`]: https://crates.io/crates/tracing
[span]: https://docs.rs/tracing/0.1.6/tracing/span/index.html

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tokio by you, shall be licensed as MIT, without any additional
terms or conditions.
