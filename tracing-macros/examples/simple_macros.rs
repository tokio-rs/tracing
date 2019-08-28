use tracing::Level;

fn main() {
    tracing_macros::event!(Level::INFO, target: "my target", foo = 3, bar.baz = "a string");

    tracing_macros::event!(Level::TRACE, "hello", foo = 3, bar.baz = "a string");

    tracing_macros::event!(
        Level::TRACE,
        "hello {} world {:?}",
        foo = 3,
        bar.baz = "a string"
    );
}
