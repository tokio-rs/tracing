# 0.1.2 (December 28, 2020)

### Changed

- **non_blocking**: Updated `crossbeam-channel` dependency to 0.5 (#1031)

### Fixed

- **non_blocking**: Fixed a race condition when logging on shutdown (#1125)
- Several documentation improvements (#1109, #1110, #941, #953)

# 0.1.1 (July 20, 2020)

### Added

- **rolling**: `minutely` rotation schedule to rotate the log file once every
  minute (#748)

### Fixed

- Fixed broken links in docs (#718)
- `tracing-appender` now only enables the necessary `tracing-subscriber`'s
  feature flags, rather than all of them (#779) 

Thanks to new contributors @ericjheinz and @sourcefrog for contributing
to this release!

# 0.1.0 (May 5, 2020)

- Initial release