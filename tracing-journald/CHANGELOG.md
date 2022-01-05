# 0.2.1 (December 29, 2021)

This release improves how `tracing-journald` communicates with `journald`,
including the handling of large payloads.

### Added

- Use an unconnected socket, so that logging can resume after a `journald`
  restart ([#1758])

### Fixed

- Fixed string values being written using `fmt::Debug` ([#1714])
- Fixed `EMSGSIZE` when log entries exceed a certain size ([#1744])

A huge thank-you to new contributor @lunaryorn, for contributing all of the
changes in this release!

[#1714]: https://github.com/tokio-rs/tracing/pull/1714
[#1744]: https://github.com/tokio-rs/tracing/pull/1744
[#1758]: https://github.com/tokio-rs/tracing/pull/1758

# 0.2.0 (October 22nd, 2021)

### Changed

- Updated `tracing-subscriber` dependency to 0.3.0 ([#1677])

[#1677]: https://github.com/tokio-rs/tracing/pull/1677
# 0.1.0 (June 29, 2020)

- Initial release