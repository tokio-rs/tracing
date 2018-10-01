use super::{DebugFields, Dispatcher, StaticMeta, Subscriber, Value, Parents, dedup::IteratorDedup};
use std::{
    cell::RefCell,
    cmp, fmt,
    hash::{Hash, Hasher},
    slice,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

lazy_static! {
    static ref ROOT_SPAN: Active = Active::new(
        Data::new(
            Instant::now(),
            None,
            &static_meta!("root"),
            Vec::new(),
        ),
        None
    );
}

thread_local! {
    static CURRENT_SPAN: RefCell<Active> = RefCell::new(ROOT_SPAN.clone());
}

/// A handle that represents a span in the process of executing.
///
/// # Entering a Span
///
/// A thread of execution is said to _enter_ a span when it begins executing,
/// and _exit_ the span when it switches to another context. Spans may be
/// entered through the [`Span::enter`] method, which enters the target span,
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
/// let my_span = span!("my_span", my_var = my_var);
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
/// # Span States
///
/// At any given point in time, a `Span` is in one of four [`State`]s:
/// - `State::Unentered`: The span has been constructed but has not yet been
///   entered for the first time.
/// - `State::Running`: One or more threads are currently executing inside this
///   span or one of its children.
/// - `State::Idle`: The flow of execution has exited the span, but it may be
///   entered again and resume execution.
/// - `State::Done`: The span has completed execution and may not be entered
///   again. When all subscribers have finished using its' [`Data`], it may be
///   dropped.
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
/// # Accessing a Span's Data
///
/// The [`Data`] type represents a *non-entering* reference to a `Span`'s data
/// --- a set of key-value pairs (known as _fields_), a creation timestamp,
/// a reference to the span's parent in the trace tree, and metadata describing
/// the source code location where the span was created.
///
/// Since we may wish to view the data associated with a span *after* the span
/// has finished executing, the `Data` handle is separated from the `Span`
/// handle that permits entry into the span. However, `Span` handles can also be
/// dereferenced to `Data` handles, if we wish to view the data while the span
/// is still "alive". The data is reference-counted separately from the
/// capability to enter the span, so once all the `Span` handles have been
/// dropped and the span marks itself as completed, the data remains alive until
/// all the [`Data`] handles have *also* been dropped.
#[derive(Clone, PartialEq, Hash)]
pub struct Span {
    inner: Option<Active>,
}

/// A handle on the data associated with a span.
///
/// This may be used to access the span but may *not* be used to enter the span.
#[derive(Clone, PartialEq, Hash)]
pub struct Data {
    inner: Arc<DataInner>,
}

/// Internal representation of the data associated with a span.
///
/// This has the potential to outlive the span itself, if a `Data` reference to
/// it exists after the span completes executing --- such as if it is still
/// being processed by a subscriber.
///
/// This type is purely internal to the `span` module and is not intended to be
/// interacted with directly by downstream users of `tokio-trace`. Instead, all
/// interaction with a span's data is carried out through `Data` references.
#[derive(Debug)]
struct DataInner {
    pub opened_at: Instant,

    pub parent: Option<Data>,

    pub static_meta: &'static StaticMeta,

    pub field_values: Vec<Box<dyn Value>>,

    state: AtomicUsize,
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
/// The span's `DataInner` is reference-counted eparately, and may remain alive
/// after the span has completed and the active span state has been dropped.
/// For example, if a `Data` reference has been cloned by a subscriber to be
/// processed later, the data must outlive the active span state.
///
/// This type is purely internal to the `span` module and is not intended to be
/// interacted with directly by downstream users of `tokio-trace`. Instead, all
/// interaction with an active span's state is carried out through `Span`
/// references.
#[derive(Debug)]
struct ActiveInner {
    /// A reference to the span's associated data.
    data: Data,

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
}

/// Enumeration of the potential states of a [`Span`].
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
    /// This is primarily used by the `span!` macro, so it has to be public,
    /// but it's not intended for use by consumers of the tokio-trace API
    /// directly.
    #[doc(hidden)]
    pub fn new(
        opened_at: Instant,
        static_meta: &'static StaticMeta,
        field_values: Vec<Box<dyn Value>>,
    ) -> Self {
        let parent = Active::current();
        let data = Data::new(
            opened_at,
            Some(parent.data()),
            static_meta,
            field_values,
        );
        let inner = Some(Active::new(data, Some(parent)));
        Span {
            inner,
        }
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
        let inner = Some(Active::current());
        Self { inner }
    }

    pub fn enter<F: FnOnce() -> T, T>(self, f: F) -> T {
        match self.inner {
            Some(inner) => inner.enter(f),
            None => f(),
        }
    }

    pub fn data(&self) -> Option<&Data> {
        self.inner.as_ref().map(Active::data)
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(inner) = self.data() {
            f.debug_struct("Span")
                .field("name", &inner.name())
                .field("opened_at", &inner.opened_at())
                .field("parent", &inner.parent().map(Data::name))
                .field("fields", &inner.debug_fields())
                .field("meta", &inner.meta())
                .finish()
        } else {
            f.debug_struct("Span").field("disabled", &true).finish()
        }

    }
}

impl<'a> Into<Option<&'a Data>> for &'a Span {
    fn into(self) -> Option<&'a Data> {
        self.data()
    }
}

