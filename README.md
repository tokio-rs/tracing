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

## Usage

### In libraries

Libraries should only rely on the `tracing` crate and use the provided macros
and types to collect whatever information might be useful to downstream consumers.

```toml
[dependencies]
tracing = "0.1"
```

```rust
use tracing::{debug, error, info, span, trace, warn, Level};

use std::{error::Error, fmt};

#[tracing::instrument]
pub fn shave(yak: usize) -> Result<(), Box<dyn Error + 'static>> {
    debug!(
        message = "hello! I'm gonna shave a yak.",
        excitement = "yay!"
    );
    if yak == 3 {
        warn!(target: "yak_events", "could not locate yak!");
        return Err(ShaveError::new(yak, YakError::new("could not locate yak")).into());
    } else {
        trace!(target: "yak_events", "yak shaved successfully");
    }
    Ok(())
}

pub fn shave_all(yaks: usize) -> usize {
    let span = span!(Level::TRACE, "shaving_yaks", yaks_to_shave = yaks);
    let _enter = span.enter();

    info!("shaving yaks");

    let mut num_shaved = 0;
    for yak in 1..=yaks {
        let res = shave(yak);
        trace!(target: "yak_events", yak, shaved = res.is_ok());

        if let Err(ref error) = res {
            error!(
                message = "failed to shave yak!",
                yak,
                error = error.as_ref()
            );
        } else {
            num_shaved += 1;
        }

        trace!(target: "yak_events", yaks_shaved = num_shaved);
    }

    num_shaved
}
```

### In applications

In order to produce output, applications need a concrete subscriber implementation. 
[`tracing_subscriber`](https://docs.rs/tracing-subscriber/) is a reasonable default.
By default, `tracing-subscriber` is able to consume messages emitted by
`log`-instrumented libraries and modules.

```rust
use tracing::debug;
use tracing_subscriber;
use yak_shave;

fn main() {
    tracing_subscriber::fmt::init();

    let number_of_yaks = 3;
    debug!("preparing to shave {} yaks", number_of_yaks);

    let number_shaved = yak_shave::shave_all(number_of_yaks);

    debug!(
        message = "yak shaving completed.",
        all_yaks_shaved = number_shaved == number_of_yaks,
    );
}
```

Note that if you are instrumenting libraries or applications that make use of [`std::future::Future`](https://doc.rust-lang.org/stable/std/future/trait.Future.html) or async/await, be sure to use the [`tracing-futures`](https://docs.rs/tracing-futures) crate.

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
