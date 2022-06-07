# 0.1.27 (June 7, 2022)

This release of `tracing-core` introduces a new `DefaultCallsite` type, which
can be used by instrumentation crates rather than implementing their own
callsite types. Using `DefaultCallsite` may offer reduced overhead from callsite
registration.

### Added

- `DefaultCallsite`, a pre-written `Callsite` implementation for use in
  instrumentation crates ([#2083])
- `ValueSet::len` and `Record::len` methods returning the number of fields in a
  `ValueSet` or `Record` ([#2152])

### Changed

- Replaced `lazy_static` dependency with `once_cell` ([#2147])

### Documented

- Added documentation to the `callsite` module ([#2088], [#2149])

Thanks to new contributors @jamesmunns and @james7132 for contributing to this
release!

[#2083]: https://github.com/tokio-rs/tracing/pull/2083
[#2152]: https://github.com/tokio-rs/tracing/pull/2152
[#2147]: https://github.com/tokio-rs/tracing/pull/2147
[#2088]: https://github.com/tokio-rs/tracing/pull/2088
[#2149]: https://github.com/tokio-rs/tracing/pull/2149

# 0.1.26 (April 14, 2022)

This release adds a `Value` implementation for `Box<T: Value>` to allow
recording boxed values more conveniently. In particular, this should improve
the ergonomics of the implementations for `dyn std::error::Error` trait objects,
including those added in [v0.1.25].

### Added

- `Value` implementation for `Box<T> where T: Value` ([#2071])

### Fixed

- Broken documentation links ([#2068])

Thanks to new contributor @ben0x539 for contributing to this release!


[v0.1.25]: https://github.com/tokio-rs/tracing/releases/tag/tracing-core-0.1.25
[#2071]: https://github.com/tokio-rs/tracing/pull/2071
[#2068]: https://github.com/tokio-rs/tracing/pull/2068

# 0.1.25 (April 12, 2022)

This release adds additional `Value` implementations for `std::error::Error`
trait objects with auto trait bounds (`Send` and `Sync`), as Rust will not
auto-coerce trait objects. Additionally, it fixes a bug when setting scoped
dispatchers that was introduced in the previous release ([v0.1.24]).

### Added

- `Value` implementations for `dyn Error + Send + 'static`, `dyn Error + Send +
  Sync + 'static`, `dyn Error + Sync + 'static` ([#2066])

### Fixed

- Failure to use the global default dispatcher if a thread has set a scoped
  default prior to setting the global default, and unset the scoped default
  after setting the global default ([#2065])

Thanks to @lilyball for contributing to this release!

[v0.1.24]: https://github.com/tokio-rs/tracing/releases/tag/tracing-core-0.1.24
[#2066]: https://github.com/tokio-rs/tracing/pull/2066
[#2065]: https://github.com/tokio-rs/tracing/pull/2065

# 0.1.24 (April 1, 2022)

This release fixes a bug where setting `NoSubscriber` as the local default would
not disable the global default subscriber locally.

### Fixed

- Setting `NoSubscriber` as the local default now correctly disables the global
  default subscriber ([#2001])
- Fixed compilation warnings with the "std" feature disabled ([#2022])

### Changed

- Removed unnecessary use of `write!` and `format_args!` macros ([#1988])

[#1988]: https://github.com/tokio-rs/tracing/pull/1988
[#2001]: https://github.com/tokio-rs/tracing/pull/2001
[#2022]: https://github.com/tokio-rs/tracing/pull/2022

# 0.1.23 (March 8, 2022)

### Changed

- Removed `#[inline]` attributes from some `Dispatch` methods whose
  callers are now inlined ([#1974])
- Bumped minimum supported Rust version (MSRV) to Rust 1.49.0 ([#1913])

[#1913]: https://github.com/tokio-rs/tracing/pull/1913
[#1974]: https://github.com/tokio-rs/tracing/pull/1974

# 0.1.22 (February 3, 2022)

This release adds *experimental* support for recording structured field values
using the [`valuable`] crate. See [this blog post][post] for details on
`valuable`.

Note that `valuable` support currently requires `--cfg tracing_unstable`. See
the documentation for details.

### Added

- **field**: Experimental support for recording field values using the
  [`valuable`] crate ([#1608], [#1888], [#1887])
- **field**: Added `ValueSet::record` method ([#1823])
- **subscriber**: `Default` impl for `NoSubscriber` ([#1785])
- **metadata**: New `Kind::HINT` to support the `enabled!` macro in `tracing`
  ([#1883], [#1891])
### Fixed

- Fixed a number of documentation issues ([#1665], [#1692], [#1737])

Thanks to @xd009642, @Skepfyr, @guswynn, @Folyd, and @mbergkvist for
contributing to this release!

[`valuable`]: https://crates.io/crates/valuable
[post]: https://tokio.rs/blog/2021-05-valuable
[#1608]: https://github.com/tokio-rs/tracing/pull/1608
[#1888]: https://github.com/tokio-rs/tracing/pull/1888
[#1887]: https://github.com/tokio-rs/tracing/pull/1887
[#1823]: https://github.com/tokio-rs/tracing/pull/1823
[#1785]: https://github.com/tokio-rs/tracing/pull/1785
[#1883]: https://github.com/tokio-rs/tracing/pull/1883
[#1891]: https://github.com/tokio-rs/tracing/pull/1891
[#1665]: https://github.com/tokio-rs/tracing/pull/1665
[#1692]: https://github.com/tokio-rs/tracing/pull/1692
[#1737]: https://github.com/tokio-rs/tracing/pull/1737

# 0.1.21 (October 1, 2021)

This release adds support for recording `Option<T> where T: Value` as typed
`tracing` field values.

### Added

- **field**: `Value` impl for `Option<T> where T: Value` ([#1585])

### Fixed

- Fixed deprecation warnings when building with `default-features` disabled
  ([#1603], [#1606])
- Documentation fixes and improvements ([#1595], [#1601])

Thanks to @brianburgers, @DCjanus, and @matklad for contributing to this
release!

[#1585]: https://github.com/tokio-rs/tracing/pull/1585
[#1595]: https://github.com/tokio-rs/tracing/pull/1595
[#1601]: https://github.com/tokio-rs/tracing/pull/1601
[#1603]: https://github.com/tokio-rs/tracing/pull/1603
[#1606]: https://github.com/tokio-rs/tracing/pull/1606

# 0.1.20 (September 12, 2021)

This release adds support for `f64` as one of the `tracing-core`
primitive field values, allowing floating-point values to be recorded as
typed values rather than with `fmt::Debug`. Additionally, it adds
`NoSubscriber`, a `Subscriber` implementation that does nothing.

### Added

- **subscriber**: `NoSubscriber`, a no-op `Subscriber` implementation
  ([#1549])
- **field**: Added `Visit::record_f64` and support for recording
  floating-point values ([#1507])

Thanks to new contributors @jsgf and @maxburke for contributing to this
release!

[#1549]: https://github.com/tokio-rs/tracing/pull/1549 
[#1507]: https://github.com/tokio-rs/tracing/pull/1507

# 0.1.19 (August 17, 2021)
### Added

- `Level::as_str` ([#1413])
- `Hash` implementation for `Level` and `LevelFilter` ([#1456])
- `Value` implementation for `&mut T where T: Value` ([#1385])
- Multiple documentation fixes and improvements ([#1435], [#1446])

Thanks to @Folyd, @teozkr, and @dvdplm for contributing to this release!

[#1413]: https://github.com/tokio-rs/tracing/pull/1413
[#1456]: https://github.com/tokio-rs/tracing/pull/1456
[#1385]: https://github.com/tokio-rs/tracing/pull/1385
[#1435]: https://github.com/tokio-rs/tracing/pull/1435
[#1446]: https://github.com/tokio-rs/tracing/pull/1446

# 0.1.18 (April 30, 2021)

### Added

- `Subscriber` impl for `Box<dyn Subscriber + Send + Sync + 'static>` ([#1358])
- `Subscriber` impl for `Arc<dyn Subscriber + Send + Sync + 'static>` ([#1374])
- Symmetric `From` impls for existing `Into` impls on `Current` and `Option<Id>`
  ([#1335])
- `Attributes::fields` accessor that returns the set of fields defined on a
  span's `Attributes` ([#1331])


Thanks to @Folyd for contributing to this release!

[#1358]: https://github.com/tokio-rs/tracing/pull/1358
[#1374]: https://github.com/tokio-rs/tracing/pull/1374
[#1335]: https://github.com/tokio-rs/tracing/pull/1335
[#1331]: https://github.com/tokio-rs/tracing/pull/1331

# 0.1.17 (September 28, 2020)

### Fixed

- Incorrect inlining of `Event::dispatch` and `Event::child_of`, which could
  result in `dispatcher::get_default` being inlined at the callsite ([#994])

### Added

- `Copy` implementations for `Level` and `LevelFilter` ([#992])

Thanks to new contributors @jyn514 and @TaKO8Ki for contributing to this 
release!

[#994]: https://github.com/tokio-rs/tracing/pull/994
[#992]: https://github.com/tokio-rs/tracing/pull/992

# 0.1.16 (September 8, 2020)

### Fixed

- Added a conversion from `Option<Level>` to `LevelFilter`. This resolves a
  previously unreported regression where `Option<Level>` was no longer
  a valid LevelFilter. ([#966](https://github.com/tokio-rs/tracing/pull/966))

# 0.1.15 (August 22, 2020)

### Fixed

- When combining `Interest` from multiple subscribers, if the interests differ,
  the current subscriber is now always asked if a callsite should be enabled
  (#927)

## Added

- Internal API changes to support optimizations in the `tracing` crate (#943)
- **docs**: Multiple fixes and improvements (#913, #941)

# 0.1.14 (August 10, 2020)

### Fixed

- Incorrect calculation of global max level filter which could result in fast
  filtering paths not being taken (#908)
  
# 0.1.13 (August 4, 2020)

### Fixed

- Missing `fmt::Display` impl for `field::DisplayValue` causing a compilation
  failure when the "log" feature is enabled (#887)
  
Thanks to @d-e-s-o for contributing to this release!

# 0.1.12 (July 31, 2020)

### Added

- `LevelFilter` type and `LevelFilter::current()` for returning the highest level
  that any subscriber will enable (#853)
- `Subscriber::max_level_hint` optional trait method, for setting the value
  returned by `LevelFilter::current()` (#853)
  
### Fixed

- **docs**: Removed outdated reference to a Tokio API that no longer exists
  (#857)

Thanks to new contributor @dignati for contributing to this release!

# 0.1.11 (June 8, 2020)

### Changed

- Replaced use of `inner_local_macros` with `$crate::` (#729)

### Added

- `must_use` warning to guards returned by `dispatcher::set_default` (#686)
- `fmt::Debug` impl to `dyn Value`s (#696) 
- Functions to convert between `span::Id` and `NonZeroU64` (#770)
- More obvious warnings in documentation (#769)

### Fixed

- Compiler error when `tracing-core/std` feature is enabled but `tracing/std` is
  not (#760)
- Clippy warning on vtable address comparison in `callsite::Identifier` (#749)
- Documentation formatting issues (#715, #771)

Thanks to @bkchr, @majecty, @taiki-e, @nagisa, and @nvzqz for contributing to
this release!

# 0.1.10 (January 24, 2020)

### Added

- `field::Empty` type for declaring empty fields whose values will be recorded
  later (#548)
- `field::Value` implementations for `Wrapping` and `NonZero*` numbers (#538)

### Fixed

- Broken and unresolvable links in RustDoc (#595)

Thanks to @oli-cosmian for contributing to this release!

# 0.1.9 (January 10, 2020)

### Added

- API docs now show what feature flags are required to enable each item (#523)

### Fixed

- A panic when the current default subscriber subscriber calls
  `dispatcher::with_default` as it is being dropped (#522)
- Incorrect documentation for `Subscriber::drop_span` (#524)

# 0.1.8 (December 20, 2019)

### Added

- `Default` impl for `Dispatch` (#411)

### Fixed

- Removed duplicate `lazy_static` dependencies (#424)
- Fixed no-std dependencies being enabled even when `std` feature flag is set
  (#424)
- Broken link to `Metadata` in `Event` docs (#461)

# 0.1.7 (October 18, 2019)

### Added

- Added `dispatcher::set_default` API which returns a drop guard (#388)

### Fixed

- Added missing `Value` impl for `u8` (#392)
- Broken links in docs.

# 0.1.6 (September 12, 2019)

### Added

- Internal APIs to support performance optimizations (#326)

### Fixed

- Clarified wording in `field::display` documentation (#340)

# 0.1.5 (August 16, 2019)

### Added

- `std::error::Error` as a new primitive `Value` type (#277)
- `Event::new` and `Event::new_child_of` to manually construct `Event`s (#281)

# 0.1.4 (August 9, 2019)

### Added

- Support for `no-std` + `liballoc` (#256)

### Fixed

- Broken links in RustDoc (#259)

# 0.1.3 (August 8, 2019)

### Added

- `std::fmt::Display` implementation for `Level` (#194)
- `std::str::FromStr` implementation for `Level` (#195)

# 0.1.2 (July 10, 2019)

### Deprecated

- `Subscriber::drop_span` in favor of new `Subscriber::try_close` (#168)

### Added

- `Into<Option<&Id>>`, `Into<Option<Id>>`, and
  `Into<Option<&'static Metadata<'static>>>` impls for `span::Current` (#170)
- `Subscriber::try_close` method (#153)
- Improved documentation for `dispatcher` (#171)

# 0.1.1 (July 6, 2019)

### Added

- `Subscriber::current_span` API to return the current span (#148).
- `span::Current` type, representing the `Subscriber`'s view of the current
  span (#148).

### Fixed

- Typos and broken links in documentation (#123, #124, #128, #154)

# 0.1.0 (June 27, 2019)

- Initial release
