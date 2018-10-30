//! Spans represent periods of time in the execution of a program.
use std::{
    cell::RefCell,
    cmp, fmt,
    hash::{Hash, Hasher},
    iter, slice,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use {
    subscriber::{AddValueError, PriorError, Subscriber},
    value::{IntoValue, OwnedValue},
    DebugFields, Dispatch, StaticMeta,
};

thread_local! {
    static CURRENT_SPAN: RefCell<Option<Active>> = RefCell::new(None);
}

/// A handle that represents a span in the process of executing.
///
/// # Entering a Span
///
/// A thread of execution is said to _enter_ a span when it begins executing,
/// and _exit_ the span when it switches to another context. Spans may be
/// entered through the [`enter`](`Span::enter`) method, which enters the target span,
/// performs a given function (either a closure or a function pointer), exits
/// the span, and then returns the result.
///
/// Calling `enter` on a span handle consumes that handle (as the number of
/// currently extant span handles is used for span completion bookkeeping), but
/// it may be `clone`d inexpensively (span handles are atomically reference
/// counted) in order to enter the span multiple times. For example:
/// ```
/// # #[macro_use] extern crate tokio_trace;
/// # fn main() {
/// let my_var = 5;
/// let my_span = span!("my_span", my_var = &my_var);
///
/// my_span.clone().enter(|| {
///     // perform some work in the context of `my_span`...
/// });
///
/// // Perform some work outside of the context of `my_span`...
///
/// my_span.enter(|| {
///     // Perform some more work in the context of `my_span`.
///     // Since this call to `enter` *consumes* rather than clones `my_span`,
///     // it may not be entered again (unless any more clones of the handle
///     // exist elsewhere). Thus, `my_span` is free to mark itself as "done"
///     // upon exiting.
/// });
/// # }
/// ```
///
/// # The Span Lifecycle
///
/// At any given point in time, a `Span` is in one of four [`State`]s:
/// - `State::Unentered`: The span has been constructed but has not yet been
///   entered for the first time.
/// - `State::Running`: One or more threads are currently executing inside this
///   span or one of its children.
/// - `State::Idle`: The flow of execution has exited the span, but it may be
///   entered again and resume execution.
/// - `State::Done`: The span has completed execution and may not be entered
///   again.
///
/// Spans transition between these states when execution enters and exit them.
/// Upon entry, if a span is not currently in the `Running` state, it will
/// transition to the running state. Upon exit, a span checks if it is executing
/// in any other threads, and if it is not, it transitions to either the `Idle`
/// or `Done` state. The determination of which state to transition to is made
/// based on whether or not the potential exists for the span to be entered
/// again (i.e. whether any `Span` handles with that capability currently
/// exist).
///
/// **Note**: A `Span` handle represents a _single entry_ into the span.
/// Entering a `Span` handle, but a handle may be `clone`d prior to entry if the
/// span expects to be entered again. This is due to how spans determine whether
/// or not to close themselves.
///
/// Rather than requiring the user to _explicitly_ close a span, spans are able
/// to account for their own completion automatically. When a span is exited,
/// the span is responsible for determining whether it should transition back to
/// the `Idle` state, or transition to the `Done` state. This is determined
/// prior to notifying the subscriber that the span has been exited, so that the
/// subscriber can be informed of the state that the span has transitioned to.
/// The next state is chosen based on whether or not the possibility to re-enter
/// the span exists --- namely, are there still handles with the capacity to
/// enter the span? If so, the span transitions back to `Idle`. However, if no
/// more handles exist, the span cannot be entered again; it may instead
/// transition to `Done`.
///
/// Thus, span handles are single-use. Cloning the span handle _signals the
/// intent to enter the span again_.
///
/// # Accessing a Span's Data
///
/// The [`Data`] type represents a *non-entering* reference to a `Span`'s data
/// --- a set of key-value pairs (known as _fields_), a creation timestamp,
/// a reference to the span's parent in the trace tree, and metadata describing
/// the source code location where the span was created. This data is provided
/// to the [`Subscriber`] when the span is created; it may then choose to cache
/// the data for future use, record it in some manner, or discard it completely.
///
/// [`Subscriber`]: ::Subscriber
/// [`State`]: ::span::State
/// [`Data`]: ::span::Data
#[derive(Clone, PartialEq, Hash)]
pub struct Span {
    inner: Option<Active>,
}

/// Representation of the data associated with a span.
///
/// This has the potential to outlive the span itself if it exists after the
/// span completes executing --- such as if it is still being processed by a
/// subscriber.
///
/// This may *not* be used to enter the span.
pub struct Data {
    /// The span ID of the parent span, or `None` if that span does not exist.
    pub parent: Option<Id>,

    /// Metadata describing this span.
    pub static_meta: &'static StaticMeta,

    /// The values of the fields attached to this span.
    ///
    /// These may be `None` if a field was defined but the value has yet to be
    /// attached. The name of the field at each index is defined by
    /// `self.static_meta.field_names[i]`.
    pub field_values: Vec<Option<OwnedValue>>,
}

/// Identifies a span within the context of a process.
///
/// Span IDs are used primarily to determine of two handles refer to the same
/// span, without requiring the comparison of the span's fields.
///
/// They are generated by [`Subscriber`](::Subscriber)s for each span as it is created, through
/// the [`new_span_id`](::Subscriber::new_span_id) trait method. See the documentation for that
/// method for more information on span ID generation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Id(u64);

/// Trait representing something which is associated with a span `Id`.
///
/// This is used primarily to allow [`Span::follows`](::Span::follows) to accept both `Span`s and
/// `Id`s as valid arguments.
pub trait AsId {
    /// Returns the span `Id` that `self` is associated with, or `None` if that
    /// span is disabled.
    fn as_id(&self) -> Option<Id>;
}

#[derive(Clone, Debug, PartialEq, Hash)]
struct Active {
    inner: Arc<ActiveInner>,
}

/// Internal representation of the inner state of a span which has not yet
/// completed.
///
/// This is kept separate from the `Data`, which holds the data about the
/// span, because this type is referenced only by *entering* (`Span`) handles.
/// It is only necessary to track this state while the capacity still exists to
/// re-enter the span; once it can no longer be re-entered, the `ActiveInner`
/// can be dropped (and *should* be dropped, as this may allow the parent span
/// to finish as well, if the `ActiveInner` holds the only remaining entering
/// reference to the parent span).
///
/// This type is purely internal to the `span` module and is not intended to be
/// interacted with directly by downstream users of `tokio-trace`. Instead, all
/// interaction with an active span's state is carried out through `Span`
/// references.
#[derive(Debug)]
struct ActiveInner {
    id: Id,

    /// An entering reference to the span's parent, used to re-enter the parent
    /// span upon exiting this span.
    ///
    /// Implicitly, this also keeps the parent span from becoming `Done` as long
    /// as the child span's `Inner` remains alive.
    enter_parent: Option<Active>,

    /// The number of threads which have entered this span.
    ///
    /// Incremented on enter and decremented on exit.
    currently_entered: AtomicUsize,

    /// The subscriber with which this span was registered.
    /// TODO: it would be nice if this could be any arbitrary `Subscriber`,
    /// rather than `Dispatch`, but object safety.
    subscriber: Dispatch,

    state: AtomicUsize,
}

/// Enumeration of the potential states of a [`Span`](Span).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[repr(usize)]
pub enum State {
    /// The span has been created but has yet to be entered.
    Unentered,
    /// A thread is currently executing inside the span or one of its children.
    Running,
    /// The span has previously been entered, but is not currently
    /// executing. However, it is not done and may be entered again.
    Idle,
    /// The span has completed.
    ///
    /// It will *not* be entered again (and may be dropped once all
    /// subscribers have finished processing it).
    Done,
}

// ===== impl Span =====

impl Span {
    #[doc(hidden)]
    pub fn new(dispatch: Dispatch, static_meta: &'static StaticMeta) -> Span {
        let parent = Active::current();
        let data = Data::new(parent.as_ref().map(Active::id), static_meta);
        let id = dispatch.new_span(data);
        let inner = Some(Active::new(id, dispatch, parent));
        Self { inner }
    }

    /// This is primarily used by the `span!` macro, so it has to be public,
    /// but it's not intended for use by consumers of the tokio-trace API
    /// directly.
    #[doc(hidden)]
    pub fn new_disabled() -> Self {
        Span { inner: None }
    }

    /// Returns a reference to the span that this thread is currently
    /// executing.
    pub fn current() -> Self {
        Self {
            inner: Active::current(),
        }
    }

    /// Returns a reference to the dispatcher that tracks this span, or `None`
    /// if the span is disabled.
    pub(crate) fn dispatch(&self) -> Option<&Dispatch> {
        self.inner.as_ref().map(|inner| &inner.inner.subscriber)
    }

    /// Executes the given function in the context of this span.
    ///
    /// If this span is enabled, then this function enters the span, invokes
    /// and then exits the span. If the span is disabled, `f` will still be
    /// invoked, but in the context of the currently-executing span (if there is
    /// one).
    ///
    /// Returns the result of evaluating `f`.
    pub fn enter<F: FnOnce() -> T, T>(self, f: F) -> T {
        match self.inner {
            Some(inner) => inner.enter(f),
            None => f(),
        }
    }

    /// Returns the `Id` of the parent of this span, if one exists.
    pub fn parent(&self) -> Option<Id> {
        self.inner.as_ref().and_then(Active::parent)
    }

    /// Sets the field on this span named `name` to the given `value`.
    ///
    /// `name` must name a field already defined by this span's metadata, and
    /// the field must not already have a value. If this is not the case, this
    /// function returns an [`AddValueError`](::subscriber::AddValueError).
    pub fn add_value(
        &self,
        field: &'static str,
        value: &dyn IntoValue,
    ) -> Result<(), AddValueError> {
        if let Some(ref inner) = self.inner {
            let inner = &inner.inner;
            match inner.subscriber.add_value(&inner.id, field, value) {
                Ok(()) => Ok(()),
                Err(AddValueError::NoSpan) => panic!("span should still exist!"),
                Err(e) => Err(e),
            }
        } else {
            // If the span doesn't exist, silently do nothing.
            Ok(())
        }
    }

    /// Indicates that the span with the given ID has an indirect causal
    /// relationship with this span.
    ///
    /// This relationship differs somewhat from the parent-child relationship: a
    /// span may have any number of prior spans, rather than a single one; and
    /// spans are not considered to be executing _inside_ of the spans they
    /// follow from. This means that a span may close even if subsequent spans
    /// that follow from it are still open, and time spent inside of a
    /// subsequent span should not be included in the time its precedents were
    /// executing. This is used to model causal relationships such as when a
    /// single future spawns several related background tasks, et cetera.
    ///
    /// If this span is disabled, this function will do nothing. Otherwise, it
    /// returns `Ok(())` if the other span was added as a precedent of this
    /// span, or an error if this was not possible.
    pub fn follows<I: AsId>(&self, from: I) -> Result<(), PriorError> {
        if let Some(ref inner) = self.inner {
            let from_id = from.as_id().ok_or(PriorError::NoPreceedingId)?;
            let inner = &inner.inner;
            match inner.subscriber.add_prior_span(&inner.id, from_id) {
                Ok(()) => Ok(()),
                Err(PriorError::NoSpan(ref id)) if id == &inner.id => {
                    panic!("span {:?} should exist to add a preceeding span", inner.id)
                }
                Err(e) => Err(e),
            }
        } else {
            // If the span doesn't exist, silently do nothing.
            Ok(())
        }
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut span = f.debug_struct("Span");
        if let Some(ref inner) = self.inner {
            span.field("id", &inner.id())
                .field("parent", &inner.parent())
                .field("state", &inner.state())
                .field("is_last_standing", &inner.is_last_standing())
        } else {
            span.field("disabled", &true)
        }.finish()
    }
}

impl AsId for Span {
    fn as_id(&self) -> Option<Id> {
        self.inner.as_ref().map(Active::id)
    }
}

// ===== impl Data =====

impl Data {
    fn new(parent: Option<Id>, static_meta: &'static StaticMeta) -> Self {
        // Preallocate enough `None`s to hold the unset state of every field
        // name.
        let field_values = iter::repeat(())
            .map(|_| None)
            .take(static_meta.field_names.len())
            .collect();
        Data {
            parent,
            static_meta,
            field_values,
        }
    }

    /// Returns the name of this span, or `None` if it is unnamed,
    pub fn name(&self) -> Option<&'static str> {
        self.static_meta.name
    }

    /// Returns the `Id` of the parent of this span, if one exists.
    pub fn parent(&self) -> Option<&Id> {
        self.parent.as_ref()
    }

    /// Borrows this span's metadata.
    pub fn meta(&self) -> &'static StaticMeta {
        self.static_meta
    }

    /// Returns an iterator over the names of all the fields on this span.
    pub fn field_names<'a>(&self) -> slice::Iter<&'a str> {
        self.static_meta.field_names.iter()
    }

    /// Returns true if a field named 'name' has been declared on this span,
    /// even if the field does not currently have a value.
    pub fn has_field<Q>(&self, key: Q) -> bool
    where
        &'static str: PartialEq<Q>,
    {
        self.field_names().any(|&name| name == key)
    }

    /// Borrows the value of the field named `name`, if it exists. Otherwise,
    /// returns `None`.
    pub fn field<Q>(&self, key: Q) -> Option<&OwnedValue>
    where
        &'static str: PartialEq<Q>,
    {
        self.field_names()
            .position(|&field_name| field_name == key)
            .and_then(|i| self.field_values.get(i)?.as_ref())
    }

    /// Returns an iterator over all the field names and values on this span.
    pub fn fields<'a>(&'a self) -> impl Iterator<Item = (&'a str, &'a OwnedValue)> {
        self.field_names()
            .filter_map(move |&name| self.field(name).map(move |val| (name, val)))
    }

    /// Edits the span data to add the given `value` to the field named `name`.
    ///
    /// `name` must name a field already defined by this span's metadata, and
    /// the field must not already have a value. If this is not the case, this
    /// function returns an [`AddValueError`](::subscriber::AddValueError).
    pub fn add_value(
        &mut self,
        name: &'static str,
        value: &dyn IntoValue,
    ) -> Result<(), AddValueError> {
        if let Some(i) = self
            .field_names()
            .position(|&field_name| field_name == name)
        {
            let field = &mut self.field_values[i];
            if field.is_some() {
                Err(AddValueError::FieldAlreadyExists)
            } else {
                *field = Some(value.into_value());
                Ok(())
            }
        } else {
            Err(AddValueError::NoField)
        }
    }

    /// Returns a struct that can be used to format all the fields on this
    /// span with `fmt::Debug`.
    pub fn debug_fields<'a>(&'a self) -> DebugFields<'a, Self, &'a OwnedValue> {
        DebugFields(self)
    }
}

