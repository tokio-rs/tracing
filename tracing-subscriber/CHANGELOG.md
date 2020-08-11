# 0.2.11 (August 10, 2020)

### Fixed

- **env-filter**: Incorrect max level hint when filters involving span field
  values are in use (#907)
- **registry**: Fixed inconsistent span stacks when multiple registries are in
  use on the same thread (#901)

### Changed

- **env-filter**: `regex` dependency enables fewer unused feature flags (#899)

Thanks to @bdonlan and @jeromegn for contributing to this release!

# 0.2.10 (July 31, 2020)

### Fixed

- **docs**: Incorrect formatting (#862)

### Changed

- **filter**: `LevelFilter` is now a re-export of the
  `tracing_core::LevelFilter` type, it can now be used interchangably with the
  versions in `tracing` and `tracing-core` (#853)
- **filter**: Significant performance improvements when comparing `LevelFilter`s
  and `Level`s (#853)
- Updated the minimum `tracing-core` dependency to 0.1.12 (#853)

### Added

- **filter**: `LevelFilter` and `EnvFilter` now participate in `tracing-core`'s
  max level hinting, improving performance significantly in some use cases where
  levels are disabled globally (#853)

# 0.2.9 (July 23, 2020)

### Fixed

- **fmt**: Fixed compilation failure on MSRV when the `chrono` feature is
  disabled (#844)

### Added

- **fmt**: Span lookup methods defined by `layer::Context` are now also provided
  by `FmtContext` (#834)

# 0.2.8 (July 17, 2020)

### Changed

- **fmt**: When the `chrono` dependency is enabled, the `SystemTime` timestamp
  formatter now emits human-readable timestamps rather than using `SystemTime`'s
  `fmt::Debug`implementation (`chrono` is still required for customized
  timestamp formatting) (#807)
- **ansi**: Updated `ansi_term` dependency to 0.12 (#816)

### Added

- **json**: `with_span_list` method to configure the JSON formatter to include a
  list of all spans in the current trace in formatting events (similarly to the
  text formatter) (#741) 
- **json**: `with_current_span` method to configure the JSON formatter to include
  a field for the _current_ span (the leaf of the trace) in formatted events
  (#741)
- **fmt**: `with_thread_names` and `with_thread_ids` methods to configure
  `fmt::Subscriber`s and `fmt::Layer`s to include the thread name and/or thread ID
  of the current thread when formatting events (#818)
  
Thanks to new contributors @mockersf, @keetonian, and @Pothulapati for
contributing to this release!

# 0.2.7 (July 1, 2020)

### Changed

- **parking_lot**: Updated the optional `parking_lot` dependency to accept the
  latest `parking_lot` version (#774)

### Fixed

- **fmt**: Fixed events with explicitly overridden parent spans being formatted
  as though they were children of the current span (#767)

### Added

- **fmt**: Added the option to print synthesized events when spans are created,
  entered, exited, and closed, including span durations (#761)
- Documentation clarification and improvement (#762, #769)

Thanks to @rkuhn, @greenwoodcm, and @Ralith for contributing to this release!

# 0.2.6 (June 19, 2020)

### Fixed

- **fmt**: Fixed an issue in the JSON formatter where using `Span::record` would
  result in malformed spans (#709)

# 0.2.5 (April 21, 2020)

### Changed

- **fmt**: Bump sharded-slab dependency (#679)

### Fixed

- **fmt**: remove trailing space in `ChronoUtc` `format_time` (#677)

# 0.2.4 (April 6, 2020)

This release includes several API ergonomics improvements, including shorthand
constructors for many types, and an extension trait for initializing subscribers
using method-chaining style. Additionally, several bugs in less commonly used
`fmt` APIs were fixed.

### Added

- **fmt**: Shorthand free functions for constructing most types in `fmt`
  (including `tracing_subscriber::fmt()` to return a `SubscriberBuilder`,
  `tracing_subscriber::fmt::layer()` to return a format `Layer`, etc) (#660)
- **registry**: Shorthand free function `tracing_subscriber::registry()` to
  construct a new registry (#660)
- Added `SubscriberInitExt` extension trait for more ergonomic subscriber
  initialization (#660)
  
### Changed

- **fmt**: Moved `LayerBuilder` methods to `Layer` (#655)

### Deprecated

- **fmt**: `LayerBuilder`, as `Layer` now implements all builder methods (#655)
  
### Fixed

- **fmt**: Fixed `Compact` formatter not omitting levels with
  `with_level(false)` (#657)
- **fmt**: Fixed `fmt::Layer` duplicating the fields for a new span if another
  layer has already formatted its fields (#634)
- **fmt**: Added missing space when using `record` to add new fields to a span
  that already has fields (#659)
- Updated outdated documentation (#647)


# 0.2.3 (March 5, 2020)

### Fixed

- **env-filter**: Regression where filter directives were selected in the order
  they were listed, rather than most specific first (#624)

# 0.2.2 (February 27, 2020)

### Added

- **fmt**: Added `flatten_event` to `SubscriberBuilder` (#599)
- **fmt**: Added `with_level` to `SubscriberBuilder` (#594)

# 0.2.1 (February 13, 2020)

### Changed

- **filter**: `EnvFilter` directive selection now behaves correctly (i.e. like
  `env_logger`) (#583)

### Fixed

- **filter**: Fixed `EnvFilter` incorrectly allowing less-specific filter
  directives to enable events that are disabled by more-specific filters (#583)
- **filter**: Multiple significant `EnvFilter` performance improvements,
  especially when filtering events generated by `log` records (#578, #583)
- **filter**: Replaced `BTreeMap` with `Vec` in `DirectiveSet`, improving
  iteration performance significantly with typical numbers of filter directives
  (#580)

A big thank-you to @samschlegel for lots of help with `EnvFilter` performance
tuning in this release!

# 0.2.0 (February 4, 2020)

### Breaking Changes

- **fmt**: Renamed `Context` to `FmtContext` (#420, #425)
- **fmt**: Renamed `Builder` to `SubscriberBuilder` (#420)
- **filter**: Removed `Filter`. Use `EnvFilter` instead (#434)

### Added

- **registry**: `Registry`, a `Subscriber` implementation that `Layer`s can use
  as a high-performance, in-memory span store. (#420, #425, #432, #433, #435)
- **registry**: Added `LookupSpan` trait, implemented by `Subscriber`s to expose
  stored span data to `Layer`s (#420)
- **fmt**: Added `fmt::Layer`, to allow composing log formatting with other `Layer`s
- **fmt**: Added support for JSON field and event formatting (#377, #415)
- **filter**: Documentation for filtering directives (#554)

### Changed

- **fmt**: Renamed `Context` to `FmtContext` (#420, #425) (BREAKING)
- **fmt**: Renamed `Builder` to `SubscriberBuilder` (#420) (BREAKING)
- **fmt**: Reimplemented `fmt::Subscriber` in terms of the `Registry`
  and `Layer`s (#420)

### Removed

- **filter**: Removed `Filter`. Use `EnvFilter` instead (#434) (BREAKING)

### Fixed

- **fmt**: Fixed memory leaks in the slab used to store per-span data
  (3c35048)
- **fmt**: `fmt::SubscriberBuilder::init` not setting up `log` compatibility
  (#489)
- **fmt**: Spans closed by a child span closing not also closing _their_
  parents (#514)
- **Layer**: Fixed `Layered` subscribers failing to downcast to their own type
  (#549)
- **Layer**: Fixed `Layer::downcast_ref` returning invalid references (#454)

# 0.2.0-alpha.6 (February 3, 2020)

### Fixed

- **fmt**: Fixed empty `{}` printed after spans with no fields (f079f2d)
- **fmt**: Fixed inconsistent formatting when ANSI colors are disabled (506a482)
- **fmt**: Fixed mis-aligned levels when ANSI colors are disabled (eba1adb)
- Fixed warnings on nightly Rust compilers (#558)

# 0.2.0-alpha.5 (January 31, 2020)

### Added

- **env_filter**: Documentation for filtering directives (#554)
- **registry**, **env_filter**: Updated `smallvec` dependency to 0.1 (#543)

### Fixed

- **registry**: Fixed a memory leak in the slab used to store per-span data
  (3c35048)
- **Layer**: Fixed `Layered` subscribers failing to downcast to their own type
  (#549)
- **fmt**: Fixed a panic when multiple layers insert `FormattedFields`
  extensions from the same formatter type (1c3bb70)
- **fmt**: Fixed `fmt::Layer::on_record` inserting a new `FormattedFields` when
  formatted fields for a span already exist (1c3bb70)

# 0.2.0-alpha.4 (January 11, 2020)

### Fixed

- **registry**: Removed inadvertently committed `dbg!` macros (#533)

# 0.2.0-alpha.3 (January 10, 2020)

### Added

- **fmt**: Public `FormattedFields::new` constructor (#478)
- **fmt**: Added examples to `fmt::Layer` documentation (#510)
- Documentation now shows what feature flags are required by each API item (#525)

### Fixed

- **fmt**: Missing space between timestamp and level (#480)
- **fmt**: Incorrect formatting with `with_target(false)` (#481)
- **fmt**: `fmt::SubscriberBuilder::init` not setting up `log` compatibility
  (#489)
- **registry**: Spans exited out of order not being closed properly on exit
  (#509)
- **registry**: Memory leak when spans are closed by a child span closing (#514)
- **registry**: Spans closed by a child span closing not also closing _their_
  parents (#514)
- Compilation errors with `no-default-features` (#499, #500)

# 0.2.0-alpha.2 (December 8, 2019)

### Added

- `LookupSpans` implementation for `Layered` (#448)
- `SpanRef::from_root` to iterate over a span's parents from the root (#460)
- `Context::scope`, to iterate over the current context from the root (#460)
- `Context::lookup_current`, which returns a `SpanRef` to the current
  span's data (#460)

### Changed

- Lifetimes on some new `Context` methods to be less restrictive (#460)

### Fixed

- `Layer::downcast_ref` returning invalid references (#454)
- Compilation failure on 32-bit platforms (#462)
- Compilation failure with ANSI formatters (#438)

# 0.2.0-alpha.1 (November 18, 2019)

### Added

- `Registry`, a reusable span store that `Layer`s can use a
  high-performance, in-memory store. (#420, #425, #432, #433, #435)
- Reimplemented `fmt::Subscriber` in terms of the `Registry`
  and `Layer`s (#420)
- Add benchmarks for fmt subscriber (#421)
- Add support for JSON field and event formatting (#377, #415)

### Changed

- **BREAKING**: Change `fmt::format::FormatFields` and
  `fmt::format::FormatEvent` to accept a mandatory `FmtContext`. These
  `FormatFields` and `FormatEvent` will likely see additional breaking
  changes in subsequent alpha. (#420, #425)
- **BREAKING**: Removed `Filter`. Use `EnvFilter` instead (#434)

### Contributers

Thanks to all the contributers to this release!

- @pimeys for #377 and #415

# 0.1.6 (October 29, 2019)

### Added

- Add `init` and `try_init` functions to `FmtSubscriber` (#385)
- Add `ChronoUtc` and `ChronoLocal` timers, RFC 3339 support (#387)
- Add `tracing::subscriber::set_default` which sets the default
  subscriber and returns a drop guard. This drop guard will reset the
  dispatch on drop (#388).

### Fixed

- Fix default level for `EnvFilter`. Setting `RUST_LOG=target`
  previously only the `ERROR` level, while it should enable everything.
  `tracing-subscriber` now defaults to `TRACE` if no level is specified
  (#401)
- Fix `tracing-log` feature flag for init + try_init. The feature flag
  `tracing_log` was used instead of the correct `tracing-log`. As a
  result, both `tracing-log` and `tracing_log` needed to be specified in
  order to initialize the global logger. Only `tracing-log` needs to be
  specified now (#400).

### Contributers

Thanks to all the contributers to this release!

- @emschwartz for #385, #387, #400 and #401
- @bIgBV for #388

# 0.1.5 (October 7, 2019)

### Fixed

- Spans not being closed properly when `FmtSubscriber::current_span` is used
  (#371)

# 0.1.4 (September 26, 2019)

### Fixed

- Spans entered twice on the same thread sometimes being completely exited when
  the more deeply-nested entry is exited (#361)
- Setting `with_ansi(false)` on `FmtSubscriber` not disabling ANSI color
  formatting for timestamps (#354)
- Incorrect reference counting in `FmtSubscriber` that could cause spans to not
  be closed when all references are dropped (#366)

# 0.1.3 (September 16, 2019)

### Fixed

- `Layered` subscribers not properly forwarding calls to `current_span`
  (#350)

# 0.1.2 (September 12, 2019)

### Fixed

- `EnvFilter` ignoring directives with targets that are the same number of
  characters (#333)
- `EnvFilter` failing to properly apply filter directives to events generated
  from `log` records by`tracing-log` (#344)

### Changed

- Renamed `Filter` to `EnvFilter`, deprecated `Filter` (#339)
- Renamed "filter" feature flag to "env-filter", deprecated "filter" (#339)
- `FmtSubscriber` now defaults to enabling only the `INFO` level and above when
  a max level filter or `EnvFilter` is not set (#336)
- Made `parking_lot` dependency an opt-in feature flag (#348)

### Added

- `EnvFilter::add_directive` to add new directives to filters after they are
  constructed (#334)
- `fmt::Builder::with_max_level` to set a global level filter for a
  `FmtSubscriber` without requiring the use of `EnvFilter` (#336)
- `Layer` implementation for `LevelFilter` (#336)
- `EnvFilter` now implements `fmt::Display` (#329)

### Removed

- Removed dependency on `crossbeam-util` (#348)

# 0.1.1 (September 4, 2019)

### Fixed

- Potential double panic in `CurrentSpan` (#325)

# 0.1.0 (September 3, 2019)

- Initial release
