# 0.1.0-beta.3 (November 28, 2025)

#### Important

The previous release [0.1.0-beta.2] was yanked as it depended explicitly on
[tracing-0.1.42], which was yanked due to a breaking change (see [#3424] for
details). This release contains all the changes from the previous release, plus
an update to the newer version of `tracing`.

### Changed

- `tracing`: updated to 0.1.43 ([#3427])

[#3424]: https://github.com/tokio-rs/tracing/pull/3424
[#3427]: https://github.com/tokio-rs/tracing/pull/3427
[0.1.0-beta.2]: https://github.com/tokio-rs/tracing/releases/tag/tracing-mock-0.1.0-beta.2
[tracing-0.1.42]: https://github.com/tokio-rs/tracing/releases/tag/tracing-0.1.42

# 0.1.0-beta.2 (November 26, 2025)

### Added

- Add `on_register_dispatch` expectation for subscriber and layer mocks ([#3415])
- Add doctests for `on_register_dispatch` negative cases ([#3416])

### Changed

- `tracing`: updated to 0.1.42 ([#3418])

[#3415]: https://github.com/tokio-rs/tracing/pull/3415
[#3416]: https://github.com/tokio-rs/tracing/pull/3416
[#3418]: https://github.com/tokio-rs/tracing/pull/3418

# 0.1.0-beta.1 (November 29, 2024)

[ [crates.io][crate-0.1.0-beta.1] ] | [ [docs.rs][docs-0.1.0-beta.1] ]

`tracing-mock` provides tools for making assertions about what `tracing`
diagnostics are emitted by code under test.

- Initial beta release

[docs-0.1.0-beta.1]: https://docs.rs/tracing-mock/0.1.0-beta.1
[crate-0.1.0-beta.1]: https://crates.io/crates/tracing-mock/0.1.0-beta.1
