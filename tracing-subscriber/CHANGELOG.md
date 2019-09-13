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
