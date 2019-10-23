# tracing-attributes

Macro attributes for application-level tracing.

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![Documentation (master)][docs-master-badge]][docs-master-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][azure-badge]][azure-url]
[![Gitter chat][gitter-badge]][gitter-url]
[![Discord chat][discord-badge]][discord-url]

[Documentation][docs-url] |
[Chat (gitter)][gitter-url] | [Chat (discord)][discord-url]

[crates-badge]: https://img.shields.io/crates/v/tracing-attributes.svg
[crates-url]: https://crates.io/crates/tracing-attributes
[docs-badge]: https://docs.rs/tracing-attributes/badge.svg
[docs-url]: https://docs.rs/tracing-attributes/0.1.5
[docs-master-badge]: https://img.shields.io/badge/docs-master-blue
[docs-master-url]: https://tracing-rs.netlify.com/tracing_attributes
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[azure-badge]: https://dev.azure.com/tracing/tracing/_apis/build/status/tokio-rs.tracing?branchName=master
[azure-url]: https://dev.azure.com/tracing/tracing/_build/latest?definitionId=1&branchName=master
[gitter-badge]: https://img.shields.io/gitter/room/tokio-rs/tracing.svg
[gitter-url]: https://gitter.im/tokio-rs/tracing
[discord-badge]: https://img.shields.io/discord/500028886025895936?logo=discord&label=discord&logoColor=white
[discord-url]: https://discordapp.com/invite/XdPzyTZ

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
tracing-attributes = "0.1.5"
```

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
[span]: https://docs.rs/tracing/0.1.5/tracing/span/index.html

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tokio by you, shall be licensed as MIT, without any additional
terms or conditions.
