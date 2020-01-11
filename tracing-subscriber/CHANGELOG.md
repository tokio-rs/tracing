# 0.2.0-alpha.4 (January 11, 2020)

### Fixed

- **registry**: Removed inadvertantly committed `dbg!` macros (#533)

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
