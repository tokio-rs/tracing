#[path = "../../tracing-futures/tests/support.rs"]
// we don't use some of the test support functions, but `tracing-futures` does.
#[allow(dead_code)]
mod support;
use support::*;

use tracing::subscriber::with_default;
use tracing_attributes::instrument;

#[instrument]
async fn test_async_fn(polls: usize) -> Result<(), ()> {
    let future = PollN::new_ok(polls);
    tracing::trace!(awaiting = true);
    future.await
}

#[instrument]
async fn test_async_fns_nested() {
    test_async_fns_nested_other().await
}

#[instrument]
async fn test_async_fns_nested_other() {
    tracing::trace!(nested = true);
}

#[test]
fn async_fn_only_enters_for_polls() {
    let (subscriber, handle) = subscriber::mock()
        .new_span(span::mock().named("test_async_fn"))
        .enter(span::mock().named("test_async_fn"))
        .event(event::mock().with_fields(field::mock("awaiting").with_value(&true)))
        .exit(span::mock().named("test_async_fn"))
        .enter(span::mock().named("test_async_fn"))
        .exit(span::mock().named("test_async_fn"))
        .drop_span(span::mock().named("test_async_fn"))
        .done()
        .run_with_handle();
    with_default(subscriber, || {
        block_on_future(async { test_async_fn(2).await }).unwrap();
    });
    handle.assert_finished();
}

#[test]
fn async_fn_nested() {
    let span = span::mock().named("test_async_fns_nested");
    let span2 = span::mock().named("test_async_fns_nested_other");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .new_span(span2.clone())
        .enter(span2.clone())
        .event(event::mock().with_fields(field::mock("nested").with_value(&true)))
        .exit(span2.clone())
        .drop_span(span2)
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        block_on_future(async { test_async_fns_nested().await });
    });

    handle.assert_finished();
}

#[test]
fn async_fn_with_async_trait() {
    use async_trait::async_trait;
    #[async_trait]
    pub trait TestA {
        async fn foo(&mut self, v: usize);
    }

    #[async_trait]
    pub trait TestB {
        async fn bar(&self);
    }

    #[derive(Debug)]
    struct TestImpl(usize);

    #[async_trait]
    impl TestA for TestImpl {
        #[instrument]
        async fn foo(&mut self, v: usize) {
            self.0 = v;
            self.bar().await
        }
    }

    #[async_trait]
    impl TestB for TestImpl {
        #[instrument(skip(self))]
        async fn bar(&self) {
            tracing::trace!(val = self.0);
        }
    }

    let span = span::mock().named("foo");
    let span2 = span::mock().named("bar");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .new_span(span2.clone())
        .enter(span2.clone())
        .event(event::mock().with_fields(field::mock("val").with_value(&5u64)))
        .exit(span2.clone())
        .drop_span(span2)
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        let mut test = TestImpl(2);
        block_on_future(async { test.foo(5).await });
    });

    handle.assert_finished();
}
