# 0.4.0 (May 12, 2020)

### Added

- `tracing_opentelemetry::layer()` method to construct a default layer.
- `OpenTelemetryLayer::with_sampler` method to configure the opentelemetry
  sampling behavior.
- `OpenTelemetryLayer::new` method to configure both the tracer and sampler.

### Breaking Changes

- `OpenTelemetrySpanExt::set_parent` now accepts a reference to an extracted
  parent `Context` instead of a `SpanContext` to match propagators.
- `OpenTelemetrySpanExt::context` now returns a `Context` instead of a
  `SpanContext` to match propagators.
- `OpenTelemetryLayer::with_tracer` now takes `&self` as a parameter
- Upgrade to `v0.5.0` of `opentelemetry`.

### Fixed

- Fixes bug where child spans were always marked as sampled

# 0.3.1 (April 19, 2020)

### Added

- Change span status code to unknown on error event

# 0.3.0 (April 5, 2020)

### Added

- Span extension for injecting and extracting `opentelemetry` span contexts
  into `tracing` spans

### Removed

- Disabled the `metrics` feature of the opentelemetry as it is unused.

# 0.2.0 (February 7, 2020)

### Changed

- Update `tracing-subscriber` to 0.2.0 stable
- Update to `opentelemetry` 0.2.0
