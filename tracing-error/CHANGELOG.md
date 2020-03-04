# 0.1.2 (March 3, 2020)

### Added

- **TracedError**: `TracedError`, an error type wrapper that annotates an error
  with the current span.
- **SpanTrace**:`SpanTrace::status` method and `SpanTraceStatus` type for
  determing whether a `SpanTrace` was successfully captured (#614)

### Changed

- **SpanTrace**: Made backtrace formatting more consistent with upstream changes
  to `std::backtrace` (#584)

# 0.1.1 (February 5, 2020)

### Fixed

- Fixed a typo in the crate description

### Changed

- the maintenance badge from active to experimental

# 0.1.0 (February 5, 2020)

- Initial release
