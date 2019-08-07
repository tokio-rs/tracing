# nightly-only examples

These examples demonstrate compatibility with Rust features that are not yet
stable (primarily, async-await syntax).

Note that these examples are _not_ in the root workspace, to avoid compiling
them on unsupported Rust versions when running `cargo test --all`. Therefore,
they must be run from within the `nightly-examples` directory.

## Examples

- `async_fn.rs`: demonstrates how the `trace` attribute macro can be used to
  instrument an `async fn`.
