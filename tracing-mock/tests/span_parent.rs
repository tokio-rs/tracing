//! Tests assertions for the parent made on [`ExpectedSpan`].
//!
//! The tests in this module completely cover the positive and negative cases
//! when expecting that a span is a contextual or explicit root or expecting
//! that a span has a specific contextual or explicit parent.
//!
//! [`ExpectedSpan`]: crate::span::ExpectedSpan
//!
use tracing::collect::with_default;
use tracing_mock::{collector, expect};

#[test]
fn contextual_parent() {
    let span = expect::span()
        .named("span")
        .with_contextual_parent(Some("contextual parent"));

    let (collector, handle) = collector::mock()
        .enter(expect::span())
        .new_span(span)
        .run_with_handle();

    with_default(collector, || {
        let _guard = tracing::info_span!("contextual parent").entered();
        tracing::info_span!("span");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to have a contextual parent with name='contextual parent', but \
    actually has a contextual parent with name='another parent'"
)]
fn contextual_parent_wrong_name() {
    let span = expect::span()
        .named("span")
        .with_contextual_parent(Some("contextual parent"));

    let (collector, handle) = collector::mock()
        .enter(expect::span())
        .new_span(span)
        .run_with_handle();

    with_default(collector, || {
        let _guard = tracing::info_span!("another parent").entered();
        tracing::info_span!("span");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to have a contextual parent with name='contextual parent', but was actually a \
    contextual root"
)]
fn expect_contextual_parent_actual_contextual_root() {
    let span = expect::span()
        .named("span")
        .with_contextual_parent(Some("contextual parent"));

    let (collector, handle) = collector::mock().new_span(span).run_with_handle();

    with_default(collector, || {
        tracing::info_span!("span");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to have a contextual parent with name='contextual parent', but actually has an \
    explicit parent with name='explicit parent'"
)]
fn expect_contextual_parent_actual_explicit_parent() {
    let span = expect::span()
        .named("span")
        .with_contextual_parent(Some("contextual parent"));

    let (collector, handle) = collector::mock()
        .new_span(expect::span())
        .new_span(span)
        .run_with_handle();

    with_default(collector, || {
        let span = tracing::info_span!("explicit parent");
        tracing::info_span!(parent: span.id(), "span");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to have a contextual parent with name='contextual parent', but was actually an \
    explicit root"
)]
fn expect_contextual_parent_actual_explicit_root() {
    let span = expect::span()
        .named("span")
        .with_contextual_parent(Some("contextual parent"));

    let (collector, handle) = collector::mock()
        .enter(expect::span())
        .new_span(span)
        .run_with_handle();

    with_default(collector, || {
        let _guard = tracing::info_span!("contextual parent").entered();
        tracing::info_span!(parent: None, "span");
    });

    handle.assert_finished();
}

#[test]
fn contextual_root() {
    let span = expect::span().named("span").with_contextual_parent(None);

    let (collector, handle) = collector::mock().new_span(span).run_with_handle();

    with_default(collector, || {
        tracing::info_span!("span");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to be a contextual root, but actually has a contextual parent with \
    name='contextual parent'"
)]
fn expect_contextual_root_actual_contextual_parent() {
    let span = expect::span().named("span").with_contextual_parent(None);

    let (collector, handle) = collector::mock()
        .enter(expect::span())
        .new_span(span)
        .run_with_handle();

    with_default(collector, || {
        let _guard = tracing::info_span!("contextual parent").entered();
        tracing::info_span!("span");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to be a contextual root, but actually has an explicit parent with \
    name='explicit parent'"
)]
fn expect_contextual_root_actual_explicit_parent() {
    let span = expect::span().named("span").with_contextual_parent(None);

    let (collector, handle) = collector::mock()
        .new_span(expect::span())
        .new_span(span)
        .run_with_handle();

    with_default(collector, || {
        let span = tracing::info_span!("explicit parent");
        tracing::info_span!(parent: span.id(), "span");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(expected = "to be a contextual root, but was actually an explicit root")]
fn expect_contextual_root_actual_explicit_root() {
    let span = expect::span().named("span").with_contextual_parent(None);

    let (collector, handle) = collector::mock()
        .enter(expect::span())
        .new_span(span)
        .run_with_handle();

    with_default(collector, || {
        let _guard = tracing::info_span!("contextual parent").entered();
        tracing::info_span!(parent: None, "span");
    });

    handle.assert_finished();
}

#[test]
fn explicit_parent() {
    let span = expect::span()
        .named("span")
        .with_explicit_parent(Some("explicit parent"));

    let (collector, handle) = collector::mock()
        .new_span(expect::span())
        .new_span(span)
        .run_with_handle();

    with_default(collector, || {
        let span = tracing::info_span!("explicit parent");
        tracing::info_span!(parent: span.id(), "span");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to have an explicit parent with name='explicit parent', but actually has an \
    explicit parent with name='another parent'"
)]
fn explicit_parent_wrong_name() {
    let span = expect::span()
        .named("span")
        .with_explicit_parent(Some("explicit parent"));

    let (collector, handle) = collector::mock()
        .new_span(expect::span())
        .new_span(span)
        .run_with_handle();

    with_default(collector, || {
        let span = tracing::info_span!("another parent");
        tracing::info_span!(parent: span.id(), "span");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to have an explicit parent with name='explicit parent', but actually has a \
    contextual parent with name='contextual parent'"
)]
fn expect_explicit_parent_actual_contextual_parent() {
    let span = expect::span()
        .named("span")
        .with_explicit_parent(Some("explicit parent"));

    let (collector, handle) = collector::mock()
        .enter(expect::span())
        .new_span(span)
        .run_with_handle();

    with_default(collector, || {
        let _guard = tracing::info_span!("contextual parent").entered();
        tracing::info_span!("span");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to have an explicit parent with name='explicit parent', but was actually a \
    contextual root"
)]
fn expect_explicit_parent_actual_contextual_root() {
    let span = expect::span()
        .named("span")
        .with_explicit_parent(Some("explicit parent"));

    let (collector, handle) = collector::mock().new_span(span).run_with_handle();

    with_default(collector, || {
        tracing::info_span!("span");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to have an explicit parent with name='explicit parent', but was actually an \
    explicit root"
)]
fn expect_explicit_parent_actual_explicit_root() {
    let span = expect::span()
        .named("span")
        .with_explicit_parent(Some("explicit parent"));

    let (collector, handle) = collector::mock()
        .enter(expect::span())
        .new_span(span)
        .run_with_handle();

    with_default(collector, || {
        let _guard = tracing::info_span!("contextual parent").entered();
        tracing::info_span!(parent: None, "span");
    });

    handle.assert_finished();
}

#[test]
fn explicit_root() {
    let span = expect::span().named("span").with_explicit_parent(None);

    let (collector, handle) = collector::mock()
        .new_span(expect::span())
        .enter(expect::span())
        .new_span(span)
        .run_with_handle();

    with_default(collector, || {
        let _guard = tracing::info_span!("contextual parent").entered();
        tracing::info_span!(parent: None, "span");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to be an explicit root, but actually has a contextual parent with \
    name='contextual parent'"
)]
fn expect_explicit_root_actual_contextual_parent() {
    let span = expect::span().named("span").with_explicit_parent(None);

    let (collector, handle) = collector::mock()
        .enter(expect::span())
        .new_span(span)
        .run_with_handle();

    with_default(collector, || {
        let _guard = tracing::info_span!("contextual parent").entered();
        tracing::info_span!("span");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(expected = "to be an explicit root, but was actually a contextual root")]
fn expect_explicit_root_actual_contextual_root() {
    let span = expect::span().named("span").with_explicit_parent(None);

    let (collector, handle) = collector::mock().new_span(span).run_with_handle();

    with_default(collector, || {
        tracing::info_span!("span");
    });

    handle.assert_finished();
}

#[test]
#[should_panic(
    expected = "to be an explicit root, but actually has an explicit parent with name='explicit parent'"
)]
fn expect_explicit_root_actual_explicit_parent() {
    let span = expect::span().named("span").with_explicit_parent(None);

    let (collector, handle) = collector::mock()
        .new_span(expect::span())
        .new_span(span)
        .run_with_handle();

    with_default(collector, || {
        let span = tracing::info_span!("explicit parent");
        tracing::info_span!(parent: span.id(), "span");
    });

    handle.assert_finished();
}

#[test]
fn explicit_and_contextual_root_is_explicit() {
    let span = expect::span().named("span").with_explicit_parent(None);

    let (collector, handle) = collector::mock().new_span(span).run_with_handle();

    with_default(collector, || {
        tracing::info_span!(parent: None, "span");
    });

    handle.assert_finished();
}