impl<'a> IntoIterator for &'a Data {
    type Item = (&'a str, &'a OwnedValue);
    type IntoIter = Box<Iterator<Item = (&'a str, &'a OwnedValue)> + 'a>; // TODO: unbox
    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.fields())
    }
}

impl fmt::Debug for Data {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Span")
            .field("name", &self.name())
            .field("parent", &self.parent)
            .field("fields", &self.debug_fields())
            .field("meta", &self.meta())
            .finish()
    }
}

// ===== impl Id =====

impl Id {
    /// Constructs a new span ID from the given `u64`.
    pub fn from_u64(u: u64) -> Self {
        Id(u)
    }

    /// Returns the ID of the currently-executing span.
    pub fn current() -> Option<Self> {
        Active::current().as_ref().map(Active::id)
    }
}

impl AsId for Id {
    fn as_id(&self) -> Option<Id> {
        Some(self.clone())
    }
}

// ===== impl Active =====

impl Active {
    fn current() -> Option<Self> {
        CURRENT_SPAN.with(|span| span.borrow().as_ref().cloned())
    }

    fn new(id: Id, subscriber: Dispatch, enter_parent: Option<Self>) -> Self {
        let inner = Arc::new(ActiveInner {
            id,
            enter_parent,
            currently_entered: AtomicUsize::new(0),
            state: AtomicUsize::new(State::Unentered as usize),
            subscriber,
        });
        Self { inner }
    }

