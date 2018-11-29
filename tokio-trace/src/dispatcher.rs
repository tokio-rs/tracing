pub use tokio_trace_core::dispatcher::*;

use std::cell::RefCell;

thread_local! {
    static CURRENT_DISPATCH: RefCell<Dispatch> = RefCell::new(Dispatch::none());
}

/// Sets this dispatch as the default for the duration of a closure.
///
/// The default dispatcher is used when creating a new [`Span`] or
/// [`Event`], _if no span is currently executing_. If a span is currently
/// executing, new spans or events are dispatched to the subscriber that
/// tagged that span, instead.
///
/// [`Span`]: ::span::Span
/// [`Subscriber`]: ::Subscriber
/// [`Event`]: ::Event
pub fn with_default<T>(dispatcher: Dispatch, f: impl FnOnce() -> T) -> T {
    let prior = CURRENT_DISPATCH.try_with(|current| current.replace(dispatcher));
    let result = f();
    if let Ok(prior) = prior {
        let _ = CURRENT_DISPATCH.try_with(|current| {
            *current.borrow_mut() = prior;
        });
    }
    result
}

pub(crate) fn with_current<T, F>(mut f: F) -> T
where
    F: FnMut(&Dispatch) -> T,
{
    CURRENT_DISPATCH
        .try_with(|current| f(&*current.borrow()))
        .unwrap_or_else(|_| f(&Dispatch::none()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use {dispatcher, span, subscriber};

    #[test]
    fn dispatcher_is_sticky() {
        // Test ensuring that entire trace trees are collected by the same
        // dispatcher, even across dispatcher context switches.
        let (subscriber1, handle1) = subscriber::mock()
            .enter(span::mock().named("foo"))
            .exit(span::mock().named("foo"))
            .enter(span::mock().named("foo"))
            .enter(span::mock().named("bar"))
            .exit(span::mock().named("bar"))
            .drop_span(span::mock().named("bar"))
            .exit(span::mock().named("foo"))
            .drop_span(span::mock().named("foo"))
            .done()
            .run_with_handle();
        let mut foo = dispatcher::with_default(Dispatch::new(subscriber1), || {
            let mut foo = span!("foo");
            foo.enter(|| {});
            foo
        });
        dispatcher::with_default(Dispatch::new(subscriber::mock().done().run()), move || {
            foo.enter(|| span!("bar").enter(|| {}))
        });

        handle1.assert_finished();
    }

    #[test]
    fn dispatcher_isnt_too_sticky() {
        // Test ensuring that new trace trees are collected by the current
        // dispatcher.
        let (subscriber1, handle1) = subscriber::mock()
            .enter(span::mock().named("foo"))
            .exit(span::mock().named("foo"))
            .enter(span::mock().named("foo"))
            .enter(span::mock().named("bar"))
            .exit(span::mock().named("bar"))
            .drop_span(span::mock().named("bar"))
            .exit(span::mock().named("foo"))
            .drop_span(span::mock().named("foo"))
            .done()
            .run_with_handle();
        let (subscriber2, handle2) = subscriber::mock()
            .enter(span::mock().named("baz"))
            .enter(span::mock().named("quux"))
            .exit(span::mock().named("quux"))
            .drop_span(span::mock().named("quux"))
            .exit(span::mock().named("baz"))
            .drop_span(span::mock().named("baz"))
            .done()
            .run_with_handle();

        let mut foo = dispatcher::with_default(Dispatch::new(subscriber1), || {
            let mut foo = span!("foo");
            foo.enter(|| {});
            foo
        });
        let mut baz = dispatcher::with_default(Dispatch::new(subscriber2), || span!("baz"));
        dispatcher::with_default(Dispatch::new(subscriber::mock().done().run()), move || {
            foo.enter(|| span!("bar").enter(|| {}));
            baz.enter(|| span!("quux").enter(|| {}))
        });

        handle1.assert_finished();
        handle2.assert_finished();
    }

}
