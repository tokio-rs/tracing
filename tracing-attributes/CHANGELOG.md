# 0.1.21 (April 26, 2022)

This release adds support for setting explicit parent and follows-from spans
in the `#[instrument]` attribute.

### Added

- `#[instrument(follows_from = ...)]` argument for setting one or more
  follows-from span ([#2093])
- `#[instrument(parent = ...)]` argument for overriding the generated span's
  parent ([#2091])

### Fixed

- Extra braces around `async` blocks in expanded code (causes a Clippy warning)
  ([#2090])
- Broken documentation links ([#2068], [#2077])

Thanks to @jarrodldavis, @ben0x539, and new contributor @jswrenn for
contributing to this release!


[#2093]: https://github.com/tokio-rs/tracing/pull/2093
[#2091]: https://github.com/tokio-rs/tracing/pull/2091
[#2090]: https://github.com/tokio-rs/tracing/pull/2090
[#2077]: https://github.com/tokio-rs/tracing/pull/2077
[#2068]: https://github.com/tokio-rs/tracing/pull/2068

# 0.1.20 (March 8, 2022)

### Fixed

- Compilation failure with `--minimal-versions` due to a too-permissive `syn`
  dependency ([#1960])

### Changed

- Bumped minimum supported Rust version (MSRV) to 1.49.0 ([#1913])

Thanks to new contributor @udoprog for contributing to this release!

[#1960]: https://github.com/tokio-rs/tracing/pull/1960
[#1913]: https://github.com/tokio-rs/tracing/pull/1913

# 0.1.19 (February 3, 2022)

This release introduces a new `#[instrument(ret)]` argument to emit an event
with the return value of an instrumented function.

### Added

- `#[instrument(ret)]` to record the return value of a function ([#1716])
- added `err(Debug)` argument to cause `#[instrument(err)]` to record errors
  with `Debug` rather than `Display ([#1631])

### Fixed

- incorrect code generation for functions returning async blocks ([#1866])
- incorrect diagnostics when using `rust-analyzer` ([#1634])

Thanks to @Swatinem, @hkmatsumoto, @cynecx, and @ciuncan for contributing to
this release!

[#1716]: https://github.com/tokio-rs/tracing/pull/1716
[#1631]: https://github.com/tokio-rs/tracing/pull/1631
[#1634]: https://github.com/tokio-rs/tracing/pull/1634
[#1866]: https://github.com/tokio-rs/tracing/pull/1866

# 0.1.18 (October 5, 2021)

This release fixes issues introduced in v0.1.17.

### Fixed

- fixed mismatched types compiler error that may occur when using
  `#[instrument]` on an `async fn` that returns an `impl Trait` value that
  includes a closure ([#1616])
- fixed false positives for `clippy::suspicious_else_formatting` warnings due to
  rust-lang/rust-clippy#7760 and rust-lang/rust-clippy#6249 ([#1617])
- fixed `clippy::let_unit_value` lints when using `#[instrument]` ([#1614])

[#1617]: https://github.com/tokio-rs/tracing/pull/1617
[#1616]: https://github.com/tokio-rs/tracing/pull/1616
[#1614]: https://github.com/tokio-rs/tracing/pull/1614

# 0.1.17 (YANKED) (October 1, 2021)

This release significantly improves performance when `#[instrument]`-generated
spans are below the maximum enabled level.

### Added

- improve performance when skipping `#[instrument]`-generated spans below the
  max level ([#1600], [#1605])

Thanks to @oli-obk for contributing to this release!

[#1600]: https://github.com/tokio-rs/tracing/pull/1600
[#1605]: https://github.com/tokio-rs/tracing/pull/1605

# 0.1.16 (September 13, 2021)

This release adds a new `#[instrument(skip_all)]` option to skip recording *all*
arguments to an instrumented function as fields. Additionally, it adds support
for recording arguments that are `tracing` primitive types as typed values,
rather than as `fmt::Debug`.

### Added

- add `skip_all` option to `#[instrument]` ([#1548])
- record primitive types as primitive values rather than as `fmt::Debug`
  ([#1378])
- added support for `f64`s as typed values ([#1522])

Thanks to @Folyd and @jsgf for contributing to this release!

[#1548]: https://github.com/tokio-rs/tracing/pull/1548
[#1378]: https://github.com/tokio-rs/tracing/pull/1378
[#1522]: https://github.com/tokio-rs/tracing/pull/1524

# 0.1.15 (March 12, 2021)

### Fixed

- `#[instrument]` on functions returning `Box::pin`ned futures incorrectly
  skipping function bodies prior to returning a future ([#1297])

Thanks to @nightmared for contributing to this release!

[#1297]: https://github.com/tokio-rs/tracing/pull/1297

# 0.1.14 (March 10, 2021)

### Fixed

- Compatibility between `#[instrument]` and `async-trait` v0.1.43 and newer
  ([#1228])

Thanks to @nightmared for lots of hard work on this fix!

[#1228]: https://github.com/tokio-rs/tracing/pull/1228

# 0.1.13 (February 17, 2021)

### Fixed

- Compiler error when using `#[instrument(err)]` on functions which return `impl
  Trait` ([#1236])

[#1236]: https://github.com/tokio-rs/tracing/pull/1236

# 0.1.12 (February 4, 2021)

### Fixed

- Compiler error when using `#[instrument(err)]` on functions with mutable
  parameters ([#1167])
- Missing function visibility modifier when using `#[instrument]` with
  `async-trait` ([#977])
- Multiple documentation fixes and improvements ([#965], [#981], [#1215])

### Changed

- `tracing-futures` dependency is no longer required when using `#[instrument]`
  on async functions ([#808])

Thanks to @nagisa, @Txuritan, @TaKO8Ki, and @okready for contributing to this
release!

[#1167]: https://github.com/tokio-rs/tracing/pull/1167
[#977]: https://github.com/tokio-rs/tracing/pull/977
[#965]: https://github.com/tokio-rs/tracing/pull/965
[#981]: https://github.com/tokio-rs/tracing/pull/981
[#1215]: https://github.com/tokio-rs/tracing/pull/1215
[#808]: https://github.com/tokio-rs/tracing/pull/808

# 0.1.11 (August 18, 2020)

### Fixed

- Corrected wrong minimum supported Rust version note in docs (#941)
- Removed unused `syn` features (#928)

Thanks to new contributor @jhpratt for contributing to this release!

# 0.1.10 (August 10, 2020)

### Added

- Support for using `self` in field expressions when instrumenting `async-trait`
  functions (#875)
- Several documentation improvements (#832, #897, #911, #913)

Thanks to @anton-dutov and @nightmared for contributing to this release!

# 0.1.9 (July 8, 2020)

### Added

- Support for arbitrary expressions as fields in `#[instrument]` (#672)

### Changed

- `#[instrument]` now emits a compiler warning when ignoring unrecognized
  input (#672, #786)

# 0.1.8 (May 13, 2020)

### Added

- Support for using `#[instrument]` on methods that are part of [`async-trait`]
  trait implementations (#711)
- Optional `#[instrument(err)]` argument to automatically emit an event if an
  instrumented function returns `Err` (#637) 

Thanks to @ilana and @nightmared for contributing to this release!

[`async-trait`]: https://crates.io/crates/async-trait

# 0.1.7 (February 26, 2020)

### Added

- Support for adding arbitrary literal fields to spans generated by
  `#[instrument]` (#569)
- `#[instrument]` now emits a helpful compiler error when attempting to skip a
  function parameter (#600)

Thanks to @Kobzol for contributing to this release!

# 0.1.6 (December 20, 2019)

### Added

-  Updated documentation (#468)

# 0.1.5 (October 22, 2019)

### Added

- Support for destructuring in arguments to `#[instrument]`ed functions (#397)
- Generated field for `self` parameters when `#[instrument]`ing methods (#397)

# 0.1.4 (September 26, 2019)

### Added

- Optional `skip` argument to `#[instrument]` for excluding function parameters
  from generated spans (#359)

# 0.1.3 (September 12, 2019)

### Fixed

- Fixed `#[instrument]`ed async functions not compiling on `nightly-2019-09-11`
  or newer (#342)

# 0.1.2 (August 19, 2019)

### Changed

- Updated `syn` and `quote` dependencies to 1.0 (#292)
- Removed direct dependency on `proc-macro2` to avoid potential version
  conflicts (#296)

### Fixed

- Outdated idioms in examples (#271, #273)

# 0.1.1 (August 9, 2019)

### Changed

- Using the `#[instrument]` attribute on `async fn`s no longer requires a
  feature flag (#258)

### Fixed

- The `#[instrument]` macro now works on generic functions (#262)

# 0.1.0 (August 8, 2019)

- Initial release