impl Into<Option<Data>> for Span {
    fn into(self) -> Option<Data> {
        self.data().cloned()
    }
}

// ===== impl Data =====

impl Data {
    fn new(
        opened_at: Instant,
        parent: Option<&Data>,
        static_meta: &'static StaticMeta,
        field_values: Vec<Box<dyn Value>>,
    ) -> Self {
        Data {
            inner: Arc::new(DataInner {
                opened_at,
                parent: parent.cloned(),
                static_meta,
                field_values,
                state: AtomicUsize::new(0),
            })
        }
    }

    pub fn current() -> Self {
        CURRENT_SPAN.with(|current| {
            current.borrow().data().clone()
        })
    }

    /// Returns the name of this span, or `None` if it is unnamed,
    pub fn name(&self) -> Option<&'static str> {
        self.inner.static_meta.name
    }

    /// Returns a `Data` reference to the parent of this span, if one exists.
    pub fn parent(&self) -> Option<&Data> {
        self.inner.parent.as_ref()
    }

    /// Borrows this span's metadata.
    pub fn meta(&self) -> &'static StaticMeta {
        self.inner.static_meta
    }

    /// Returns an iterator over the names of all the fields on this span.
    pub fn field_names<'a>(&self) -> slice::Iter<&'a str> {
        self.inner.static_meta.field_names.iter()
    }

    /// Returns the `Instant` at which this span was created.
    pub fn opened_at(&self) -> Instant {
        self.inner.opened_at
    }

    /// Borrows the value of the field named `name`, if it exists. Otherwise,
    /// returns `None`.
    pub fn field<Q>(&self, key: Q) -> Option<&dyn Value>
    where
        &'static str: PartialEq<Q>,
    {
        self.field_names()
            .position(|&field_name| field_name == key)
            .and_then(|i| self.inner
                .field_values
                .get(i)
                .map(AsRef::as_ref))
    }

    /// Returns an iterator over all the field names and values on this span.
    pub fn fields<'a>(&'a self) -> impl Iterator<Item = (&'a str, &'a dyn Value)> {
        self.field_names()
            .enumerate()
            .map(move |(idx, &name)| (name, self.inner.field_values[idx].as_ref()))
    }

    /// Returns a struct that can be used to format all the fields on this
    /// span ith `fmt::Debug`.
    pub fn debug_fields<'a>(&'a self) -> DebugFields<'a, Self> {
        DebugFields(self)
    }

    /// Returns an iterator over [`Data`] references to all the spans that are
    /// parents of this span.
    ///
    /// The iterator will traverse the trace tree in ascending order from this
    /// span's immediate parent to the root span of the trace.
    pub fn parents<'a>(&'a self) -> Parents<'a> {
        Parents {
            next: self.parent(),
        }
    }

    /// Returns an iterator over all the field names and values of this span
    /// and all of its parent spans.
    ///
    /// Fields with duplicate names are skipped, and the value defined lowest
    /// in the tree is used. For example:
    /// ```
    /// # #[macro_use]
    /// # extern crate tokio_trace;
    /// # use tokio_trace::Level;
    /// # fn main() {
    /// span!("parent 1", foo = 1, bar = 1).enter(|| {
    ///     span!("parent 2", foo = 2, bar = 1).enter(|| {
    ///         span!("my span", bar = 2).enter(|| {
    ///             // do stuff...
    ///         })
    ///     })
    /// });
    /// # }
    /// ```
    /// If a `Subscriber` were to call `all_fields` on "my span" event, it will
    /// receive an iterator with the values `("foo", 2)` and `("bar", 2)`.
    pub fn all_fields<'a>(
        &'a self,
    ) -> impl Iterator<Item = (&'a str, &'a dyn Value)> {
        self.fields()
            .chain(self.parents().flat_map(|parent| parent.fields()))
            .dedup_by(|(k, _)| k)
    }

    /// Returns the current [`State`] of this span.
    pub fn state(&self) -> State {
        match self.inner.state.load(Ordering::Acquire) {
            s if s == State::Unentered as usize => State::Unentered,
            s if s == State::Running as usize => State::Running,
            s if s == State::Idle as usize => State::Idle,
            s if s == State::Done as usize => State::Done,
            invalid => panic!("invalid state: {:?}", invalid),
        }
    }

    fn set_state(&self, prev: State, next: State) {
        self.inner.state.compare_and_swap(
            prev as usize,
            next as usize,
            Ordering::Release,
        );
    }
}

