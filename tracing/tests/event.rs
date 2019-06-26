#[macro_use]
extern crate tracing;
mod support;

use self::support::*;

use tracing::{
    field::{debug, display},
    subscriber::with_default,
    Level,
};

#[test]
fn event_without_message() {
    let (subscriber, handle) = subscriber::mock()
        .event(
            event::mock().with_fields(
                field::mock("answer")
                    .with_value(&42)
                    .and(
                        field::mock("to_question")
                            .with_value(&"life, the universe, and everything"),
                    )
                    .only(),
            ),
        )
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        info!(
            answer = 42,
            to_question = "life, the universe, and everything"
        );
    });

    handle.assert_finished();
}

#[test]
fn event_with_message() {
    let (subscriber, handle) = subscriber::mock()
        .event(event::mock().with_fields(field::mock("message").with_value(
            &tracing::field::debug(format_args!("hello from my event! yak shaved = {:?}", true)),
        )))
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        debug!("hello from my event! yak shaved = {:?}", true);
    });

    handle.assert_finished();
}

#[test]
fn one_with_everything() {
    let (subscriber, handle) = subscriber::mock()
        .event(
            event::mock()
                .with_fields(
                    field::mock("message")
                        .with_value(&tracing::field::debug(format_args!(
                            "{:#x} make me one with{what:.>20}",
                            4277009102u64,
                            what = "everything"
                        )))
                        .and(field::mock("foo").with_value(&666))
                        .and(field::mock("bar").with_value(&false))
                        .only(),
                )
                .at_level(Level::ERROR)
                .with_target("whatever"),
        )
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        event!(
            target: "whatever",
            Level::ERROR,
            { foo = 666, bar = false },
             "{:#x} make me one with{what:.>20}", 4277009102u64, what = "everything"
        );
    });

    handle.assert_finished();
}

#[test]
fn moved_field() {
    let (subscriber, handle) = subscriber::mock()
        .event(
            event::mock().with_fields(
                field::mock("foo")
                    .with_value(&display("hello from my event"))
                    .only(),
            ),
        )
        .done()
        .run_with_handle();
    with_default(subscriber, || {
        let from = "my event";
        event!(Level::INFO, foo = display(format!("hello from {}", from)))
    });

    handle.assert_finished();
}

#[test]
fn dotted_field_name() {
    let (subscriber, handle) = subscriber::mock()
        .event(
            event::mock().with_fields(
                field::mock("foo.bar")
                    .with_value(&true)
                    .and(field::mock("foo.baz").with_value(&false))
                    .only(),
            ),
        )
        .done()
        .run_with_handle();
    with_default(subscriber, || {
        event!(Level::INFO, foo.bar = true, foo.baz = false);
    });

    handle.assert_finished();
}

#[test]
fn borrowed_field() {
    let (subscriber, handle) = subscriber::mock()
        .event(
            event::mock().with_fields(
                field::mock("foo")
                    .with_value(&display("hello from my event"))
                    .only(),
            ),
        )
        .done()
        .run_with_handle();
    with_default(subscriber, || {
        let from = "my event";
        let mut message = format!("hello from {}", from);
        event!(Level::INFO, foo = display(&message));
        message.push_str(", which happened!");
    });

    handle.assert_finished();
}

#[test]
// If emitting log instrumentation, this gets moved anyway, breaking the test.
#[cfg(not(feature = "log"))]
fn move_field_out_of_struct() {
    use tracing::field::debug;

    #[derive(Debug)]
    struct Position {
        x: f32,
        y: f32,
    }

    let pos = Position {
        x: 3.234,
        y: -1.223,
    };
    let (subscriber, handle) = subscriber::mock()
        .event(
            event::mock().with_fields(
                field::mock("x")
                    .with_value(&debug(3.234))
                    .and(field::mock("y").with_value(&debug(-1.223)))
                    .only(),
            ),
        )
        .event(event::mock().with_fields(field::mock("position").with_value(&debug(&pos))))
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        let pos = Position {
            x: 3.234,
            y: -1.223,
        };
        debug!(x = debug(pos.x), y = debug(pos.y));
        debug!(target: "app_events", { position = debug(pos) }, "New position");
    });
    handle.assert_finished();
}

#[test]
fn display_shorthand() {
    let (subscriber, handle) = subscriber::mock()
        .event(
            event::mock().with_fields(
                field::mock("my_field")
                    .with_value(&display("hello world"))
                    .only(),
            ),
        )
        .done()
        .run_with_handle();
    with_default(subscriber, || {
        event!(Level::TRACE, my_field = %"hello world");
    });

    handle.assert_finished();
}

#[test]
fn debug_shorthand() {
    let (subscriber, handle) = subscriber::mock()
        .event(
            event::mock().with_fields(
                field::mock("my_field")
                    .with_value(&debug("hello world"))
                    .only(),
            ),
        )
        .done()
        .run_with_handle();
    with_default(subscriber, || {
        event!(Level::TRACE, my_field = ?"hello world");
    });

    handle.assert_finished();
}

#[test]
fn both_shorthands() {
    let (subscriber, handle) = subscriber::mock()
        .event(
            event::mock().with_fields(
                field::mock("display_field")
                    .with_value(&display("hello world"))
                    .and(field::mock("debug_field").with_value(&debug("hello world")))
                    .only(),
            ),
        )
        .done()
        .run_with_handle();
    with_default(subscriber, || {
        event!(Level::TRACE, display_field = %"hello world", debug_field = ?"hello world");
    });

    handle.assert_finished();
}

#[test]
fn explicit_child() {
    let (subscriber, handle) = subscriber::mock()
        .new_span(span::mock().named("foo"))
        .event(event::mock().with_explicit_parent(Some("foo")))
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        let foo = span!(Level::TRACE, "foo");
        event!(Level::TRACE, parent: foo.id(), "bar");
    });

    handle.assert_finished();
}

#[test]
fn explicit_child_at_levels() {
    let (subscriber, handle) = subscriber::mock()
        .new_span(span::mock().named("foo"))
        .event(event::mock().with_explicit_parent(Some("foo")))
        .event(event::mock().with_explicit_parent(Some("foo")))
        .event(event::mock().with_explicit_parent(Some("foo")))
        .event(event::mock().with_explicit_parent(Some("foo")))
        .event(event::mock().with_explicit_parent(Some("foo")))
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        let foo = span!(Level::TRACE, "foo");
        trace!(parent: foo.id(), "a");
        debug!(parent: foo.id(), "b");
        info!(parent: foo.id(), "c");
        warn!(parent: foo.id(), "d");
        error!(parent: foo.id(), "e");
    });

    handle.assert_finished();
}
