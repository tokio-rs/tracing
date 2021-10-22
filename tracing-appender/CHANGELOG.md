# 0.2.0 (October 22, 2021)

This breaking change release adds support for the new v0.3.x series of
`tracing-subscriber`. In addition, it resolves the security advisory for the
`chrono` crate, [RUSTSEC-2020-0159].

This release increases the minimum supported Rust version to 1.51.0.
### Breaking Changes

- Updated `tracing-subscriber` to v0.3.x ([#1677])
- Changed `NonBlocking::error_counter` to return an `ErrorCounter` type, rather
  than an `Arc<AtomicU64>` ([#1675])
### Changed

- Updated `tracing-subscriber` to v0.3.x ([#1677])
### Fixed

- **non-blocking**: Fixed compilation on 32-bit targets ([#1675])
- **rolling**: Replaced `chrono` dependency with `time` to resolve
  [RUSTSEC-2020-0159] ([#1652])
- **rolling**: Fixed an issue where `RollingFileAppender` would fail to print
  errors that occurred while flushing a previous logfile ([#1604])

Thanks to new contributors @dzvon and @zvkemp for contributing to this release!

[RUSTSEC-2020-0159]: https://rustsec.org/advisories/RUSTSEC-2020-0159.html
[#1677]: https://github.com/tokio-rs/tracing/pull/1677
[#1675]: https://github.com/tokio-rs/tracing/pull/1675
[#1652]: https://github.com/tokio-rs/tracing/pull/1675
[#1604]: https://github.com/tokio-rs/tracing/pull/1604

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