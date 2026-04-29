use tracing::{event, span, Level};

/// Test that spans and events only use their argument once. See #196 and #1739.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn test_move_arg() {
    let foo = Foo;
    let parent_span = span!(Level::INFO, "Span 1", bar = ?Bar(foo));
    let foo = Foo;
    span!(parent: &parent_span, Level::INFO, "Span 2", bar = ?Bar(foo));

    let foo = Foo;
    event!(Level::INFO, bar = ?Bar(foo), "Event 1");
    let foo = Foo;
    event!(parent: &parent_span, Level::INFO, bar = ?Bar(foo), "Event 2");
}

#[derive(Debug)]
struct Foo;

#[derive(Debug)]
struct Bar(Foo);
