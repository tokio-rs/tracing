# tracing

Application-level tracing for Rust.

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![Documentation (master)][docs-master-badge]][docs-master-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/tracing.svg
[crates-url]: https://crates.io/crates/tracing
[docs-badge]: https://docs.rs/tracing/badge.svg
[docs-url]: https://docs.rs/tracing
[docs-master-badge]: https://img.shields.io/badge/docs-master-blue
[docs-master-url]: https://tracing-rs.netlify.com
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[actions-badge]: https://github.com/tokio-rs/tracing/workflows/CI/badge.svg
[actions-url]:https://github.com/tokio-rs/tracing/actions?query=workflow%3ACI
[discord-badge]: https://img.shields.io/discord/500028886025895936?logo=discord&label=discord&logoColor=white
[discord-url]: https://discord.gg/EeF3cQw

[Website](https://tokio.rs) |
[Chat](https://discord.gg/EeF3cQw) | [Documentation (master branch)](https://tracing-rs.netlify.com/)

## Overview

`tracing` is a framework for instrumenting Rust programs to collect
structured, event-based diagnostic information. `tracing` is maintained by the
Tokio project, but does _not_ require the `tokio` runtime to be used.

## Getting Help

First, see if the answer to your question can be found in the API documentation.
If the answer is not there, there is an active community in
the [Tracing Discord channel][chat]. We would be happy to try to answer your
question.  Last, if that doesn't work, try opening an [issue] with the question.

[chat]: https://discord.gg/EeF3cQw
[issue]: https://github.com/tokio-rs/tracing/issues/new

## Contributing

:balloon: Thanks for your help improving the project! We are so happy to have
you! We have a [contributing guide][guide] to help you get involved in the Tracing
project.

[guide]: CONTRIBUTING.md

## Project layout

The [`tracing`] crate contains the primary _instrumentation_ API, used for
instrumenting libraries and applications to emit trace data. The [`tracing-core`]
crate contains the _core_ API primitives on which the rest of `tracing` is
instrumented. Authors of trace subscribers may depend on `tracing-core`, which
guarantees a higher level of stability.

Additionally, this repository contains several compatibility and utility
libraries built on top of `tracing`. Some of these crates are in a pre-release
state, and are less stable than the `tracing` and `tracing-core` crates.

The crates included as part of Tracing are:

* [`tracing-futures`]: Utilities for instrumenting `futures`.
  ([crates.io][fut-crates]|[docs][fut-docs])

* [`tracing-macros`]: Experimental macros for emitting trace events (unstable).

* [`tracing-attributes`]: Procedural macro attributes for automatically
    instrumenting functions. ([crates.io][attr-crates]|[docs][attr-docs])

* [`tracing-log`]: Compatibility with the `log` crate (unstable).

* [`tracing-serde`]: A compatibility layer for serializing trace data with
    `serde` (unstable).

* [`tracing-subscriber`]: Subscriber implementations, and utilities for
  implementing and composing `Subscriber`s.
  ([crates.io][sub-crates]|[docs][sub-docs])

* [`tracing-tower`]: Compatibility with the `tower` ecosystem (unstable).

[`tracing`]: tracing
[`tracing-core`]: tracing
[`tracing-futures`]: tracing-futures
[`tracing-macros`]: tracing-macros
[`tracing-attributes`]: tracing-attributes
[`tracing-log`]: tracing-log
[`tracing-serde`]: tracing-serde
[`tracing-subscriber`]: tracing-subscriber
[`tracing-tower`]: tracing-tower

[fut-crates]: https://crates.io/crates/tracing-futures
[fut-docs]: https://docs.rs/tracing-futures

[attr-crates]: https://crates.io/crates/tracing-attributes
[attr-docs]: https://docs.rs/tracing-attributes

[sub-crates]: https://crates.io/crates/tracing-subscriber
[sub-docs]: https://docs.rs/tracing-subscriber

## Related Crates

In addition to this repository, here are also several third-party crates which
are not maintained by the `tokio` project. These include:

- [`tracing-timing`] implements inter-event timing metrics on top of `tracing`.
  It provides a subscriber that records the time elapsed between pairs of
  `tracing` events and generates histograms.
- [`tracing-opentelemetry`] provides a subscriber for emitting traces to
  [OpenTelemetry]-compatible distributed tracing systems.
- [`tracing-honeycomb`] implements a subscriber for reporting traces to
  [honeycomb.io].
- [`tracing-actix`] provides `tracing` integration for the `actix` actor
  framework.
- [`tracing-gelf`] implements a subscriber for exporting traces in Greylog
  GELF format.
- [`tracing-coz`] provides integration with the [coz] causal profiler
  (Linux-only).

(if you're the maintainer of a `tracing` ecosystem crate not in this list,
please let us know!)

[`tracing-timing`]: https://crates.io/crates/tracing-timing
[`tracing-opentelemetry`]: https://crates.io/crates/tracing-opentelemetry
[OpenTelemetry]: https://opentelemetry.io/
[`tracing-honeycomb`]: https://crates.io/crates/honeycomb-tracing
[honeycomb.io]: https://www.honeycomb.io/
[`tracing-actix]: https://crates.io/crates/tracing-actix
[`tracing-gelf`]: https://crates.io/crates/tracing-gelf
[`tracing-coz`]: https://crates.io/crates/tracing-coz
[coz]: https://github.com/plasma-umass/coz

**Note:** that some of the ecosystem crates are currently unreleased and
undergoing active development. They may be less stable than `tracing` and
`tracing-core`.

## External Resources

This is a list of links to blog posts, conference talks, and tutorials about
Tracing.

#### Blog Posts

* [Diagnostics with Tracing][tokio-blog-2019-08] on the Tokio blog, August 2019

[tokio-blog-2019-08]: https://tokio.rs/blog/2019-08-tracing/

#### Talks

* [Bay Area Rust Meetup talk and Q&A][bay-rust-2018-03], March 2018
* [RustConf 2019 talk][rust-conf-2019-08-video] and [slides][rust-conf-2019-08-slides], August 2019

[bay-rust-2018-03]: https://www.youtube.com/watch?v=j_kXRg3zlec
[rust-conf-2019-08-video]: https://www.youtube.com/watch?v=JjItsfqFIdo
[rust-conf-2019-08-slides]: https://www.elizas.website/slides/rustconf-8-2019.pdf

Help us expand this list! If you've written or spoken about Tracing, or
know of resources that aren't listed, please open a pull request adding them.

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tracing by you, shall be licensed as MIT, without any additional
terms or conditions.
