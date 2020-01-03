#[macro_use]
extern crate tracing;
extern crate test_log_support;

use test_log_support::Test;
use tracing::Level;

pub struct NopSubscriber;

impl tracing::Subscriber for NopSubscriber {
    fn enabled(&self, _: &tracing::Metadata) -> bool {
        true
    }
    fn new_span(&self, _: &tracing::span::Attributes) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, _: &tracing::Event) {}
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
    fn try_close(&self, _: tracing::span::Id) -> bool {
        true
    }
}

#[test]
fn test_always_log() {
    tracing::subscriber::set_global_default(NopSubscriber).expect("set global should succeed");

    let test = Test::start();

    error!(foo = 5);
    test.assert_logged("foo=5");

    warn!("hello {};", "world");
    test.assert_logged("hello world;");

    info!(message = "hello world;", thingy = 42, other_thingy = 666);
    test.assert_logged("hello world; thingy=42 other_thingy=666");

    let foo = span!(Level::TRACE, "foo");
    test.assert_logged("++ foo;");

    foo.in_scope(|| {
        test.assert_logged("-> foo");

        trace!({foo = 3, bar = 4}, "hello {};", "san francisco");
        test.assert_logged("hello san francisco; foo=3 bar=4");
    });
    test.assert_logged("<- foo");

    drop(foo);
    test.assert_logged("-- foo");

    let foo = span!(Level::TRACE, "foo", bar = 3, baz = false);
    test.assert_logged("++ foo; bar=3 baz=false");

    drop(foo);
    test.assert_logged("-- foo");

    trace!(foo = 1, bar = 2, "hello world");
    test.assert_logged("hello world foo=1 bar=2");

    // TODO(#1138): determine a new syntax for uninitialized span fields, and
    // re-enable these.
    // let span = span!(Level::TRACE, "foo", bar = _, baz = _);
    // span.record("bar", &3);
    // test.assert_logged("foo; bar=3");
    // span.record("baz", &"a string");
    // test.assert_logged("foo; baz=\"a string\"");
}
