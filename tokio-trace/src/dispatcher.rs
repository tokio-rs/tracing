pub use tokio_trace_core::dispatcher::*;

#[cfg(test)]
mod tests {
    use super::*;
    use {span, subscriber};

    #[test]
    fn dispatcher_is_sticky() {
        // Test ensuring that entire trace trees are collected by the same
        // dispatcher, even across dispatcher context switches.
        let subscriber1 = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("foo")))
            .done()
            .run();
        let mut foo = Dispatch::to(subscriber1).as_default(|| {
            let mut foo = span!("foo");
            foo.enter(|| {});
            foo
        });
        Dispatch::to(subscriber::mock().done().run())
            .as_default(move || foo.enter(|| span!("bar").enter(|| {})))
    }

    #[test]
    fn dispatcher_isnt_too_sticky() {
        // Test ensuring that new trace trees are collected by the current
        // dispatcher.
        let subscriber1 = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("foo")))
            .done()
            .run();
        let subscriber2 = subscriber::mock()
            .enter(span::mock().named(Some("baz")))
            .enter(span::mock().named(Some("quux")))
            .exit(span::mock().named(Some("quux")))
            .exit(span::mock().named(Some("baz")))
            .done()
            .run();

        let mut foo = Dispatch::to(subscriber1).as_default(|| {
            let mut foo = span!("foo");
            foo.enter(|| {});
            foo
        });
        let mut baz = Dispatch::to(subscriber2).as_default(|| span!("baz"));
        Dispatch::to(subscriber::mock().run()).as_default(move || {
            foo.enter(|| span!("bar").enter(|| {}));
            baz.enter(|| span!("quux").enter(|| {}))
        })
    }

}
