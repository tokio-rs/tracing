fn main() {
    tracing_macros::event!(foo = 3, bar.baz = "a string");
}
