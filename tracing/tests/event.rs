// These tests require the thread-local scoped dispatcher, which only works when
// we have a standard library. The behaviour being tested should be the same
// with the standard lib disabled.
//
// The alternative would be for each of these tests to be defined in a separate
// file, which is :(
#![cfg(feature = "std")]

use tracing::{
    collect::with_default,
    debug, error,
    field::{debug, display},
    info, trace, warn, Level,
};
use tracing_mock::*;

macro_rules! event_without_message {
    ($name:ident: $e:expr) => {
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
        #[test]
        fn $name() {
            let (collector, handle) = collector::mock()
                .event(
                    expect::event().with_fields(
                        expect::field("answer")
                            .with_value(&42)
                            .and(
                                expect::field("to_question")
                                    .with_value(&"life, the universe, and everything"),
                            )
                            .only(),
                    ),
                )
                .only()
                .run_with_handle();

            with_default(collector, || {
                info!(
                    answer = $e,
                    to_question = "life, the universe, and everything"
                );
            });

            handle.assert_finished();
        }
    };
}

event_without_message! {event_without_message: 42}
event_without_message! {wrapping_event_without_message: std::num::Wrapping(42)}
event_without_message! {nonzeroi32_event_without_message: std::num::NonZeroI32::new(42).unwrap()}
// needs API breakage
//event_without_message!{nonzerou128_event_without_message: std::num::NonZeroU128::new(42).unwrap()}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn event_with_message() {
    let (collector, handle) = collector::mock()
        .event(
            expect::event().with_fields(expect::field("message").with_value(
                &tracing::field::debug(format_args!(
                    "hello from my tracing::event! yak shaved = {:?}",
                    true
                )),
            )),
        )
        .only()
        .run_with_handle();

    with_default(collector, || {
        debug!("hello from my tracing::event! yak shaved = {:?}", true);
    });

    handle.assert_finished();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn message_without_delims() {
    let (collector, handle) = collector::mock()
        .event(
            expect::event().with_fields(
                expect::field("answer")
                    .with_value(&42)
                    .and(
                        expect::field("question").with_value(&"life, the universe, and everything"),
                    )
                    .and(expect::message(format_args!(
                        "hello from my event! tricky? {:?}!",
                        true
                    )))
                    .only(),
            ),
        )
        .only()
        .run_with_handle();

    with_default(collector, || {
        let question = "life, the universe, and everything";
        debug!(answer = 42, question, "hello from {where}! tricky? {:?}!", true, where = "my event");
    });

    handle.assert_finished();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn string_message_without_delims() {
    let (collector, handle) = collector::mock()
        .event(
            expect::event().with_fields(
                expect::field("answer")
                    .with_value(&42)
                    .and(
                        expect::field("question").with_value(&"life, the universe, and everything"),
                    )
                    .and(expect::message(format_args!("hello from my event")))
                    .only(),
            ),
        )
        .only()
        .run_with_handle();

    with_default(collector, || {
        let question = "life, the universe, and everything";
        debug!(answer = 42, question, "hello from my event");
    });

    handle.assert_finished();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn one_with_everything() {
    let (collector, handle) = collector::mock()
        .event(
            expect::event()
                .with_fields(
                    expect::field("message")
                        .with_value(&tracing::field::debug(format_args!(
                            "{:#x} make me one with{what:.>20}",
                            4_277_009_102u64,
                            what = "everything"
                        )))
                        .and(expect::field("foo").with_value(&666))
                        .and(expect::field("bar").with_value(&false))
                        .and(expect::field("like_a_butterfly").with_value(&42.0))
                        .only(),
                )
                .at_level(Level::ERROR)
                .with_target("whatever"),
        )
        .only()
        .run_with_handle();

    with_default(collector, || {
        tracing::event!(
            target: "whatever",
            Level::ERROR,
            { foo = 666, bar = false, like_a_butterfly = 42.0 },
             "{:#x} make me one with{what:.>20}", 4_277_009_102u64, what = "everything"
        );
    });

    handle.assert_finished();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn moved_field() {
    let (collector, handle) = collector::mock()
        .event(
            expect::event().with_fields(
                expect::field("foo")
                    .with_value(&display("hello from my event"))
                    .only(),
            ),
        )
        .only()
        .run_with_handle();
    with_default(collector, || {
        let from = "my event";
        tracing::event!(Level::INFO, foo = display(format!("hello from {}", from)))
    });

    handle.assert_finished();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn dotted_field_name() {
    let (collector, handle) = collector::mock()
        .event(
            expect::event().with_fields(
                expect::field("foo.bar")
                    .with_value(&true)
                    .and(expect::field("foo.baz").with_value(&false))
                    .only(),
            ),
        )
        .only()
        .run_with_handle();
    with_default(collector, || {
        tracing::event!(Level::INFO, foo.bar = true, foo.baz = false);
    });

    handle.assert_finished();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn borrowed_field() {
    let (collector, handle) = collector::mock()
        .event(
            expect::event().with_fields(
                expect::field("foo")
                    .with_value(&display("hello from my event"))
                    .only(),
            ),
        )
        .only()
        .run_with_handle();
    with_default(collector, || {
        let from = "my event";
        let mut message = format!("hello from {}", from);
        tracing::event!(Level::INFO, foo = display(&message));
        message.push_str(", which happened!");
    });

    handle.assert_finished();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
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
    let (collector, handle) = collector::mock()
        .event(
            expect::event().with_fields(
                expect::field("x")
                    .with_value(&debug(3.234))
                    .and(expect::field("y").with_value(&debug(-1.223)))
                    .only(),
            ),
        )
        .event(expect::event().with_fields(expect::field("position").with_value(&debug(&pos))))
        .only()
        .run_with_handle();

    with_default(collector, || {
        let pos = Position {
            x: 3.234,
            y: -1.223,
        };
        debug!(x = debug(pos.x), y = debug(pos.y));
        debug!(target: "app_events", { position = debug(pos) }, "New position");
    });
    handle.assert_finished();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn display_shorthand() {
    let (collector, handle) = collector::mock()
        .event(
            expect::event().with_fields(
                expect::field("my_field")
                    .with_value(&display("hello world"))
                    .only(),
            ),
        )
        .only()
        .run_with_handle();
    with_default(collector, || {
        tracing::event!(Level::TRACE, my_field = %"hello world");
    });

    handle.assert_finished();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn debug_shorthand() {
    let (collector, handle) = collector::mock()
        .event(
            expect::event().with_fields(
                expect::field("my_field")
                    .with_value(&debug("hello world"))
                    .only(),
            ),
        )
        .only()
        .run_with_handle();
    with_default(collector, || {
        tracing::event!(Level::TRACE, my_field = ?"hello world");
    });

    handle.assert_finished();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn both_shorthands() {
    let (collector, handle) = collector::mock()
        .event(
            expect::event().with_fields(
                expect::field("display_field")
                    .with_value(&display("hello world"))
                    .and(expect::field("debug_field").with_value(&debug("hello world")))
                    .only(),
            ),
        )
        .only()
        .run_with_handle();
    with_default(collector, || {
        tracing::event!(Level::TRACE, display_field = %"hello world", debug_field = ?"hello world");
    });

    handle.assert_finished();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn explicit_child() {
    let (collector, handle) = collector::mock()
        .new_span(expect::span().named("foo"))
        .event(expect::event().with_ancestry(expect::has_explicit_parent("foo")))
        .only()
        .run_with_handle();

    with_default(collector, || {
        let foo = tracing::span!(Level::TRACE, "foo");
        tracing::event!(parent: foo.id(), Level::TRACE, "bar");
    });

    handle.assert_finished();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn explicit_child_at_levels() {
    let (collector, handle) = collector::mock()
        .new_span(expect::span().named("foo"))
        .event(expect::event().with_ancestry(expect::has_explicit_parent("foo")))
        .event(expect::event().with_ancestry(expect::has_explicit_parent("foo")))
        .event(expect::event().with_ancestry(expect::has_explicit_parent("foo")))
        .event(expect::event().with_ancestry(expect::has_explicit_parent("foo")))
        .event(expect::event().with_ancestry(expect::has_explicit_parent("foo")))
        .only()
        .run_with_handle();

    with_default(collector, || {
        let foo = tracing::span!(Level::TRACE, "foo");
        trace!(parent: foo.id(), "a");
        debug!(parent: foo.id(), "b");
        info!(parent: foo.id(), "c");
        warn!(parent: foo.id(), "d");
        error!(parent: foo.id(), "e");
    });

    handle.assert_finished();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn string_field() {
    let (collector, handle) = collector::mock()
        .event(expect::event().with_fields(expect::field("my_string").with_value(&"hello").only()))
        .event(
            expect::event().with_fields(
                expect::field("my_string")
                    .with_value(&"hello world!")
                    .only(),
            ),
        )
        .only()
        .run_with_handle();
    with_default(collector, || {
        let mut my_string = String::from("hello");

        tracing::event!(Level::INFO, my_string);

        // the string is not moved by using it as a field!
        my_string.push_str(" world!");

        tracing::event!(Level::INFO, my_string);
    });

    handle.assert_finished();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn constant_field_name() {
    let expect_event = || {
        expect::event().with_fields(
            expect::field("foo")
                .with_value(&"bar")
                .and(expect::field("constant string").with_value(&"also works"))
                .and(expect::field("foo.bar").with_value(&"baz"))
                .and(expect::field("message").with_value(&debug(format_args!("quux"))))
                .only(),
        )
    };
    let (collector, handle) = collector::mock()
        .event(expect_event())
        .event(expect_event())
        .event(expect_event())
        .event(expect_event())
        .event(expect_event())
        .event(expect_event())
        .event(expect_event())
        .event(expect_event())
        .only()
        .run_with_handle();

    with_default(collector, || {
        const FOO: &str = "foo";
        tracing::event!(
            Level::INFO,
            { std::convert::identity(FOO) } = "bar",
            { "constant string" } = "also works",
            foo.bar = "baz",
            "quux"
        );
        tracing::event!(
            Level::INFO,
            {
                { std::convert::identity(FOO) } = "bar",
                { "constant string" } = "also works",
                foo.bar = "baz",
            },
            "quux"
        );
        tracing::info!(
            { std::convert::identity(FOO) } = "bar",
            { "constant string" } = "also works",
            foo.bar = "baz",
            "quux"
        );
        tracing::info!(
            {
                { std::convert::identity(FOO) } = "bar",
                { "constant string" } = "also works",
                foo.bar = "baz",
            },
            "quux"
        );
        tracing::event!(
            Level::INFO,
            { std::convert::identity(FOO) } = "bar",
            { "constant string" } = "also works",
            foo.bar = "baz",
            "{}",
            "quux"
        );
        tracing::event!(
            Level::INFO,
            {
                { std::convert::identity(FOO) } = "bar",
                { "constant string" } = "also works",
                foo.bar = "baz",
            },
            "{}",
            "quux"
        );
        tracing::info!(
            { std::convert::identity(FOO) } = "bar",
            { "constant string" } = "also works",
            foo.bar = "baz",
            "{}",
            "quux"
        );
        tracing::info!(
            {
                { std::convert::identity(FOO) } = "bar",
                { "constant string" } = "also works",
                foo.bar = "baz",
            },
            "{}",
            "quux"
        );
    });

    handle.assert_finished();
}