impl<'a> IntoIterator for &'a Data {
    type Item = (&'a str, &'a dyn Value);
    type IntoIter = Box<Iterator<Item = (&'a str, &'a dyn Value)> + 'a>; // TODO: unbox
    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.fields())
    }
}

impl fmt::Debug for Data {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Span")
            .field("name", &self.name())
            .field("opened_at", &self.inner.opened_at)
            .field("parent", &self.parent().unwrap_or(self).name())
            .field("fields", &self.debug_fields())
            .field("meta", &self.meta())
            .finish()
    }
}

// ===== impl Active =====

impl Active {
    #[doc(hidden)]
    pub fn current() -> Self {
        CURRENT_SPAN.with(|span| span.borrow().clone())
    }

    fn new(data: Data, enter_parent: Option<Self>) -> Self  {
        let inner = Arc::new(ActiveInner {
            data,
            enter_parent,
            currently_entered: AtomicUsize::new(0)
        });
        Self {
            inner,
        }
    }

    fn enter<F: FnOnce() -> T, T>(self, f: F) -> T {
        let prior_state = self.inner.data.state();
        match prior_state {
            // The span has been marked as done; it may not be reentered again.
            // TODO: maybe this should not crash the thread?
            State::Done => panic!("cannot re-enter completed span!"),
            _ => {
                let result = CURRENT_SPAN.with(|current_span| {
                    self.inner.transition_on_enter(prior_state);
                    current_span.replace(self.clone());
                    Dispatcher::current().enter(self.data(), Instant::now());
                    f()
                });

                CURRENT_SPAN.with(|current_span| {
                    let timestamp = Instant::now();
                    if let Some(ref parent) = self.inner.enter_parent {
                        current_span.replace(parent.clone());
                    }
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
                    Dispatcher::current().exit(self.data(), timestamp);
                });
                result
            }
        }
    }

    fn data(&self) -> &Data {
        &self.inner.data
    }

    /// Returns true if this is the last remaining handle with the capacity to
    /// enter the span.
    ///
    /// Used to determine when the span can be marked as completed.
    fn is_last_standing(&self) -> bool {
        Arc::strong_count(&self.inner) == 1
    }
}


// ===== impl ActiveInnInner =====

impl ActiveInner {

    /// Performs the state transition when entering the span.
    fn transition_on_enter(&self, from_state: State) {
        self.currently_entered
            .fetch_add(1, Ordering::Release);
        self.data.set_state(from_state, State::Running);
    }

