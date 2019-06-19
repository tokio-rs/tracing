# tokio-trace-nursery

Less-stable utility crates for [`tokio-trace`].

[![MIT licensed][mit-badge]][mit-url]
[![Build Status][travis-badge]][travis-url]
[![Gitter chat][gitter-badge]][gitter-url]

[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[travis-badge]: https://travis-ci.org/tokio-rs/tokio-trace-nursery.svg?branch=master
[travis-url]: https://travis-ci.org/tokio-rs/tokio-trace-nursery/branches
[gitter-badge]: https://img.shields.io/gitter/room/tokio-rs/tokio.svg
[gitter-url]: https://gitter.im/tokio-rs/tokio

[Website](https://tokio.rs) |
[Chat](https://gitter.im/tokio-rs/tokio)

## Overview

[`tokio-trace`] is a framework for instrumenting Rust programs to collect
structured, event-based diagnostic information. This repository contains a set
of utility and compatibility crates for use with `tokio-trace`.

### Stability

While `tokio-trace` and `tokio-trace-core` have been published on crates.io and
adhere to the same stability policies as the rest of the Tokio project, the
crates in the nursery are generally less stable. Many of these crates are not
yet released and are undergoing active development. Therefore, users are warned
that breaking changes may occur.

In general, when depending on a crate from the nursery as a git dependency,
users are advised to pin to a specific git revision using the [`rev`] Cargo key.
This prevents your build from breaking should a breaking change to that crate be
merged to master.

[`rev`]: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-dependencies-from-git-repositories


## Getting Help

First, see if the answer to your question can be found in the API documentation.
If the answer is not there, there is an active community in
the [Tokio Gitter channel][chat]. We would be happy to try to answer your
question.  Last, if that doesn't work, try opening an [issue] with the question.

[chat]: https://gitter.im/tokio-rs/tokio
[issue]: https://github.com/tokio-rs/tokio-trace-nursery/issues/new

## Contributing

:balloon: Thanks for your help improving the project! We are so happy to have
you! We have a [contributing guide][guide] to help you get involved in the Tokio
project.

[guide]: CONTRIBUTING.md
<!--
## Project layout
 TODO: add this
-->

## Supported Rust Versions

Tokio is built against the latest stable, nightly, and beta Rust releases. The
minimum version supported is the stable release from three months before the
current stable release version. For example, if the latest stable Rust is 1.29,
the minimum version supported is 1.26. The current Tokio version is not
guaranteed to build on Rust versions earlier than the minimum supported version.

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tokio by you, shall be licensed as MIT, without any additional
terms or conditions.
