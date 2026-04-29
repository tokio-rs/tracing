# 0.3.2 (November 26, 2025)

### Added

- Use argv0 for syslog identifier ([#3372])

[#3372]: https://github.com/tokio-rs/tracing/pull/#3372

# 0.3.1 (November 29, 2024)

[ [crates.io][crate-0.3.1] ] | [ [docs.rs][docs-0.3.1] ]

### Changed

- disable default features of tracing-subscriber ([#1476])
- allow custom journal fields ([#2708])
- Bump MSRV to 1.63 ([#2793])
- make level mappings configurable ([#2824])

[#1476]: https://github.com/tokio-rs/tracing/pull/1476
[#2708]: https://github.com/tokio-rs/tracing/pull/2708
[#2793]: https://github.com/tokio-rs/tracing/pull/2793
[#2824]: https://github.com/tokio-rs/tracing/pull/2824
[docs-0.3.1]: https://docs.rs/tracing-journald/0.3.1
[crate-0.3.1]: https://crates.io/crates/tracing-journald/0.3.1

# 0.3.0 (April 21, 2022)

This is a breaking release which changes the format in which span fields
are output to `journald`. Previously, span field names were prefixed with the
depth of the span in the current trace tree. However, these prefixes are
unnecessary, as `journald` has built in support for duplicated field names.

See PR [#1986] for details on this change.

## Changed

- Removed span field prefixes ([#1986])
- Renamed `S{num}_NAME` fields to `SPAN_NAME` ([#1986])

### Fixed

- Fixed broken links in documentation ([#2077])

Thanks to @wiktorsikora and @ben0x539 for contributing to this release!

[#1986]: https://github.com/tokio-rs/tracing/pull/1986
[#2077]: https://github.com/tokio-rs/tracing/pull/2077

# 0.2.4 (March 17, 2022)

### Fixed

- Fixed compilation error in `memfd_create_syscall` on 32-bit targets ([#1982])

Thanks to new contributor @chrta for contributing to this release!


[#1982]: https://github.com/tokio-rs/tracing/pull/1982

# 0.2.3 (February 7, 2022)

### Fixed

- Fixed missing `memfd_create` with `glibc` versions < 2.25 ([#1912])

### Changed

- Updated minimum supported Rust version to 1.49.0 ([#1913])

Thanks to @9999years for contributing to this release!

[#1912]: https://github.com/tokio-rs/tracing/pull/1912
[#1913]: https://github.com/tokio-rs/tracing/pull/1913

# 0.2.2 (January 14, 2022)
### Added

- Include a syslog identifier in log messages ([#1822])
- Added `Layer::with_syslog_identifier` method to override the syslog identifier
  ([#1822])

Thanks to @lunaryorn for contributing to this release!

[#1822]: https://github.com/tokio-rs/tracing/pull/1822

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
