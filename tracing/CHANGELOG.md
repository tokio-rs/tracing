# 0.1.5 (August 9, 2019)

### Added

- Support for `no-std` + `liballoc` (#263)

### Changed

- Using the `#[instrument]` attribute on `async fn`s no longer requires a
  feature flag (#258)

### Fixed

- The `#[instrument]` macro now works on generic functions (#262)

# 0.1.4 (August 8, 2019)

### Added

- `#[instrument]` attribute for automatically adding spans to functions (#253)

# 0.1.3 (July 11, 2019)

### Added

- Log messages when a subscriber indicates that a span has closed, when the
  `log` feature flag is enabled (#180).

### Changed

- `tracing-core` minimum dependency version to 0.1.2 (#174).

### Fixed

- Fixed an issue where event macro invocations with a single field, using local
  variable shorthand, would recur infinitely (#166).
- Fixed uses of deprecated `tracing-core` APIs (#174).

# 0.1.2 (July 6, 2019)

### Added

- `Span::none()` constructor, which does not require metadata and
  returns a completely empty span (#147).
- `Span::current()` function, returning the current span if it is
  known to the subscriber (#148).

### Fixed

- Broken macro imports when used prefixed with `tracing::` (#152).

# 0.1.1 (July 3, 2019)

### Changed

- `cfg_if` dependency to 0.1.9.

### Fixed

- Compilation errors when the `log` feature is enabled (#131).
- Unclear wording and typos in documentation (#124, #128, #142).

# 0.1.0 (June 27, 2019)

- Initial release
