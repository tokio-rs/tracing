# 0.1.35 (June 8, 2022)

This release reduces the overhead of callsite registration by using new
`tracing-core` APIs.

### Added

- Use `DefaultCallsite` to reduce callsite registration overhead ([#2083])

### Changed

- `tracing-core`: updated to [0.1.27][core-0.1.27]

[core-0.1.27]: https://github.com/tokio-rs/tracing/releases/tag/tracing-core-0.1.27
[#2088]: https://github.com/tokio-rs/tracing/pull/2083

# 0.1.34 (April 14, 2022)

This release includes bug fixes for the "log" support feature and for the use of
both scoped and global default dispatchers in the same program.

### Fixed

- Failure to use the global default dispatcher when a thread sets a local
  default dispatcher before the global default is set ([#2065])
- **log**: Compilation errors due to `async` block/fn futures becoming `!Send`
  when the "log" feature flag is enabled ([#2073])
- Broken links in documentation ([#2068])

Thanks to @ben0x539 for contributing to this release!

[#2065]: https://github.com/tokio-rs/tracing/pull/2065
[#2073]: https://github.com/tokio-rs/tracing/pull/2073
[#2068]: https://github.com/tokio-rs/tracing/pull/2068

# 0.1.33 (April 9, 2022)

This release adds new `span_enabled!` and `event_enabled!` variants of the
`enabled!` macro, for testing whether a subscriber would specifically enable a
span or an event.

### Added

- `span_enabled!` and `event_enabled!` macros ([#1900])
- Several documentation improvements ([#2010], [#2012])

### Fixed

- Compilation warning when compiling for <=32-bit targets (including `wasm32`)
  ([#2060])

Thanks to @guswynn, @arifd, @hrxi, @CAD97, and @name1e5s for contributing to
this release!

[#1900]: https://github.com/tokio-rs/tracing/pull/1900
[#2010]: https://github.com/tokio-rs/tracing/pull/2010
[#2012]: https://github.com/tokio-rs/tracing/pull/2012
[#2060]: https://github.com/tokio-rs/tracing/pull/2060

# 0.1.32 (March 8th, 2022)

This release reduces the overhead of creating and dropping disabled
spans significantly, which should improve performance when no `tracing`
subscriber is in use or when spans are disabled by a filter.

### Fixed

- **attributes**: Compilation failure with `--minimal-versions` due to a
  too-permissive `syn` dependency ([#1960])

### Changed

- Reduced `Drop` overhead for disabled spans ([#1974])
- `tracing-attributes`: updated to [0.1.20][attributes-0.1.20]

[#1974]: https://github.com/tokio-rs/tracing/pull/1974
[#1960]: https://github.com/tokio-rs/tracing/pull/1960
[attributes-0.1.20]: https://github.com/tokio-rs/tracing/releases/tag/tracing-attributes-0.1.20

# 0.1.31 (February 17th, 2022)

This release increases the minimum supported Rust version (MSRV) to 1.49.0. In
addition, it fixes some relatively rare macro bugs.

### Added

- Added `tracing-forest` to the list of related crates ([#1935])

### Changed

- Updated minimum supported Rust version (MSRV) to 1.49.0 ([#1913])

### Fixed

- Fixed the `warn!` macro incorrectly generating an event with the `TRACE` level
  ([#1930])
- Fixed macro hygiene issues when used in a crate that defines its own `concat!`
  macro, for real this time ([#1918])

Thanks to @QnnOkabayashi, @nicolaasg, and @teohhanhui for contributing to this
release!

[#1935]: https://github.com/tokio-rs/tracing/pull/1935
[#1913]: https://github.com/tokio-rs/tracing/pull/1913
[#1930]: https://github.com/tokio-rs/tracing/pull/1930
[#1918]: https://github.com/tokio-rs/tracing/pull/1918

# 0.1.30 (February 3rd, 2022)

This release adds *experimental* support for recording structured field
values using the [`valuable`] crate. See [this blog post][post] for
details on `valuable`.

Note that `valuable` support currently requires `--cfg tracing_unstable`. See
the documentation for details.

This release also adds a new `enabled!` macro for testing if a span or event
would be enabled.

### Added

- **field**: Experimental support for recording field values using the
  [`valuable`] crate ([#1608], [#1888], [#1887])
- `enabled!` macro for testing if a span or event is enabled ([#1882])

### Changed

- `tracing-core`: updated to [0.1.22][core-0.1.22]
- `tracing-attributes`: updated to [0.1.19][attributes-0.1.19]

### Fixed

- **log**: Fixed "use of moved value" compiler error when the "log" feature is
  enabled ([#1823])
- Fixed macro hygiene issues when used in a crate that defines its own `concat!`
  macro ([#1842])
- A very large number of documentation fixes and improvements.

Thanks to @@Vlad-Scherbina, @Skepfyr, @Swatinem, @guswynn, @teohhanhui,
@xd009642, @tobz, @d-e-s-o@0b01, and @nickelc for contributing to this release!

[`valuable`]: https://crates.io/crates/valuable
[post]: https://tokio.rs/blog/2021-05-valuable
[core-0.1.22]: https://github.com/tokio-rs/tracing/releases/tag/tracing-core-0.1.22
[attributes-0.1.19]: https://github.com/tokio-rs/tracing/releases/tag/tracing-attributes-0.1.19
[#1608]: https://github.com/tokio-rs/tracing/pull/1608
[#1888]: https://github.com/tokio-rs/tracing/pull/1888
[#1887]: https://github.com/tokio-rs/tracing/pull/1887
[#1882]: https://github.com/tokio-rs/tracing/pull/1882
[#1823]: https://github.com/tokio-rs/tracing/pull/1823
[#1842]: https://github.com/tokio-rs/tracing/pull/1842

# 0.1.29 (October 5th, 2021)

This release adds support for recording `Option<T> where T: Value` as typed
`tracing` field values. It also includes significant performance improvements
for functions annotated with the `#[instrument]` attribute when the generated
span is disabled.

### Changed

- `tracing-core`: updated to v0.1.21
- `tracing-attributes`: updated to v0.1.18

### Added

- **field**: `Value` impl for `Option<T> where T: Value` ([#1585])
- **attributes**: - improved performance when skipping `#[instrument]`-generated
  spans below the max level ([#1600], [#1605], [#1614], [#1616], [#1617])

### Fixed

- **instrument**: added missing `Future` implementation for `WithSubscriber`,
  making the `WithDispatch` extension trait actually useable ([#1602])
- Documentation fixes and improvements ([#1595], [#1601], [#1597])

Thanks to @brianburgers, @mattiast, @DCjanus, @oli-obk, and @matklad for
contributing to this release!

[#1585]: https://github.com/tokio-rs/tracing/pull/1585
[#1595]: https://github.com/tokio-rs/tracing/pull/1596
[#1597]: https://github.com/tokio-rs/tracing/pull/1597
[#1600]: https://github.com/tokio-rs/tracing/pull/1600
[#1601]: https://github.com/tokio-rs/tracing/pull/1601
[#1602]: https://github.com/tokio-rs/tracing/pull/1602
[#1605]: https://github.com/tokio-rs/tracing/pull/1605
[#1614]: https://github.com/tokio-rs/tracing/pull/1614
[#1616]: https://github.com/tokio-rs/tracing/pull/1616
[#1617]: https://github.com/tokio-rs/tracing/pull/1617

# 0.1.28 (September 17th, 2021)

This release fixes an issue where the RustDoc documentation was rendered
incorrectly. It doesn't include any actual code changes, and is very boring and
can be ignored.

### Fixed

- **docs**: Incorrect documentation rendering due to unclosed `<div>` tag
  ([#1572])

[#1572]: https://github.com/tokio-rs/tracing/pull/1572

# 0.1.27 (September 13, 2021)

This release adds a new [`Span::or_current`] method to aid in efficiently
propagating span contexts to spawned threads or tasks. Additionally, it updates
the [`tracing-core`] version to [0.1.20] and the [`tracing-attributes`] version to
[0.1.16], ensuring that a number of new features in those crates are present.

### Fixed

- **instrument**: Added missing `WithSubscriber` implementations for futures and
  other types ([#1424])

### Added

- `Span::or_current` method, to help with efficient span context propagation
  ([#1538])
- **attributes**: add `skip_all` option to `#[instrument]` ([#1548])
- **attributes**: record primitive types as primitive values rather than as
  `fmt::Debug` ([#1378])
- **core**: `NoSubscriber`, a no-op `Subscriber` implementation
  ([#1549])
- **core**: Added `Visit::record_f64` and support for recording floating-point
  values ([#1507], [#1522])
- A large number of documentation improvements and fixes ([#1369], [#1398],
  [#1435], [#1442], [#1524], [#1556])

Thanks to new contributors @dzvon and @mbergkvist, as well as @teozkr,
@maxburke, @LukeMathWalker, and @jsgf, for contributing to this
release!

[`Span::or_current`]: https://docs.rs/tracing/0.1.27/tracing/struct.Span.html#method.or_current
[`tracing-core`]: https://crates.io/crates/tracing-core
[`tracing-attributes`]: https://crates.io/crates/tracing-attributes
[`tracing-core`]: https://crates.io/crates/tracing-core
[0.1.20]: https://github.com/tokio-rs/tracing/releases/tag/tracing-core-0.1.20
[0.1.16]: https://github.com/tokio-rs/tracing/releases/tag/tracing-attributes-0.1.16
[#1424]: https://github.com/tokio-rs/tracing/pull/1424
[#1538]: https://github.com/tokio-rs/tracing/pull/1538
[#1548]: https://github.com/tokio-rs/tracing/pull/1548
[#1378]: https://github.com/tokio-rs/tracing/pull/1378
[#1507]: https://github.com/tokio-rs/tracing/pull/1507
[#1522]: https://github.com/tokio-rs/tracing/pull/1522
[#1369]: https://github.com/tokio-rs/tracing/pull/1369
[#1398]: https://github.com/tokio-rs/tracing/pull/1398
[#1435]: https://github.com/tokio-rs/tracing/pull/1435
[#1442]: https://github.com/tokio-rs/tracing/pull/1442
[#1524]: https://github.com/tokio-rs/tracing/pull/1524
[#1556]: https://github.com/tokio-rs/tracing/pull/1556

# 0.1.26 (April 30, 2021)

### Fixed

- **attributes**: Compatibility between `#[instrument]` and `async-trait`
  v0.1.43 and newer ([#1228])
- Several documentation fixes ([#1305], [#1344])
### Added

- `Subscriber` impl for `Box<dyn Subscriber + Send + Sync + 'static>` ([#1358])
- `Subscriber` impl for `Arc<dyn Subscriber + Send + Sync + 'static>` ([#1374])
- Symmetric `From` impls for existing `Into` impls on `span::Current`, `Span`,
  and `Option<Id>` ([#1335], [#1338])
- `From<EnteredSpan>` implementation for `Option<Id>`, allowing `EnteredSpan` to
  be used in a `span!` macro's `parent:` field ([#1325])
- `Attributes::fields` accessor that returns the set of fields defined on a
  span's `Attributes` ([#1331])


Thanks to @Folyd, @nightmared, and new contributors @rmsc and @Fishrock123 for
contributing to this release!

[#1227]: https://github.com/tokio-rs/tracing/pull/1228
[#1305]: https://github.com/tokio-rs/tracing/pull/1305
[#1325]: https://github.com/tokio-rs/tracing/pull/1325
[#1338]: https://github.com/tokio-rs/tracing/pull/1338
[#1344]: https://github.com/tokio-rs/tracing/pull/1344
[#1358]: https://github.com/tokio-rs/tracing/pull/1358
[#1374]: https://github.com/tokio-rs/tracing/pull/1374
[#1335]: https://github.com/tokio-rs/tracing/pull/1335
[#1331]: https://github.com/tokio-rs/tracing/pull/1331

# 0.1.25 (February 23, 2021)

### Added

- `Span::entered` method for entering a span and moving it into a guard by value
  rather than borrowing it ([#1252])

Thanks to @matklad for contributing to this release!

[#1252]: https://github.com/tokio-rs/tracing/pull/1252

# 0.1.24 (February 17, 2021)

### Fixed

- **attributes**: Compiler error when using `#[instrument(err)]` on functions
  which return `impl Trait` ([#1236])
- Fixed broken match arms in event macros ([#1239])
- Documentation improvements ([#1232])

Thanks to @bkchr and @lfranke for contributing to this release!

[#1236]: https://github.com/tokio-rs/tracing/pull/1236
[#1239]: https://github.com/tokio-rs/tracing/pull/1239
[#1232]: https://github.com/tokio-rs/tracing/pull/1232

# 0.1.23 (February 4, 2021)

### Fixed

- **attributes**: Compiler error when using `#[instrument(err)]` on functions
  with mutable parameters ([#1167])
- **attributes**: Missing function visibility modifier when using
  `#[instrument]` with `async-trait` ([#977])
- **attributes** Removed unused `syn` features ([#928])
- **log**: Fixed an issue where the `tracing` macros would generate code for
  events whose levels are disabled statically by the `log` crate's
  `static_max_level_XXX` features ([#1175])
- Fixed deprecations and clippy lints ([#1195])
- Several documentation fixes and improvements ([#941], [#965], [#981], [#1146],
  [#1215])
  
### Changed

- **attributes**: `tracing-futures` dependency is no longer required when using
  `#[instrument]` on async functions ([#808])
- **attributes**: Updated `tracing-attributes` minimum dependency to v0.1.12
  ([#1222])

Thanks to @nagisa, @Txuritan, @TaKO8Ki, @okready, and @krojew for contributing
to this release!

[#1167]: https://github.com/tokio-rs/tracing/pull/1167
[#977]: https://github.com/tokio-rs/tracing/pull/977
[#965]: https://github.com/tokio-rs/tracing/pull/965
[#981]: https://github.com/tokio-rs/tracing/pull/981
[#1215]: https://github.com/tokio-rs/tracing/pull/1215
[#808]: https://github.com/tokio-rs/tracing/pull/808
[#941]: https://github.com/tokio-rs/tracing/pull/941
[#1146]: https://github.com/tokio-rs/tracing/pull/1146
[#1175]: https://github.com/tokio-rs/tracing/pull/1175
[#1195]: https://github.com/tokio-rs/tracing/pull/1195
[#1222]: https://github.com/tokio-rs/tracing/pull/1222

# 0.1.22 (November 23, 2020)

### Changed

- Updated `pin-project-lite` dependency to 0.2 ([#1108])

[#1108]: https://github.com/tokio-rs/tracing/pull/1108

# 0.1.21 (September 28, 2020)

### Fixed

- Incorrect inlining of `Span::new`, `Span::new_root`, and `Span::new_child_of`,
  which could result in  `dispatcher::get_default` being inlined at the callsite
  ([#994])
- Regression where using a struct field as a span or event field when other
  fields on that struct are borrowed mutably would fail to compile ([#987])

### Changed

- Updated `tracing-core` to 0.1.17 ([#992])

### Added

- `Instrument` trait and `Instrumented` type for attaching a `Span` to a
  `Future` ([#808])
- `Copy` implementations for `Level` and `LevelFilter` ([#992])
- Multiple documentation fixes and improvements ([#964], [#980], [#981])

Thanks to @nagisa, and new contributors @SecurityInsanity, @froydnj, @jyn514 and
@TaKO8Ki for contributing to this release!

[#994]: https://github.com/tokio-rs/tracing/pull/994
[#992]: https://github.com/tokio-rs/tracing/pull/992
[#987]: https://github.com/tokio-rs/tracing/pull/987
[#980]: https://github.com/tokio-rs/tracing/pull/980
[#981]: https://github.com/tokio-rs/tracing/pull/981
[#964]: https://github.com/tokio-rs/tracing/pull/964
[#808]: https://github.com/tokio-rs/tracing/pull/808

# 0.1.20 (August 24, 2020)

### Changed

- Significantly reduced assembly generated by macro invocations (#943)
- Updated `tracing-core` to 0.1.15 (#943)

### Added 

- Documented minimum supported Rust version policy (#941)

# 0.1.19 (August 10, 2020)

### Fixed

- Updated `tracing-core` to fix incorrect calculation of the global max level
  filter (#908)

### Added

- **attributes**: Support for using `self` in field expressions when
  instrumenting `async-trait` functions (#875)
- Several documentation improvements (#832, #881, #896, #897, #911, #913)

Thanks to @anton-dutov, @nightmared, @mystor, and @toshokan for contributing to
this release!
  
# 0.1.18 (July 31, 2020)

### Fixed

- Fixed a bug where `LevelFilter::OFF` (and thus also the `static_max_level_off`
  feature flag) would enable *all* traces, rather than *none* (#853)
- **log**: Fixed `tracing` macros and `Span`s not checking `log::max_level`
  before emitting `log` records (#870)
  
### Changed

- **macros**: Macros now check the global max level (`LevelFilter::current`)
  before the per-callsite cache when determining if a span or event is enabled.
  This significantly improves performance in some use cases (#853)
- **macros**: Simplified the code generated by macro expansion significantly,
  which may improve compile times and/or `rustc` optimizatation of surrounding
  code (#869, #869)
- **macros**: Macros now check the static max level before checking any runtime
  filtering, improving performance when a span or event is disabled by a
  `static_max_level_XXX` feature flag (#868) 
- `LevelFilter` is now a re-export of the `tracing_core::LevelFilter` type, it
  can now be used interchangably with the versions in `tracing-core` and
  `tracing-subscriber` (#853)
- Significant performance improvements when comparing `LevelFilter`s and
  `Level`s (#853)
- Updated the minimum `tracing-core` dependency to 0.1.12 (#853)

### Added

- **macros**: Quoted string literals may now be used as field names, to allow
  fields whose names are not valid Rust identifiers (#790)
- **docs**: Several documentation improvements (#850, #857, #841)
- `LevelFilter::current()` function, which returns the highest level that any
  subscriber will enable (#853)
- `Subscriber::max_level_hint` optional trait method, for setting the value
  returned by `LevelFilter::current()` (#853)
  
Thanks to new contributors @cuviper, @ethanboxx, @ben0x539, @dignati,
@colelawrence, and @rbtcollins for helping out with this release!

# 0.1.17 (July 22, 2020)

### Changed

- **log**: Moved verbose span enter/exit log records to "tracing::span::active"
  target, allowing them to be filtered separately (#833)
- **log**: All span lifecycle log records without fields now have the `Trace`
  log filter, to guard against `log` users enabling them by default with blanket
  level filtering (#833)
  
### Fixed

- **log**/**macros**: Fixed missing implicit imports of the
  `tracing::field::debug` and `tracing::field::display` functions inside the
  macros when the "log" feature is enabled (#835)

# 0.1.16 (July 8, 2020)

### Added

- **attributes**: Support for arbitrary expressions as fields in `#[instrument]` (#672)
- **attributes**: `#[instrument]` now emits a compiler warning when ignoring unrecognized
  input (#672, #786)
- Improved documentation on using `tracing` in async code (#769)

### Changed

- Updated `tracing-core` dependency to 0.1.11

### Fixed

- **macros**: Excessive monomorphization in macros, which could lead to
  longer compilation times (#787) 
- **log**: Compiler warnings in macros when `log` or `log-always` features
  are enabled (#753)
- Compiler error when `tracing-core/std` feature is enabled but `tracing/std` is
  not (#760)

Thanks to @nagisa for contributing to this release!

# 0.1.15 (June 2, 2020)

### Changed

- **macros**: Replaced use of legacy `local_inner_macros` with `$crate::` (#740)

### Added

- Docs fixes and improvements (#742, #731, #730)

Thanks to @bnjjj, @blaenk, and @LukeMathWalker for contributing to this release!

# 0.1.14 (May 14, 2020)

### Added

- **log**: When using the [`log`] compatibility feature alongside a `tracing`
  `Subscriber`, log records for spans now include span IDs (#613)
- **attributes**: Support for using `#[instrument]` on methods that are part of
  [`async-trait`] trait implementations (#711)
- **attributes**: Optional `#[instrument(err)]` argument to automatically emit
  an event if an instrumented function returns `Err` (#637) 
- Added `#[must_use]` attribute to the guard returned by
  `subscriber::set_default` (#685)
  
### Changed

- **log**: Made [`log`] records emitted by spans much less noisy when span IDs are
 not available (#613)
 
### Fixed

- Several typos in the documentation (#656, #710, #715)

Thanks to @FintanH, @shepmaster, @inanna-malick, @zekisharif, @bkchr, @majecty,
@ilana and @nightmared for contributing to this release! 

[`async-trait`]: https://crates.io/crates/async-trait 
[`log`]: https://crates.io/crates/log

# 0.1.13 (February 26, 2019)

### Added

- **field**: `field::Empty` type for declaring empty fields whose values will be
  recorded later (#548)
- **field**: `field::Value` implementations for `Wrapping` and `NonZero*`
  numbers (#538)
- **attributes**: Support for adding arbitrary literal fields to spans generated
  by `#[instrument]` (#569)
- **attributes**: `#[instrument]` now emits a helpful compiler error when
  attempting to skip a function parameter (#600)

### Changed

- **attributes**: The `#[instrument]` attribute was placed under an on-by-default
  feature flag "attributes" (#603)

### Fixed

- Broken and unresolvable links in RustDoc (#595)

Thanks to @oli-cosmian and @Kobzol for contributing to this release!

# 0.1.12 (January 11, 2019)

### Added

- `Span::with_subscriber` method to access the subscriber that tracks a `Span`
  (#503)
- API documentation now shows which features are required by feature-flagged
  items (#523)
- Improved README examples (#496)
- Documentation links to related crates (#507)

# 0.1.11 (December 20, 2019)

### Added

- `Span::is_none` method (#475)
- `LevelFilter::into_level` method (#470)
- `LevelFilter::from_level` function and `From<Level>` impl (#471)
- Documented minimum supported Rust version (#482)

### Fixed

- Incorrect parameter type to `Span::follows_from` that made it impossible to
  call (#467)
- Missing whitespace in `log` records generated when enabling the `log` feature
  flag (#484)
- Typos and missing links in documentation (#405, #423, #439)

# 0.1.10 (October 23, 2019)

### Added

- Support for destructuring in arguments to `#[instrument]`ed functions (#397)
- Generated field for `self` parameters when `#[instrument]`ing methods (#397)
- Optional `skip` argument to `#[instrument]` for excluding function parameters
  from generated spans (#359)
- Added `dispatcher::set_default` and `subscriber::set_default` APIs, which
  return a drop guard (#388)

### Fixed

- Some minor documentation errors (#356, #370)

# 0.1.9 (September 13, 2019)

### Fixed

- Fixed `#[instrument]`ed async functions not compiling on `nightly-2019-09-11`
  or newer (#342)

### Changed

- Significantly reduced performance impact of skipped spans and events when a
  `Subscriber` is not in use (#326)
- The `log` feature will now only cause `tracing` spans and events to emit log
  records when a `Subscriber` is not in use (#346)

### Added

- Added support for overriding the name of the span generated by `#[instrument]`
  (#330)
- `log-always` feature flag to emit log records even when a `Subscriber` is set
  (#346)

# 0.1.8 (September 3, 2019)

### Changed

- Reorganized and improved API documentation (#317)

### Removed

- Dev-dependencies on `ansi_term` and `humantime` crates, which were used only
  for examples (#316)

# 0.1.7 (August 30, 2019)

### Changed

- New (curly-brace free) event message syntax to place the message in the first
  field rather than the last (#309)

### Fixed

- Fixed a regression causing macro stack exhaustion when the `log` feature flag
  is enabled (#304)

# 0.1.6 (August 20, 2019)

### Added

- `std::error::Error` as a new primitive type (#277)
- Support for mixing key-value fields and `format_args` messages without curly
  braces as delimiters (#288)

### Changed

- `tracing-core` dependency to 0.1.5 (#294)
- `tracing-attributes` dependency to 0.1.2 (#297)

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