    /// Performs the state transition when exiting the span.
    fn transition_on_exit(&self, next_state: State) {
        // Decrement the exit count
        let remaining_exits = self.currently_entered
            .fetch_sub(1, Ordering::AcqRel);
        // Only advance the state if we are the last remaining
        // thread to exit the span.
        if remaining_exits == 1 {
            self.data.set_state(State::Running, next_state);
        }
    }
}

impl cmp::PartialEq for ActiveInner {
    fn eq(&self, other: &ActiveInner) -> bool {
        self.data == other.data
    }
}

impl Hash for ActiveInner {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

// ===== impl DataInner =====

impl Hash for DataInner {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.opened_at.hash(state);
        self.static_meta.hash(state);
    }
}

impl cmp::PartialEq for DataInner {
    fn eq(&self, other: &DataInner) -> bool {
        self.opened_at == other.opened_at &&
        self.static_meta == other.static_meta
    }
}

#[cfg(test)]
pub use self::test_support::*;
#[cfg(test)]
mod test_support {
    use ::{ Value, span::State };
    use std::collections::HashMap;

    pub struct MockSpan {
        pub name: Option<Option<&'static str>>,
        pub state: Option<State>,
        pub fields: HashMap<String, Box<dyn Value>>,
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
    use ::{subscriber, span};
    use super::*;

    #[test]
    fn exit_doesnt_finish_while_handles_still_exist() {
        // Test that exiting a span only marks it as "done" when no handles
        // that can re-enter the span exist.
        let _running = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("bar")))
            // The first time we exit "bar", there will be another handle with
            // which we could potentially re-enter bar.
            .exit(span::mock().named(Some("bar"))
                .with_state(State::Idle))
            // Re-enter "bar", using the cloned handle.
            .enter(span::mock().named(Some("bar")))
            // Now, when we exit "bar", there is no handle to re-enter it, so
            // it should become "done".
            .exit(span::mock().named(Some("bar"))
                .with_state(State::Done)
            )
            // "foo" never had more than one handle, so it should also become
            // "done" when we exit it.
            .exit(span::mock().named(Some("foo"))
                .with_state(State::Done)
            )
            .run();

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
    }

    #[test]
    fn exit_doesnt_finish_concurrently_executing_spans() {
        // Test that exiting a span only marks it as "done" when no other
        // threads are still executing inside that span.
        use std::{thread, sync::{Arc, Barrier}};

        let barrier1 = Arc::new(Barrier::new(2));
        let barrier2 = Arc::new(Barrier::new(2));
        // Make copies of the barriers for thread 2 to wait on.
        let t2_barrier1 = barrier1.clone();
        let t2_barrier2 = barrier2.clone();

        subscriber::mock()
            .enter(span::mock().named(Some("baz")))
            .enter(span::mock().named(Some("quux")))
            // When the main thread exits "quux", it will still be running in the
            // spawned thread.
            .exit(span::mock().named(Some("quux"))
                .with_state(State::Running))
            // "baz" never had more than one handle, so it should also become
            // "done" when we exit it.
            .exit(span::mock().named(Some("baz"))
                .with_state(State::Done)
            )
            .run();

        span!("baz",).enter(|| {
            let quux = span!("quux",);
            let quux2 = quux.clone();
            let handle = thread::Builder::new()
                .name("thread-2".to_string())
                .spawn(move || {
                    subscriber::mock()
                        // Spawned thread also enters "quux".
                        .enter(span::mock().named(Some("quux")))
                        // Now, when this thread exits "quux", there is no handle to re-enter it, so
                        // it should become "done".
                        .exit(span::mock().named(Some("quux"))
                            .with_state(State::Done)
                        )
                        .run();
                    quux2.enter(|| {
                        // Once this thread has entered "quux", allow thread 1
                        // to exit.
                        t2_barrier1.wait();
                        // Wait for the main thread to allow us to exit.
                        t2_barrier2.wait();
                    })
                })
                .expect("spawn test thread");
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
    }

}
