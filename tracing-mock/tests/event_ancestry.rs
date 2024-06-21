//! Tests assertions for the parent made on [`ExpectedEvent`].
//!
//! The tests in this module completely cover the positive and negative cases
//! when expecting that an event is a contextual or explicit root or expecting
//! that an event has a specific contextual or explicit parent.
//!
//! [`ExpectedEvent`]: crate::event::ExpectedEvent
use tracing::collect::with_default;
use tracing_mock::{collector, expect};

#[test]
fn contextual_parent() {
    let event = expect::event().with_ancestry(expect::has_contextual_parent("contextual parent"));

    let (collector, handle) = collector::mock()
        .enter(expect::span())
        .event(event)
        .run_with_handle();

    with_default(collector, || {
        let _guard = tracing::info_span!("contextual parent").entered();
        tracing::info!(field = &"value");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to have a contextual parent with name='contextual parent', but \
    actually has a contextual parent with name='another parent'"
)]
fn contextual_parent_wrong_name() {
    let event = expect::event().with_ancestry(expect::has_contextual_parent("contextual parent"));

    let (collector, handle) = collector::mock()
        .enter(expect::span())
        .event(event)
        .run_with_handle();

    with_default(collector, || {
        let _guard = tracing::info_span!("another parent").entered();
        tracing::info!(field = &"value");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to have a contextual parent with name='contextual parent', but was actually a \
    contextual root"
)]
fn expect_contextual_parent_actual_contextual_root() {
    let event = expect::event().with_ancestry(expect::has_contextual_parent("contextual parent"));

    let (collector, handle) = collector::mock().event(event).run_with_handle();

    with_default(collector, || {
        tracing::info!(field = &"value");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to have a contextual parent with name='contextual parent', but actually has an \
    explicit parent with name='explicit parent'"
)]
fn expect_contextual_parent_actual_explicit_parent() {
    let event = expect::event().with_ancestry(expect::has_contextual_parent("contextual parent"));

    let (collector, handle) = collector::mock().event(event).run_with_handle();

    with_default(collector, || {
        let span = tracing::info_span!("explicit parent");
        tracing::info!(parent: span.id(), field = &"value");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to have a contextual parent with name='contextual parent', but was actually an \
    explicit root"
)]
fn expect_contextual_parent_actual_explicit_root() {
    let event = expect::event().with_ancestry(expect::has_contextual_parent("contextual parent"));

    let (collector, handle) = collector::mock()
        .enter(expect::span())
        .event(event)
        .run_with_handle();

    with_default(collector, || {
        let _guard = tracing::info_span!("contextual parent").entered();
        tracing::info!(parent: None, field = &"value");
    });

    handle.assert_finished();
}

#[test]
fn contextual_root() {
    let event = expect::event().with_ancestry(expect::is_contextual_root());

    let (collector, handle) = collector::mock().event(event).run_with_handle();

    with_default(collector, || {
        tracing::info!(field = &"value");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to be a contextual root, but actually has a contextual parent with \
    name='contextual parent'"
)]
fn expect_contextual_root_actual_contextual_parent() {
    let event = expect::event().with_ancestry(expect::is_contextual_root());

    let (collector, handle) = collector::mock()
        .enter(expect::span())
        .event(event)
        .run_with_handle();

    with_default(collector, || {
        let _guard = tracing::info_span!("contextual parent").entered();
        tracing::info!(field = &"value");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to be a contextual root, but actually has an explicit parent with \
    name='explicit parent'"
)]
fn expect_contextual_root_actual_explicit_parent() {
    let event = expect::event().with_ancestry(expect::is_contextual_root());

    let (collector, handle) = collector::mock().event(event).run_with_handle();

    with_default(collector, || {
        let span = tracing::info_span!("explicit parent");
        tracing::info!(parent: span.id(), field = &"value");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(expected = "to be a contextual root, but was actually an explicit root")]
fn expect_contextual_root_actual_explicit_root() {
    let event = expect::event().with_ancestry(expect::is_contextual_root());

    let (collector, handle) = collector::mock()
        .enter(expect::span())
        .event(event)
        .run_with_handle();

    with_default(collector, || {
        let _guard = tracing::info_span!("contextual parent").entered();
        tracing::info!(parent: None, field = &"value");
    });

    handle.assert_finished();
}

#[test]
fn explicit_parent() {
    let event = expect::event().with_ancestry(expect::has_explicit_parent("explicit parent"));

    let (collector, handle) = collector::mock().event(event).run_with_handle();

    with_default(collector, || {
        let span = tracing::info_span!("explicit parent");
        tracing::info!(parent: span.id(), field = &"value");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to have an explicit parent with name='explicit parent', but actually has an \
    explicit parent with name='another parent'"
)]
fn explicit_parent_wrong_name() {
    let event = expect::event().with_ancestry(expect::has_explicit_parent("explicit parent"));

    let (collector, handle) = collector::mock().event(event).run_with_handle();

    with_default(collector, || {
        let span = tracing::info_span!("another parent");
        tracing::info!(parent: span.id(), field = &"value");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to have an explicit parent with name='explicit parent', but actually has a \
    contextual parent with name='contextual parent'"
)]
fn expect_explicit_parent_actual_contextual_parent() {
    let event = expect::event().with_ancestry(expect::has_explicit_parent("explicit parent"));

    let (collector, handle) = collector::mock()
        .enter(expect::span())
        .event(event)
        .run_with_handle();

    with_default(collector, || {
        let _guard = tracing::info_span!("contextual parent").entered();
        tracing::info!(field = &"value");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to have an explicit parent with name='explicit parent', but was actually a \
    contextual root"
)]
fn expect_explicit_parent_actual_contextual_root() {
    let event = expect::event().with_ancestry(expect::has_explicit_parent("explicit parent"));

    let (collector, handle) = collector::mock().event(event).run_with_handle();

    with_default(collector, || {
        tracing::info!(field = &"value");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to have an explicit parent with name='explicit parent', but was actually an \
    explicit root"
)]
fn expect_explicit_parent_actual_explicit_root() {
    let event = expect::event().with_ancestry(expect::has_explicit_parent("explicit parent"));

    let (collector, handle) = collector::mock()
        .enter(expect::span())
        .event(event)
        .run_with_handle();

    with_default(collector, || {
        let _guard = tracing::info_span!("contextual parent").entered();
        tracing::info!(parent: None, field = &"value");
    });

    handle.assert_finished();
}

#[test]
fn explicit_root() {
    let event = expect::event().with_ancestry(expect::is_explicit_root());

    let (collector, handle) = collector::mock()
        .enter(expect::span())
        .event(event)
        .run_with_handle();

    with_default(collector, || {
        let _guard = tracing::info_span!("contextual parent").entered();
        tracing::info!(parent: None, field = &"value");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to be an explicit root, but actually has a contextual parent with \
    name='contextual parent'"
)]
fn expect_explicit_root_actual_contextual_parent() {
    let event = expect::event().with_ancestry(expect::is_explicit_root());

    let (collector, handle) = collector::mock()
        .enter(expect::span())
        .event(event)
        .run_with_handle();

    with_default(collector, || {
        let _guard = tracing::info_span!("contextual parent").entered();
        tracing::info!(field = &"value");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(expected = "to be an explicit root, but was actually a contextual root")]
fn expect_explicit_root_actual_contextual_root() {
    let event = expect::event().with_ancestry(expect::is_explicit_root());

    let (collector, handle) = collector::mock().event(event).run_with_handle();

    with_default(collector, || {
        tracing::info!(field = &"value");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to be an explicit root, but actually has an explicit parent with name='explicit parent'"
)]
fn expect_explicit_root_actual_explicit_parent() {
    let event = expect::event().with_ancestry(expect::is_explicit_root());

    let (collector, handle) = collector::mock().event(event).run_with_handle();

    with_default(collector, || {
        let span = tracing::info_span!("explicit parent");
        tracing::info!(parent: span.id(), field = &"value");
    });

    handle.assert_finished();
}

#[test]
fn explicit_and_contextual_root_is_explicit() {
    let event = expect::event().with_ancestry(expect::is_explicit_root());

    let (collector, handle) = collector::mock().event(event).run_with_handle();

    with_default(collector, || {
        tracing::info!(parent: None, field = &"value");
    });

    handle.assert_finished();
}
