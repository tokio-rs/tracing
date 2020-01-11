# 0.2.1

- docs: add `doc-cfg` attributes to tracing, tracing-core, tracing-futures, tracing-subscriber (#523, #525) 
- `no_std` support (#498)

# 0.2.0 (Dec 3, 2019)

### Changed

- **Breaking Change**: the default `Future` implementation comes from the `std-future` feature.
  Compatibility with futures v0.1 is available via the `futures-01` feature.

# 0.1.1 (Oct 25, 2019)

### Added

- `Instrumented::inner` and `inner_mut` methods that expose access to the
  instrumented future (#386)

# 0.1.0 (Oct 8, 2019)

- Initial release
