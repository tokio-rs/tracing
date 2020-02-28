#[macro_use]
extern crate tracing;
extern crate test_log_support;

use test_log_support::Test;
use tracing::Level;

#[test]
fn span_lifecycle_can_be_filtered() {
    let test = Test::with_filters(&[
        ("span_lifecycle_can_be_filtered", log::LevelFilter::Trace),
        ("tracing::span", log::LevelFilter::Info),
    ]);

    error!(foo = 5);
    test.assert_logged("foo=5");

    warn!("hello {};", "world");
    test.assert_logged("hello world;");

    info!(message = "hello world;", thingy = 42, other_thingy = 666);
    test.assert_logged("hello world; thingy=42 other_thingy=666");

    let foo = span!(Level::TRACE, "foo");
    test.assert_not_logged();

    foo.in_scope(|| {
        // enter should not be logged
        test.assert_not_logged();

        trace!({foo = 3, bar = 4}, "hello {};", "san francisco");
        test.assert_logged("hello san francisco; foo=3 bar=4");
    });
    // exit should not be logged
    test.assert_not_logged();

    drop(foo);
    // drop should not be logged
    test.assert_not_logged();

    trace!(foo = 1, bar = 2, "hello world");
    test.assert_logged("hello world foo=1 bar=2");

    let foo = span!(Level::TRACE, "foo", bar = 3, baz = false);
    // creating a span with fields _should_ be logged.
    test.assert_logged("foo; bar=3 baz=false");

    foo.in_scope(|| {
        // entering the span should not be logged
        test.assert_not_logged();
    });
    // exiting the span should not be logged
    test.assert_not_logged();

    foo.record("baz", &true);
    // recording a field should be logged
    test.assert_logged("foo; baz=true");

    let bar = span!(Level::INFO, "bar");
    // lifecycles for INFO spans should be logged
    test.assert_logged("bar;");

    bar.in_scope(|| {
        // entering the INFO span should be logged
        test.assert_logged("-> bar");
    });
    // exiting the INFO span should be logged
    test.assert_logged("<- bar");

    drop(foo);
    // drop should not be logged.
    test.assert_not_logged();

    drop(bar);
    // dropping the INFO should be logged.
    test.assert_logged("-- bar");
}
