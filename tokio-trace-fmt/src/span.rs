use std::{
    cell::RefCell,
    mem, str,
    sync::{
        atomic::{self, AtomicUsize, Ordering},
        RwLock, RwLockReadGuard, RwLockWriteGuard,
    },
};

use owning_ref::OwningHandle;
pub use tokio_trace_core::Span as Id;
use tokio_trace_core::{dispatcher, field, Metadata};

pub struct Span<'a> {
    lock: OwningHandle<RwLockReadGuard<'a, Slab>, RwLockReadGuard<'a, Slot>>,
}

pub struct Context<'a> {
    store: &'a Store,
}

#[derive(Debug)]
pub(crate) struct Store {
    inner: RwLock<Slab>,
    next: AtomicUsize,
}

#[derive(Debug)]
struct Slab {
    slab: Vec<RwLock<Slot>>,
}

#[derive(Debug)]
pub(crate) struct Data {
    parent: Option<Id>,
    name: &'static str,
    ref_count: AtomicUsize,
    is_empty: bool,
}

#[derive(Debug)]
struct Slot {
    fields: String,
    span: State,
}

#[derive(Debug)]
enum State {
    Full(Data),
    Empty(usize),
}

thread_local! {
    static CONTEXT: RefCell<Vec<Id>> = RefCell::new(vec![]);
}

// ===== impl Span =====

impl<'a> Span<'a> {
    pub fn name(&self) -> &'static str {
        match self.lock.span {
            State::Full(ref data) => data.name.as_ref(),
            State::Empty(_) => unreachable!(),
        }
    }

    pub fn fields(&self) -> &str {
        self.lock.fields.as_ref()
    }

    pub fn parent(&self) -> Option<&Id> {
        match self.lock.span {
            State::Full(ref data) => data.parent.as_ref(),
            State::Empty(_) => unreachable!(),
        }
    }

    #[inline]
    pub(crate) fn clone_ref(&self) {
        if let State::Full(ref data) = self.lock.span {
            data.ref_count.fetch_add(1, Ordering::Release);
        }
    }

    #[inline]
    pub(crate) fn drop_ref(&self) -> bool {
        if let State::Full(ref data) = self.lock.span {
            if data.ref_count.fetch_sub(1, Ordering::Relaxed) == 1 {
                // Synchronize only if we are actually removing the span (stolen
                // from std::Arc);
                atomic::fence(Ordering::SeqCst);
                return true;
            }
        }
        false
    }

    #[inline(always)]
    fn with_parent<'store, F, E>(self, my_id: &Id, f: &mut F, store: &'store Store) -> Result<(), E>
    where
        F: FnMut(&Id, Span) -> Result<(), E>,
    {
        if let Some(parent_id) = self.parent() {
            if let Some(parent) = store.get(parent_id) {
                parent.with_parent(parent_id, f, store)?;
            }
        }
        f(my_id, self)
    }
}

// ===== impl Context =====

impl<'a> Context<'a> {
    pub(crate) fn current() -> Option<Id> {
        CONTEXT
            .try_with(|current| {
                current
                    .borrow()
                    .last()
                    .map(|id| dispatcher::with(|subscriber| subscriber.clone_span(id)))
            })
            .ok()?
    }

    pub(crate) fn push(id: Id) {
        let _ = CONTEXT.try_with(|current| {
            atomic::fence(Ordering::SeqCst);
            current.borrow_mut().push(id.clone());
        });
    }

    pub(crate) fn pop() -> Option<Id> {
        CONTEXT
            .try_with(|current| {
                atomic::fence(Ordering::SeqCst);
                current.borrow_mut().pop()
            })
            .ok()?
    }

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
        F: FnMut(&Id, Span) -> Result<(), E>,
    {
        CONTEXT
            .try_with(|current| {
                if let Some(id) = current.borrow().last() {
                    if let Some(span) = self.store.get(id) {
                        // with_parent uses the call stack to visit the span
                        // stack in reverse order, without having to allocate
                        // a buffer.
                        return span.with_parent(id, &mut f, self.store);
                    }
                }
                Ok(())
            })
            .unwrap_or(Ok(()))
    }

    pub fn with_current<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce((&Id, Span)) -> R,
    {
        // If the lock is poisoned or the thread local has already been
        // destroyed, we might be in the middle of unwinding, so this
        // will just do nothing rather than cause a double panic.
        CONTEXT
            .try_with(|current| {
                if let Some(id) = current.borrow().last() {
                    if let Some(span) = self.store.get(id) {
                        return Some(f((id, span)));
                    }
                }
                None
            })
            .ok()?
    }

    pub(crate) fn new(store: &'a Store) -> Self {
        Self { store }
    }
}

