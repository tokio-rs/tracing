![Tracing — Structured, application-level diagnostics][splash]

[splash]: https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/splash.svg

# tracing-journald

Support for logging [`tracing`][tracing] events natively to journald, preserving structured information.

[![Crates.io][crates-badge]][crates-url]
[![Documentation (master)][docs-master-badge]][docs-master-url]
[![MIT licensed][mit-badge]][mit-url]
![maintenance status][maint-badge]

[tracing]: https://github.com/tokio-rs/tracing/tree/master/tracing
[tracing-fmt]: https://github.com/tokio-rs/tracing/tree/master/tracing-journald
[crates-badge]: https://img.shields.io/crates/v/tracing-journald.svg
[crates-url]: https://crates.io/crates/tracing-journald
[docs-master-badge]: https://img.shields.io/badge/docs-master-blue
[docs-master-url]: https://tracing-rs.netlify.com/tracing_journald
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[maint-badge]: https://img.shields.io/badge/maintenance-experimental-blue.svg

*Compiler support: [requires `rustc` 1.40+][msrv]*

[msrv]: #supported-rust-versions

## Supported Rust Versions

Tracing is built against the latest stable release. The minimum supported
version is 1.40. The current Tracing version is not guaranteed to build on Rust
versions earlier than the minimum supported version.

Tracing follows the same compiler support policies as the rest of the Tokio
project. The current stable Rust compiler and the three most recent minor
versions before it will always be supported. For example, if the current stable
compiler version is 1.45, the minimum supported version will not be increased
past 1.42, three minor versions prior. Increasing the minimum supported compiler
version is not considered a semver breaking change as long as doing so complies
with this policy.

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tracing by you, shall be licensed as MIT, without any additional
terms or conditions.