    fn enter<F: FnOnce() -> T, T>(self, f: F) -> T {
        let prior_state = self.state();
        match prior_state {
            // The span has been marked as done; it may not be reentered again.
            // TODO: maybe this should not crash the thread?
            State::Done => panic!("cannot re-enter completed span!"),
            _ => {
                let result = CURRENT_SPAN.with(|current_span| {
                    self.inner.transition_on_enter(prior_state);
                    current_span.replace(Some(self.clone()));
                    self.inner.subscriber.enter(self.id(), self.state());
                    f()
                });

                CURRENT_SPAN.with(|current_span| {
                    current_span.replace(self.inner.enter_parent.as_ref().cloned());
                    // If we are the only remaining enter handle to this
                    // span, it can now transition to Done. Otherwise, it
                    // transitions to Idle.
                    let next_state = if self.is_last_standing() {
                        // Dropping this span handle will drop the enterable
                        // reference to self.parent.
                        State::Done
                    } else {
                        State::Idle
                    };
                    self.inner.transition_on_exit(next_state);
                    self.inner.subscriber.exit(self.id(), self.state());
                });
                result
            }
        }
    }

    /// Returns true if this is the last remaining handle with the capacity to
    /// enter the span.
    ///
    /// Used to determine when the span can be marked as completed.
    fn is_last_standing(&self) -> bool {
        Arc::strong_count(&self.inner) == 1
    }

