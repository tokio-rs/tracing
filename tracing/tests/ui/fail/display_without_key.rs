//! We support bare `display` function in macros, but only when a key is
//! specified (e.g. `foo = display(foo)` is supported but just `display(foo)`
//! isn't).

fn main() {
    let foo = "foo";
    tracing::info!(display(foo));
}
