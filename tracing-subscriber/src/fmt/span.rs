use std::{
    cell::RefCell,
    fmt, str,
    sync::atomic::{self, AtomicUsize, Ordering},
};

use crate::sync::{RwLock, RwLockReadGuard};
use sharded_slab::Slab;

use std::collections::HashSet;
pub(crate) use tracing_core::span::{Attributes, Current, Id, Record};
use tracing_core::{dispatcher, Metadata};

pub struct Span<'a> {
    data: &'a Data,
    lock: RwLockReadGuard<'a, String>,
}

/// Represents the `Subscriber`'s view of the current span context to a
/// formatter.
#[derive(Debug)]
pub struct Context<'a, N> {
    store: &'a Store,
    new_visitor: &'a N,
}

/// Stores data associated with currently-active spans.
#[derive(Debug)]
pub(crate) struct Store {
    inner: Slab<Data>,
}

#[derive(Debug)]
pub(crate) struct Data {
    parent: Option<Id>,
    metadata: &'static Metadata<'static>,
    ref_count: AtomicUsize,
    fields: RwLock<String>,
}

struct ContextId {
    id: Id,
    duplicate: bool,
}

struct SpanStack {
    stack: Vec<ContextId>,
    ids: HashSet<Id>,
}

impl SpanStack {
    fn new() -> Self {
        SpanStack {
            stack: vec![],
            ids: HashSet::new(),
        }
    }

    fn push(&mut self, id: Id) {
        let duplicate = self.ids.contains(&id);
        if !duplicate {
            self.ids.insert(id.clone());
        }
        self.stack.push(ContextId { id, duplicate })
    }

    fn pop(&mut self, expected_id: &Id) -> Option<Id> {
        if &self.stack.last()?.id == expected_id {
            let ContextId { id, duplicate } = self.stack.pop()?;
            if !duplicate {
                self.ids.remove(&id);
            }
            Some(id)
        } else {
            None
        }
    }

    #[inline]
    fn current(&self) -> Option<&Id> {
        self.stack
            .iter()
            .rev()
            .find(|context_id| !context_id.duplicate)
            .map(|context_id| &context_id.id)
    }
}

thread_local! {
    static CONTEXT: RefCell<SpanStack> = RefCell::new(SpanStack::new());
}

macro_rules! debug_panic {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)] {
            if !std::thread::panicking() {
                panic!($($args)*)
            }
        }
    }
}

// ===== impl Span =====

impl<'a> Span<'a> {
    pub fn name(&self) -> &'static str {
        self.data.metadata.name()
    }

    pub fn metadata(&self) -> &'static Metadata<'static> {
        self.data.metadata
    }

    pub fn fields(&self) -> &str {
        self.lock.as_ref()
    }

    pub fn parent(&self) -> Option<&Id> {
        self.data.parent.as_ref()
    }

    #[inline(always)]
    fn with_parent<'store, F, E>(
        self,
        my_id: &Id,
        last_id: Option<&Id>,
        f: &mut F,
        store: &'store Store,
    ) -> Result<(), E>
    where
        F: FnMut(&Id, Span<'_>) -> Result<(), E>,
    {
        if let Some(parent_id) = self.parent() {
            if Some(parent_id) != last_id {
                if let Some(parent) = store.get(parent_id) {
                    parent.with_parent(parent_id, Some(my_id), f, store)?;
                } else {
                    debug_panic!("missing span for {:?}; this is a bug", parent_id);
                }
            }
        }
        f(my_id, self)
    }
}

impl<'a> fmt::Debug for Span<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Span")
            .field("name", &self.name())
            .field("parent", &self.parent())
            .field("metadata", self.metadata())
            .field("fields", &self.fields())
            .finish()
    }
}

// ===== impl Context =====

