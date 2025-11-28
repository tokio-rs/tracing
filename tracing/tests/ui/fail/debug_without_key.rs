//! We support bare `debug` function in macros, but only when a key is
//! specified (e.g. `foo = debug(foo)` is supported but just `debug(foo)`
//! isn't).

fn main() {
    let foo = "foo";
    tracing::info!(debug(foo));
}

