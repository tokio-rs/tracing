# 0.1.10 (January 24, 2020)

### Added

- `field::Empty` type for declaring empty fields whose values will be recorded
  later (#548)
- `field::Value` implementations for `Wrapping` and `NonZero*` numbers (#538)

### Fixed

- Broken and unresolvable links in RustDoc (#595)

Thanks to @oli-cosmian for contributing to this release!

# 0.1.9 (January 10, 2020)

### Added

- API docs now show what feature flags are required to enable each item (#523)

### Fixed

- A panic when the current default subscriber subscriber calls
  `dispatcher::with_default` as it is being dropped (#522)
- Incorrect documentation for `Subscriber::drop_span` (#524)

# 0.1.8 (December 20, 2019)

### Added

- `Default` impl for `Dispatch` (#411)

### Fixed

- Removed duplicate `lazy_static` dependencies (#424)
- Fixed no-std dependencies being enabled even when `std` feature flag is set
  (#424)
- Broken link to `Metadata` in `Event` docs (#461)

# 0.1.7 (October 18, 2019)

### Added

- Added `dispatcher::set_default` API which returns a drop guard (#388)

### Fixed

- Added missing `Value` impl for `u8` (#392)
- Broken links in docs.

# 0.1.7 (October 18, 2019)

### Added

- Added `dispatcher::set_default` API which returns a drop guard (#388)

### Fixed

- Added missing `Value` impl for `u8` (#392)
- Broken links in docs.

# 0.1.6 (September 12, 2019)

### Added

- Internal APIs to support performance optimizations (#326)

### Fixed

- Clarified wording in `field::display` documentation (#340)

# 0.1.6 (August 16, 2019)

### Added

- `std::error::Error` as a new primitive `Value` type (#277)
- `Event::new` and `Event::new_child_of` to manually construct `Event`s (#281)

# 0.1.4 (August 9, 2019)

### Added

- Support for `no-std` + `liballoc` (#256)

### Fixed

- Broken links in RustDoc (#259)

# 0.1.3 (August 8, 2019)

### Added

- `std::fmt::Display` implementation for `Level` (#194)
- `std::str::FromStr` implementation for `Level` (#195)

# 0.1.2 (July 10, 2019)

### Deprecated

- `Subscriber::drop_span` in favor of new `Subscriber::try_close` (#168)

### Added

- `Into<Option<&Id>>`, `Into<Option<Id>>`, and
  `Into<Option<&'static Metadata<'static>>>` impls for `span::Current` (#170)
- `Subscriber::try_close` method (#153)
- Improved documentation for `dispatcher` (#171)

# 0.1.1 (July 6, 2019)

### Added

- `Subscriber::current_span` API to return the current span (#148).
- `span::Current` type, representing the `Subscriber`'s view of the current
  span (#148).

### Fixed

- Typos and broken links in documentation (#123, #124, #128, #154)

# 0.1.0 (June 27, 2019)

- Initial release
