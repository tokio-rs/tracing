use tracing::subscriber::with_default;
use tracing::Level;
use tracing_attributes::instrument;
use tracing_test::*;

#[test]
fn override_everything() {
    #[instrument(target = "my_target", level = "debug")]
    fn my_fn() {}

    #[instrument(level = "debug", target = "my_target")]
    fn my_other_fn() {}

    let span1 = span("my_fn")
        .at_level(Level::DEBUG)
        .with_target("my_target");
    let span2 = span("my_other_fn")
        .at_level(Level::DEBUG)
        .with_target("my_target");
    let (subscriber, handle) = subscriber::expect()
        .new_span(span1.clone())
        .enter(span1.clone())
        .exit(span1.clone())
        .drop_span(span1)
        .new_span(span2.clone())
        .enter(span2.clone())
        .exit(span2.clone())
        .drop_span(span2)
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        my_fn();
        my_other_fn();
    });

    handle.assert_finished();
}

#[test]
fn fields() {
    #[instrument(target = "my_target", level = "debug")]
    fn my_fn(arg1: usize, arg2: bool) {}

    let span1 = span("my_fn")
        .at_level(Level::DEBUG)
        .with_target("my_target");

    let span2 = span("my_fn")
        .at_level(Level::DEBUG)
        .with_target("my_target");
    let (subscriber, handle) = subscriber::expect()
        .new_span(
            span1.clone().with_field(
                field("arg1")
                    .with_value(&format_args!("2"))
                    .and(field("arg2").with_value(&format_args!("false")))
                    .only(),
            ),
        )
        .enter(span1.clone())
        .exit(span1.clone())
        .drop_span(span1)
        .new_span(
            span2.clone().with_field(
                field("arg1")
                    .with_value(&format_args!("3"))
                    .and(field("arg2").with_value(&format_args!("true")))
                    .only(),
            ),
        )
        .enter(span2.clone())
        .exit(span2.clone())
        .drop_span(span2)
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        my_fn(2, false);
        my_fn(3, true);
    });

    handle.assert_finished();
}

#[test]
fn skip() {
    struct UnDebug(pub u32);

    #[instrument(target = "my_target", level = "debug", skip(_arg2, _arg3))]
    fn my_fn(arg1: usize, _arg2: UnDebug, _arg3: UnDebug) {}

    let span1 = span("my_fn")
        .at_level(Level::DEBUG)
        .with_target("my_target");

    let span2 = span("my_fn")
        .at_level(Level::DEBUG)
        .with_target("my_target");
    let (subscriber, handle) = subscriber::expect()
        .new_span(
            span1
                .clone()
                .with_field(field("arg1").with_value(&format_args!("2")).only()),
        )
        .enter(span1.clone())
        .exit(span1.clone())
        .drop_span(span1)
        .new_span(
            span2
                .clone()
                .with_field(field("arg1").with_value(&format_args!("3")).only()),
        )
        .enter(span2.clone())
        .exit(span2.clone())
        .drop_span(span2)
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        my_fn(2, UnDebug(0), UnDebug(1));
        my_fn(3, UnDebug(0), UnDebug(1));
    });

    handle.assert_finished();
}

#[test]
fn generics() {
    #[derive(Debug)]
    struct Foo;

    #[instrument]
    fn my_fn<S, T: std::fmt::Debug>(arg1: S, arg2: T)
    where
        S: std::fmt::Debug,
    {
    }

    let span = span("my_fn");

    let (subscriber, handle) = subscriber::expect()
        .new_span(
            span.clone().with_field(
                field("arg1")
                    .with_value(&format_args!("Foo"))
                    .and(field("arg2").with_value(&format_args!("false"))),
            ),
        )
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        my_fn(Foo, false);
    });

    handle.assert_finished();
}

#[test]
fn methods() {
    #[derive(Debug)]
    struct Foo;

    impl Foo {
        #[instrument]
        fn my_fn(&self, arg1: usize) {}
    }

    let span = span("my_fn");

    let (subscriber, handle) = subscriber::expect()
        .new_span(
            span.clone().with_field(
                field("self")
                    .with_value(&format_args!("Foo"))
                    .and(field("arg1").with_value(&format_args!("42"))),
            ),
        )
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        let foo = Foo;
        foo.my_fn(42);
    });

    handle.assert_finished();
}
