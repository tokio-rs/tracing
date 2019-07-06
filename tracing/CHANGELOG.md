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
