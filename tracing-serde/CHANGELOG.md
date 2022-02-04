# 0.1.3 (February 4, 2022)

This release adds *experimental* support for recording structured field
values using the [`valuable`] crate. See [this blog post][post] for
details on `valuable`.

Note that `valuable` support currently requires `--cfg
tracing_unstable`. See the documentation for details.

### Added

- **valuable**: Experimental support for serializing user-defined types using
  [`valuable`] and [`valuable-serde`] ([#1862])
- Support for serializing `f64` values ([#1507])

### Fixed

- Fixed incorrect size hint in `SerializeFieldSet` ([#1333])
- A number of documentation fixes

Thanks to @akinnane and @maxburke for contributing to this release!

[`valuable`]: https://crates.io/crates/valuable
[`valuable-serde`]: https://crates.io/crates/valuable-serde
[post]: https://tokio.rs/blog/2021-05-valuable
[#1862]: https://github.com/tokio-rs/tracing/pull/1862
[#1507]: https://github.com/tokio-rs/tracing/pull/1507
[#1333]: https://github.com/tokio-rs/tracing/pull/1333

# 0.1.2 (September 11, 2020)

### Added

- `SerdeMapVisitor::finish` to complete serializing the visited objects
  (#892)
- `SerdeMapVisitor::take_serializer` to return the serializer wrapped by
  a `SerdeMapVisitor` (#892)

# 0.1.1 (February 27, 2020)

### Added

- Made `SerdeMapVisitor` public (#599)
- Made `SerdeStructVisitor` public (#599)

# 0.1.0 (November 18, 2019)

- Initial release
