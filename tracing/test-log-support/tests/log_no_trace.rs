use test_log_support::Test;
use tracing::{error, info, span, trace, warn, Level};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn test_always_log() {
    let test = Test::start();

    error!(foo = 5);
    test.assert_logged("foo=5");

    warn!("hello {};", "world");
    test.assert_logged("hello world;");

    info!(
        message = "hello world;",
        thingy = display(42),
        other_thingy = debug(666)
    );
    test.assert_logged("hello world; thingy=42 other_thingy=666");

    let foo = span!(Level::TRACE, "foo");
    test.assert_logged("foo;");

    foo.in_scope(|| {
        test.assert_logged("-> foo;");

        trace!({foo = 3, bar = 4}, "hello {};", "san francisco");
        test.assert_logged("hello san francisco; foo=3 bar=4");
    });
    test.assert_logged("<- foo;");

    drop(foo);
    test.assert_logged("-- foo;");

    trace!(foo = 1, bar = 2, "hello world");
    test.assert_logged("hello world foo=1 bar=2");

    let foo = span!(Level::TRACE, "foo", bar = 3, baz = false);
    test.assert_logged("foo; bar=3 baz=false");

    drop(foo);
}