    fn id(&self) -> Id {
        self.inner.id.clone()
    }

    fn state(&self) -> State {
        self.inner.state()
    }

    fn parent(&self) -> Option<Id> {
        self.inner.enter_parent.as_ref().map(Active::id)
    }
}

// ===== impl ActiveInnInner =====

impl ActiveInner {
    /// Returns the current [`State`](::span::State) of this span.
    pub fn state(&self) -> State {
        match self.state.load(Ordering::Acquire) {
            s if s == State::Unentered as usize => State::Unentered,
            s if s == State::Running as usize => State::Running,
            s if s == State::Idle as usize => State::Idle,
            s if s == State::Done as usize => State::Done,
            invalid => panic!("invalid state: {:?}", invalid),
        }
    }

    fn set_state(&self, prev: State, next: State) {
        self.state
            .compare_and_swap(prev as usize, next as usize, Ordering::Release);
    }

    /// Performs the state transition when entering the span.
    fn transition_on_enter(&self, from_state: State) {
        self.currently_entered.fetch_add(1, Ordering::Release);
        self.set_state(from_state, State::Running);
    }

    /// Performs the state transition when exiting the span.
    fn transition_on_exit(&self, next_state: State) {
        // Decrement the exit count
        let remaining_exits = self.currently_entered.fetch_sub(1, Ordering::AcqRel);
        // Only advance the state if we are the last remaining
        // thread to exit the span.
        if remaining_exits == 1 {
            self.set_state(State::Running, next_state);
        }
    }
}

impl cmp::PartialEq for ActiveInner {
    fn eq(&self, other: &ActiveInner) -> bool {
        self.id == other.id
    }
}

impl Hash for ActiveInner {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

// ===== impl DataInner =====

#[cfg(any(test, feature = "test-support"))]
pub use self::test_support::*;

#[cfg(any(test, feature = "test-support"))]
mod test_support {
    #![allow(missing_docs)]
    use std::collections::HashMap;
    use {span::State, value::OwnedValue};

    /// A mock span.
    ///
    /// This is intended for use with the mock subscriber API in the
    /// `subscriber` module.
    pub struct MockSpan {
        pub name: Option<Option<&'static str>>,
        pub state: Option<State>,
        pub fields: HashMap<String, Box<OwnedValue>>,
        // TODO: more
    }

    pub fn mock() -> MockSpan {
        MockSpan {
            name: None,
            state: None,
            fields: HashMap::new(),
        }
    }

    impl MockSpan {
        pub fn named(mut self, name: Option<&'static str>) -> Self {
            self.name = Some(name);
            self
        }

        pub fn with_state(mut self, state: State) -> Self {
            self.state = Some(state);
            self
        }

        // TODO: fields, etc
    }
}

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