impl Store {
    pub fn with_capacity(capacity: usize) -> Self {
        Store {
            inner: RwLock::new(Slab {
                slab: Vec::with_capacity(capacity),
            }),
            next: AtomicUsize::new(0),
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
    pub fn new_span<N>(&self, span: Data, fields: &field::ValueSet, new_recorder: &N) -> Id
    where
        N: for<'a> ::NewRecorder<'a>,
    {
        let idx = self.next.load(Ordering::SeqCst);

        if idx == self.inner.read().unwrap().slab.len() {
            // The next index is the end of the slab, so we need to add a
            // new slot. This allocates an additional string and grows
            // the length of the slab by 1.
            let mut slot = Slot::new(span);
            slot.record(fields, new_recorder);
            self.inner.write().unwrap().slab.push(RwLock::new(slot));
            self.next.store(idx + 1, Ordering::Release);
        } else {
            // Place the new span in an existing slot in the slab.
            let inner = self.inner.read().unwrap();
            let mut slot = inner.slab[idx].write().unwrap();
            self.next.store(slot.fill(span), Ordering::Release);
            slot.record(fields, new_recorder);
        }

        Id::from_u64(idx as u64)
    }

    /// Returns a `Span` to the span with the specified `id`, if one
    /// currently exists.
    #[inline]
    pub fn get(&self, id: &Id) -> Option<Span> {
        let lock = OwningHandle::try_new(self.inner.read().ok()?, |slab| {
            unsafe { &*slab }.read_slot(id).ok_or(())
        })
        .ok()?;
        Some(Span { lock })
    }

    /// Records that the span with the given `id` has the given `fields`.
    #[inline]
    pub fn record<N>(&self, id: &Id, fields: &field::ValueSet, new_recorder: &N)
    where
        N: for<'a> ::NewRecorder<'a>,
    {
        if let Ok(slab) = self.inner.read() {
            if let Some(mut slot) = slab.write_slot(id) {
                slot.record(fields, new_recorder);
            }
        }
    }

    /// Removes the span with the given `id`, if one exists.
    ///
    /// The allocated span slot will be reused when a new span is created.
    #[inline]
    pub fn remove(&self, id: &Id) -> Option<Data> {
        let slab = self.inner.read().ok()?;
        let next = self.next.load(Ordering::Acquire);
        let data = slab.write_slot(id).map(|mut slot| slot.empty(next))?;
        self.next.store(id.into_u64() as usize, Ordering::Release);
        data
    }
}

impl Data {
    pub(crate) fn new(metadata: &Metadata) -> Self {
        Self {
            name: metadata.name(),
            parent: Context::current(),
            ref_count: AtomicUsize::new(1),
            is_empty: true,
        }
    }
}

impl Drop for Data {
    fn drop(&mut self) {
        dispatcher::with(|subscriber| {
            if let Some(parent) = self.parent.take() {
                subscriber.drop_span(parent);
            }
        });
    }
}

impl Slot {
    fn new(data: Data) -> Self {
        Self {
            fields: String::new(),
            span: State::Full(data),
        }
    }

    fn empty(&mut self, next: usize) -> Option<Data> {
        let mut was_cleared = false;
        match mem::replace(&mut self.span, State::Empty(next)) {
            State::Full(data) => {
                // Reuse the already allocated string for the next span's
                // fields, avoiding an additional allocation.
                self.fields.clear();
                Some(data)
            }
            state => {
                self.span = state;
                None
            }
        }
    }

    fn fill(&mut self, data: Data) -> usize {
        match mem::replace(&mut self.span, State::Full(data)) {
            State::Empty(next) => next,
            State::Full(_) => unreachable!("tried to fill a full slot"),
        }
    }

    fn record<N>(&mut self, fields: &field::ValueSet, new_recorder: &N)
    where
        N: for<'a> ::NewRecorder<'a>,
    {
        let state = &mut self.span;
        let buf = &mut self.fields;
        match state {
            State::Empty(_) => return,
            State::Full(ref mut data) => {
                {
                    let mut recorder = new_recorder.make(buf, data.is_empty);
                    fields.record(&mut recorder);
                }
                if buf.len() != 0 {
                    data.is_empty = false;
                }
            }
        }
    }
}

impl Slab {
    #[inline]
    fn write_slot(&self, id: &Id) -> Option<RwLockWriteGuard<Slot>> {
        self.slab
            .get(id.into_u64() as usize)
            .and_then(|slot| slot.write().ok())
    }

    fn read_slot<'a>(&'a self, id: &Id) -> Option<RwLockReadGuard<'a, Slot>> {
        self.slab
            .get(id.into_u64() as usize)
            .and_then(|slot| slot.read().ok())
            .and_then(|lock| match lock.span {
                State::Empty(_) => None,
                State::Full(_) => Some(lock),
            })
    }
}