impl<'a, N> Context<'a, N> {
    /// Applies a function to each span in the current trace context.
    ///
    /// The function is applied in order, beginning with the root of the trace,
    /// and ending with the current span. If the function returns an error,
    /// this will short-circuit.
    ///
    /// If invoked from outside of a span, the function will not be applied.
    ///
    /// Note that if we are currently unwinding, this will do nothing, rather
    /// than potentially causing a double panic.
    pub fn visit_spans<F, E>(&self, mut f: F) -> Result<(), E>
    where
        F: FnMut(&Id, Span<'_>) -> Result<(), E>,
    {
        CONTEXT
            .try_with(|current| {
                if let Some(id) = current.borrow().current() {
                    if let Some(span) = self.store.get(id) {
                        // with_parent uses the call stack to visit the span
                        // stack in reverse order, without having to allocate
                        // a buffer.
                        return span.with_parent(id, None, &mut f, self.store);
                    } else {
                        debug_panic!("missing span for {:?}; this is a bug", id);
                    }
                }
                Ok(())
            })
            .unwrap_or(Ok(()))
    }

    /// Executes a closure with the reference to the current span.
    pub fn with_current<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce((&Id, Span<'_>)) -> R,
    {
        // If the lock is poisoned or the thread local has already been
        // destroyed, we might be in the middle of unwinding, so this
        // will just do nothing rather than cause a double panic.
        CONTEXT
            .try_with(|current| {
                if let Some(id) = current.borrow().current() {
                    if let Some(span) = self.store.get(&id) {
                        return Some(f((&id, span)));
                    } else {
                        debug_panic!("missing span for {:?}, this is a bug", id);
                    }
                }
                None
            })
            .ok()?
    }

    pub(crate) fn new(store: &'a Store, new_visitor: &'a N) -> Self {
        Self { store, new_visitor }
    }

    /// Returns a new visitor that formats span fields to the provided writer.
    /// The visitor configuration is provided by the subscriber.
    pub fn new_visitor<'writer>(
        &self,
        writer: &'writer mut dyn fmt::Write,
        is_empty: bool,
    ) -> N::Visitor
    where
        N: super::NewVisitor<'writer>,
    {
        self.new_visitor.make(writer, is_empty)
    }
}

#[inline]
fn idx_to_id(idx: usize) -> Id {
    Id::from_u64(idx as u64 + 1)
}

#[inline]
fn id_to_idx(id: &Id) -> usize {
    (id.into_u64() - 1) as usize
}

impl Store {
    pub(crate) fn new() -> Self {
        Store {
            inner: Slab::new(),
        }
    }

    #[inline]
    pub(crate) fn current(&self) -> Option<Id> {
        CONTEXT
            .try_with(|current| current.borrow().current().cloned())
            .ok()?
    }

    pub(crate) fn push(&self, id: &Id) {
        let _ = CONTEXT.try_with(|current| current.borrow_mut().push(self.clone_span(id)));
    }

    pub(crate) fn pop(&self, expected_id: &Id) {
        let id = CONTEXT
            .try_with(|current| current.borrow_mut().pop(expected_id))
            .ok()
            .and_then(|i| i);
        if let Some(id) = id {
            let _ = self.drop_span(id);
        }
    }

    /// Inserts a new span with the given data and fields into the slab,
    /// returning an ID for that span.
    ///
    /// If there are empty slots in the slab previously allocated for spans
    /// which have since been closed, the allocation and span ID of the most
    /// recently emptied span will be reused. Otherwise, a new allocation will
    /// be added to the slab.
    #[inline]
    pub(crate) fn new_span<N>(&self, attrs: &Attributes<'_>, new_visitor: &N) -> Id
    where
        N: for<'a> super::NewVisitor<'a>,
    {
        let span = Data::new(attrs, self, new_visitor);
        let id = self.inner.insert(span).expect("out of span storage!");
        idx_to_id(id)
    }

    /// Returns a `Span` to the span with the specified `id`, if one
    /// currently exists.
    #[inline]
    pub(crate) fn get(&self, id: &Id) -> Option<Span<'_>> {
        let data = self.inner.get(id_to_idx(id))?;
        let lock = data.fields.read().ok()?;
        Some(Span {
            data,
            lock,
        })
    }

    /// Records that the span with the given `id` has the given `fields`.
    #[inline]
    pub(crate) fn record<N>(&self, id: &Id, fields: &Record<'_>, new_recorder: &N)
    where
        N: for<'a> super::NewVisitor<'a>,
    {

        let data = if let Some(data) = self.inner.get(id_to_idx(id)) {
            data
        } else {
            return;
        };
        let mut lock = try_lock!(data.fields.write(), else return);
        let empty = lock.is_empty();
        let mut recorder = new_recorder.make(&mut *lock, empty);
        fields.record(&mut recorder);
    }

    /// Decrements the reference count of the span with the given `id`, and
    /// removes the span if it is zero.
    ///
    /// The allocated span slot will be reused when a new span is created.
    pub(crate) fn drop_span(&self, id: Id) -> bool {
        let idx = id_to_idx(&id);

        if let Some(span) = self.inner.get(idx) {
            if span.drop_ref() {
                // Synchronize only if we are actually removing the span (stolen
                // from std::Arc);
                atomic::fence(Ordering::Acquire);

                self.inner.remove(idx);
                return true;
            }
        }

        false
    }

    pub(crate) fn clone_span(&self, id: &Id) -> Id {
        let idx = id_to_idx(id);

        if let Some(span) = self.inner.get(idx) {
            span.clone_ref();
        } else {
            debug_panic!(
                "tried to clone {:?}, but no span exists with that ID. this is a bug!",
                id
            );
        }
        id.clone()
    }
}

impl Data {
    pub(crate) fn new<N>(attrs: &Attributes<'_>, store: &Store, new_visitor: &N) -> Self
    where
        N: for<'a> super::NewVisitor<'a>,
    {
        let parent = if attrs.is_root() {
            None
        } else if attrs.is_contextual() {
            store.current().as_ref().map(|id| store.clone_span(id))
        } else {
            attrs.parent().map(|id| store.clone_span(id))
        };
        let mut fields = String::new();
        {
            let mut recorder = new_visitor.make(&mut fields, true);
            attrs.record(&mut recorder);
        };
        Self {
            metadata: attrs.metadata(),
            parent,
            ref_count: AtomicUsize::new(1),
            fields: RwLock::new(fields),
        }
    }

    #[inline]
    fn drop_ref(&self) -> bool {
        let refs = self.ref_count.fetch_sub(1, Ordering::Release);
        debug_assert!(
            if std::thread::panicking() {
                // don't cause a double panic, even if the ref-count is wrong...
                true
            } else {
                refs != std::usize::MAX
            },
            "reference count overflow!"
        );
        refs == 1
    }

    #[inline]
    fn clone_ref(&self) {
        let _refs = self.ref_count.fetch_add(1, Ordering::Release);
        debug_assert!(_refs != 0, "tried to clone a span that already closed!");
    }
}

impl Drop for Data {
    fn drop(&mut self) {
        // We have to actually unpack the option inside the `get_default`
        // closure, since it is a `FnMut`, but testing that there _is_ a value
        // here lets us avoid the thread-local access if we don't need the
        // dispatcher at all.
        if self.parent.is_some() {
            dispatcher::get_default(|subscriber| {
                if let Some(parent) = self.parent.take() {
                    let _ = subscriber.try_close(parent);
                }
            })
        }
    }
}