//! Spans represent periods of time in the execution of a program.
//!
//! # Entering a Span
//!
//! A thread of execution is said to _enter_ a span when it begins executing,
//! and _exit_ the span when it switches to another context. Spans may be
//! entered through the [`enter`](`Span::enter`) method, which enters the target span,
//! performs a given function (either a closure or a function pointer), exits
//! the span, and then returns the result.
//!
//! Calling `enter` on a span handle consumes that handle (as the number of
//! currently extant span handles is used for span completion bookkeeping), but
//! it may be `clone`d inexpensively (span handles are atomically reference
//! counted) in order to enter the span multiple times. For example:
//! ```
//! # #[macro_use] extern crate tokio_trace;
//! # fn main() {
//! let my_var = 5;
//! let my_span = span!("my_span", my_var = &my_var);
//!
//! my_span.clone().enter(|| {
//!     // perform some work in the context of `my_span`...
//! });
//!
//! // Perform some work outside of the context of `my_span`...
//!
//! my_span.enter(|| {
//!     // Perform some more work in the context of `my_span`.
//!     // Since this call to `enter` *consumes* rather than clones `my_span`,
//!     // it may not be entered again (unless any more clones of the handle
//!     // exist elsewhere). Thus, `my_span` is free to mark itself as "done"
//!     // upon exiting.
//! });
//! # }
//! ```
//!
//! # The Span Lifecycle
//!
//! At any given point in time, a `Span` is in one of four [`State`]s:
//! - `State::Unentered`: The span has been constructed but has not yet been
//!   entered for the first time.
//! - `State::Running`: One or more threads are currently executing inside this
//!   span or one of its children.
//! - `State::Idle`: The flow of execution has exited the span, but it may be
//!   entered again and resume execution.
//! - `State::Done`: The span has completed execution and may not be entered
//!   again.
//!
//! Spans transition between these states when execution enters and exit them.
//! Upon entry, if a span is not currently in the `Running` state, it will
//! transition to the running state. Upon exit, a span checks if it is executing
//! in any other threads, and if it is not, it transitions to either the `Idle`
//! or `Done` state. The determination of which state to transition to is made
//! based on whether or not the potential exists for the span to be entered
//! again (i.e. whether any `Span` handles with that capability currently
//! exist).
//!
//! **Note**: A `Span` handle represents a _single entry_ into the span.
//! Entering a `Span` handle, but a handle may be `clone`d prior to entry if the
//! span expects to be entered again. This is due to how spans determine whether
//! or not to close themselves.
//!
//! Rather than requiring the user to _explicitly_ close a span, spans are able
//! to account for their own completion automatically. When a span is exited,
//! the span is responsible for determining whether it should transition back to
//! the `Idle` state, or transition to the `Done` state. This is determined
//! prior to notifying the subscriber that the span has been exited, so that the
//! subscriber can be informed of the state that the span has transitioned to.
//! The next state is chosen based on whether or not the possibility to re-enter
//! the span exists --- namely, are there still handles with the capacity to
//! enter the span? If so, the span transitions back to `Idle`. However, if no
//! more handles exist, the span cannot be entered again; it may instead
//! transition to `Done`.
//!
//! Thus, span handles are single-use. Cloning the span handle _signals the
//! intent to enter the span again_.
//!
//! # Accessing a Span's Data
//!
//! The [`Data`] type represents a *non-entering* reference to a `Span`'s data
//! --- a set of key-value pairs (known as _fields_), a creation timestamp,
//! a reference to the span's parent in the trace tree, and metadata describing
//! the source code location where the span was created. This data is provided
//! to the [`Subscriber`] when the span is created; it may then choose to cache
//! the data for future use, record it in some manner, or discard it completely.
//!
//! [`Subscriber`]: ::Subscriber
//! [`State`]: ::span::State
//! [`Data`]: ::span::Data
pub use tokio_trace_core::span::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use {span, subscriber, Dispatch};

    #[test]
    fn exit_doesnt_finish_while_handles_still_exist() {
        // Test that exiting a span only marks it as "done" when no handles
        // that can re-enter the span exist.
        let subscriber = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("bar")))
            // The first time we exit "bar", there will be another handle with
            // which we could potentially re-enter bar.
            .exit(span::mock().named(Some("bar")).with_state(State::Idle))
            // Re-enter "bar", using the cloned handle.
            .enter(span::mock().named(Some("bar")))
            // Now, when we exit "bar", there is no handle to re-enter it, so
            // it should become "done".
            .exit(span::mock().named(Some("bar")).with_state(State::Done))
            // "foo" never had more than one handle, so it should also become
            // "done" when we exit it.
            .exit(span::mock().named(Some("foo")).with_state(State::Done))
            .run();

        Dispatch::to(subscriber).as_default(|| {
            span!("foo",).enter(|| {
                let bar = span!("bar",);
                bar.clone().enter(|| {
                    // do nothing. exiting "bar" should leave it idle, since it can
                    // be re-entered.
                });
                bar.enter(|| {
                    // enter "bar" again. this time, the last handle is used, so
                    // "bar" should be marked as done.
                });
            });
        });
    }

    #[test]
    fn exit_doesnt_finish_concurrently_executing_spans() {
        // Test that exiting a span only marks it as "done" when no other
        // threads are still executing inside that span.
        use std::sync::{Arc, Barrier};

        let subscriber = subscriber::mock()
            .enter(span::mock().named(Some("baz")))
            // Main thread enters "quux".
            .enter(span::mock().named(Some("quux")))
            // Spawned thread also enters "quux".
            .enter(span::mock().named(Some("quux")))
            // When the main thread exits "quux", it will still be running in the
            // spawned thread.
            .exit(span::mock().named(Some("quux")).with_state(State::Running))
            // Now, when this thread exits "quux", there is no handle to re-enter it, so
            // it should become "done".
            .exit(span::mock().named(Some("quux")).with_state(State::Done))
            // "baz" never had more than one handle, so it should also become
            // "done" when we exit it.
            .exit(span::mock().named(Some("baz")).with_state(State::Done))
            .run();

        Dispatch::to(subscriber).as_default(|| {
            let barrier1 = Arc::new(Barrier::new(2));
            let barrier2 = Arc::new(Barrier::new(2));
            // Make copies of the barriers for thread 2 to wait on.
            let t2_barrier1 = barrier1.clone();
            let t2_barrier2 = barrier2.clone();

            span!("baz",).enter(move || {
                let quux = span!("quux",);
                let quux2 = quux.clone();
                let handle = thread::Builder::new()
                    .name("thread-2".to_string())
                    .spawn(move || {
                        quux2.enter(|| {
                            // Once this thread has entered "quux", allow thread 1
                            // to exit.
                            t2_barrier1.wait();
                            // Wait for the main thread to allow us to exit.
                            t2_barrier2.wait();
                        })
                    }).expect("spawn test thread");
                quux.enter(|| {
                    // Wait for thread 2 to enter "quux". When we exit "quux", it
                    // should stay running, since it's running in the other thread.
                    barrier1.wait();
                });
                // After we exit "quux", wait for the second barrier, so the other
                // thread unblocks and exits "quux".
                barrier2.wait();
                handle.join().unwrap();
            });
        });
    }

    #[test]
    fn handles_to_the_same_span_are_equal() {
        // Create a mock subscriber that will return `true` on calls to
        // `Subscriber::enabled`, so that the spans will be constructed. We
        // won't enter any spans in this test, so the subscriber won't actually
        // expect to see any spans.
        Dispatch::to(subscriber::mock().run()).as_default(|| {
            let foo1 = span!("foo");
            let foo2 = foo1.clone();

            // Two handles that point to the same span are equal.
            assert_eq!(foo1, foo2);

            // // The two span's data handles are also equal.
            // assert_eq!(foo1.data(), foo2.data());
        });
    }

    #[test]
    fn handles_to_different_spans_are_not_equal() {
        Dispatch::to(subscriber::mock().run()).as_default(|| {
            // Even though these spans have the same name and fields, they will have
            // differing metadata, since they were created on different lines.
            let foo1 = span!("foo", bar = &1, baz = &false);
            let foo2 = span!("foo", bar = &1, baz = &false);

            assert_ne!(foo1, foo2);
            // assert_ne!(foo1.data(), foo2.data());
        });
    }

    #[test]
    fn handles_to_different_spans_with_the_same_metadata_are_not_equal() {
        // Every time time this function is called, it will return a _new
        // instance_ of a span with the same metadata, name, and fields.
        fn make_span() -> Span {
            span!("foo", bar = &1, baz = &false)
        }

        Dispatch::to(subscriber::mock().run()).as_default(|| {
            let foo1 = make_span();
            let foo2 = make_span();

            assert_ne!(foo1, foo2);
            // assert_ne!(foo1.data(), foo2.data());
        });
    }

    #[test]
    fn spans_always_go_to_the_subscriber_that_tagged_them() {
        let subscriber1 = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")).with_state(State::Idle))
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")).with_state(State::Done));
        let subscriber1 = Dispatch::to(subscriber1.run());
        let subscriber2 = Dispatch::to(subscriber::mock().run());

        let foo = subscriber1.as_default(|| {
            let foo = span!("foo");
            foo.clone().enter(|| {});
            foo
        });
        // Even though we enter subscriber 2's context, the subscriber that
        // tagged the span should see the enter/exit.
        subscriber2.as_default(move || foo.enter(|| {}));
    }

    #[test]
    fn spans_always_go_to_the_subscriber_that_tagged_them_even_across_threads() {
        let subscriber1 = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")).with_state(State::Idle))
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")).with_state(State::Done));
        let subscriber1 = Dispatch::to(subscriber1.run());
        let foo = subscriber1.as_default(|| {
            let foo = span!("foo");
            foo.clone().enter(|| {});
            foo
        });

        // Even though we enter subscriber 2's context, the subscriber that
        // tagged the span should see the enter/exit.
        thread::spawn(move || {
            Dispatch::to(subscriber::mock().run()).as_default(|| {
                foo.enter(|| {});
            })
        }).join()
        .unwrap();
    }
}
