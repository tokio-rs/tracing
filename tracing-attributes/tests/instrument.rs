mod support;
use support::*;

use tracing::subscriber::with_default;
use tracing::Level;
use tracing_attributes::instrument;

#[test]
fn override_everything() {
    #[instrument(target = "my_target", level = "debug")]
    fn my_fn() {}

    #[instrument(level = "debug", target = "my_target")]
    fn my_other_fn() {}

    let span = span::mock()
        .named("my_fn")
        .at_level(Level::DEBUG)
        .with_target("my_target");
    let span2 = span::mock()
        .named("my_other_fn")
        .at_level(Level::DEBUG)
        .with_target("my_target");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
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

    let span = span::mock()
        .named("my_fn")
        .at_level(Level::DEBUG)
        .with_target("my_target");

    let span2 = span::mock()
        .named("my_fn")
        .at_level(Level::DEBUG)
        .with_target("my_target");
    let (subscriber, handle) = subscriber::mock()
        .new_span(
            span.clone().with_field(
                field::mock("arg1")
                    .with_value(&format_args!("2"))
                    .and(field::mock("arg2").with_value(&format_args!("false")))
                    .only(),
            ),
        )
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .new_span(
            span2.clone().with_field(
                field::mock("arg1")
                    .with_value(&format_args!("3"))
                    .and(field::mock("arg2").with_value(&format_args!("true")))
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

    let span = span::mock()
        .named("my_fn")
        .at_level(Level::DEBUG)
        .with_target("my_target");

    let span2 = span::mock()
        .named("my_fn")
        .at_level(Level::DEBUG)
        .with_target("my_target");
    let (subscriber, handle) = subscriber::mock()
        .new_span(
            span.clone()
                .with_field(field::mock("arg1").with_value(&format_args!("2")).only()),
        )
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .new_span(
            span2
                .clone()
                .with_field(field::mock("arg1").with_value(&format_args!("3")).only()),
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

    let span = span::mock().named("my_fn");

    let (subscriber, handle) = subscriber::mock()
        .new_span(
            span.clone().with_field(
                field::mock("arg1")
                    .with_value(&format_args!("Foo"))
                    .and(field::mock("arg2").with_value(&format_args!("false"))),
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
fn destructure_tuples() {
    #[instrument]
    fn my_fn((arg1, arg2): (usize, usize)) {}

    let span = span::mock().named("my_fn");

    let (subscriber, handle) = subscriber::mock()
        .new_span(
            span.clone().with_field(
                field::mock("arg1").with_value(&format_args!("1"))
                    .and(field::mock("arg2").with_value(&format_args!("2")))
                    .only()
            ),
        )
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        my_fn((1, 2));
    });

    handle.assert_finished();
}

#[test]
fn destructure_nested_tuples() {
    #[instrument]
    fn my_fn(((arg1, arg2), (arg3, arg4)): ((usize, usize), (usize, usize))) {}

    let span = span::mock().named("my_fn");

    let (subscriber, handle) = subscriber::mock()
        .new_span(
            span.clone().with_field(
                field::mock("arg1").with_value(&format_args!("1"))
                    .and(field::mock("arg2").with_value(&format_args!("2")))
                    .and(field::mock("arg3").with_value(&format_args!("3")))
                    .and(field::mock("arg4").with_value(&format_args!("4")))
                    .only()
            ),
        )
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        my_fn(((1, 2), (3, 4)));
    });

    handle.assert_finished();
}

#[test]
fn destructure_refs() {
    #[instrument]
    fn my_fn(&arg1: &usize) {}

    let span = span::mock().named("my_fn");

    let (subscriber, handle) = subscriber::mock()
        .new_span(
            span.clone().with_field(
                field::mock("arg1").with_value(&format_args!("1"))
                    .only()
            ),
        )
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        my_fn(&1);
    });

    handle.assert_finished();
}

#[test]
fn destructure_tuple_structs() {
    struct Foo(usize, usize);

    #[instrument]
    fn my_fn(Foo(arg1, arg2): Foo) {}

    let span = span::mock().named("my_fn");

    let (subscriber, handle) = subscriber::mock()
        .new_span(
            span.clone().with_field(
                field::mock("arg1").with_value(&format_args!("1"))
                    .and(field::mock("arg2").with_value(&format_args!("2")))
                    .only()
            ),
        )
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        my_fn(Foo(1, 2));
    });

    handle.assert_finished();
}

#[test]
fn destructure_structs() {
    struct Foo {
        bar: usize,
        baz: usize,
    }

    #[instrument]
    fn my_fn(Foo { bar: arg1, baz: arg2 }: Foo) { let _ = (arg1, arg2); }

    let span = span::mock().named("my_fn");

    let (subscriber, handle) = subscriber::mock()
        .new_span(
            span.clone().with_field(
                field::mock("arg1").with_value(&format_args!("1"))
                    .and(field::mock("arg2").with_value(&format_args!("2")))
                    .only()
            ),
        )
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        my_fn(Foo { bar: 1, baz: 2 });
    });

    handle.assert_finished();
}
