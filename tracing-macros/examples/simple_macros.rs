fn main() {
    tracing_macros::event!(target: "my target", foo = 3, bar.baz = "a string");
}
