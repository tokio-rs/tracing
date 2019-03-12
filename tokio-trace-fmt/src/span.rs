use std::{
    cell::RefCell,
    fmt, mem, str,
    sync::atomic::{self, AtomicUsize, Ordering},
};

use owning_ref::OwningHandle;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub(crate) use tokio_trace_core::span::{Attributes, Id, Record};
use tokio_trace_core::{dispatcher, Metadata};

pub struct Span<'a> {
    lock: OwningHandle<RwLockReadGuard<'a, Slab>, RwLockReadGuard<'a, Slot>>,
}

pub struct Context<'a, N> {
    store: &'a Store,
    new_visitor: &'a N,
}

/// Stores data associated with currently-active spans.
#[derive(Debug)]
pub(crate) struct Store {
    // Active span data is stored in a slab of span slots. Each slot has its own
    // read-write lock to guard against concurrent modification to its data.
    // Thus, we can modify any individual slot by acquiring a read lock on the
    // slab, and using that lock to acquire a write lock on the slot we wish to
    // modify. It is only necessary to acquire the write lock here when the
    // slab itself has to be modified (i.e., to allocate more slots).
    inner: RwLock<Slab>,

    // The head of the slab's "free list".
    next: AtomicUsize,
}

#[derive(Debug)]
pub(crate) struct Data {
    parent: Option<Id>,
    name: &'static str,
    ref_count: AtomicUsize,
    is_empty: bool,
}

#[derive(Debug)]
struct Slab {
    slab: Vec<RwLock<Slot>>,
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

    #[inline(always)]
    fn with_parent<'store, F, E>(
        self,
        my_id: &Id,
        last_id: Option<&Id>,
        f: &mut F,
        store: &'store Store,
    ) -> Result<(), E>
    where
        F: FnMut(&Id, Span) -> Result<(), E>,
    {
        if let Some(parent_id) = self.parent() {
            if Some(parent_id) != last_id {
                if let Some(parent) = store.get(parent_id) {
                    parent.with_parent(parent_id, Some(my_id), f, store)?;
                }
            }
        }
        f(my_id, self)
    }
}

// ===== impl Context =====

impl<'a, N> Context<'a, N>
where
    N: ::NewVisitor<'a>,
{
    pub(crate) fn current() -> Option<Id> {
        CONTEXT
            .try_with(|current| {
                current
                    .borrow()
                    .last()
                    .map(|id| dispatcher::get_default(|subscriber| subscriber.clone_span(id)))
            })
            .ok()?
    }

    pub(crate) fn push(id: Id) {
        let _ = CONTEXT.try_with(|current| {
            current.borrow_mut().push(id);
        });
    }

    pub(crate) fn pop() -> Option<Id> {
        CONTEXT
            .try_with(|current| current.borrow_mut().pop())
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
                        return span.with_parent(id, None, &mut f, self.store);
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

    pub(crate) fn new(store: &'a Store, new_visitor: &'a N) -> Self {
        Self { store, new_visitor }
    }

    pub(crate) fn new_visitor(&self, writer: &'a mut fmt::Write, is_empty: bool) -> N::Visitor {
        self.new_visitor.make(writer, is_empty)
    }
}

#[inline]
fn idx_to_id(idx: usize) -> Id {
    Id::from_u64(idx as u64 + 1)
}

#[inline]
fn id_to_idx(id: &Id) -> usize {
    id.into_u64() as usize - 1
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
    pub fn new_span<N>(&self, span: Data, attrs: &Attributes, new_visitor: &N) -> Id
    where
        N: for<'a> ::NewVisitor<'a>,
    {
        let mut span = Some(span);

        // The slab's free list is a modification of Treiber's lock-free stack,
        // using slab indices instead of pointers, and with a provison for
        // growing the slab when needed.
        //
        // In order to insert a new span into the slab, we "pop" the next free
        // index from the stack.
        loop {
            // Acquire a snapshot of the head of the free list.
            let head = self.next.load(Ordering::Relaxed);

            {
                // Try to insert the span without modifying the overall
                // structure of the stack.
                let this = self.inner.read();

                // Can we insert without reallocating?
                if head < this.slab.len() {
                    // If someone else is writing to the head slot, we need to
                    // acquire a new snapshot!
                    if let Some(mut slot) = this.slab[head].try_write() {
                        // Is the slot we locked actually empty? If not, fall
                        // through and try to grow the slab.
                        if let Some(next) = slot.next() {
                            // Is our snapshot still valid?
                            if self.next.compare_and_swap(head, next, Ordering::Release) == head {
                                // We can finally fill the slot!
                                slot.fill(span.take().unwrap(), attrs, new_visitor);
                                return idx_to_id(head);
                            }
                        }
                    }

                    // Our snapshot got stale, try again!
                    atomic::spin_loop_hint();
                    continue;
                }
            }

            // We need to grow the slab, and must acquire a write lock.
            if let Some(mut this) = self.inner.try_write() {
                let len = this.slab.len();

                // Insert the span into a new slot.
                let mut slot = Slot::new(span.take().unwrap(), attrs, new_visitor);
                this.slab.push(RwLock::new(slot));
                // TODO: can we grow the slab in chunks to avoid having to
                // realloc as often?

                // Update the head pointer and return.
                self.next.store(len + 1, Ordering::Release);
                return idx_to_id(len);
            }

            atomic::spin_loop_hint();
        }
    }

    /// Returns a `Span` to the span with the specified `id`, if one
    /// currently exists.
    #[inline]
    pub fn get(&self, id: &Id) -> Option<Span> {
        let lock = OwningHandle::try_new(self.inner.read(), |slab| {
            unsafe { &*slab }.read_slot(id_to_idx(id)).ok_or(())
        })
        .ok()?;
        Some(Span { lock })
    }

    /// Records that the span with the given `id` has the given `fields`.
    #[inline]
    pub fn record<N>(&self, id: &Id, fields: &Record, new_recorder: &N)
    where
        N: for<'a> ::NewVisitor<'a>,
    {
        let slab = self.inner.read();
        let slot = slab.write_slot(id_to_idx(id));
        if let Some(mut slot) = slot {
            slot.record(fields, new_recorder);
        }
    }

    /// Decrements the reference count of the the span with the given `id`, and
    /// removes the span if it is zero.
    ///
    /// The allocated span slot will be reused when a new span is created.
    pub fn drop_span(&self, id: Id) {
        let this = self.inner.read();
        let idx = id_to_idx(&id);

        if !this
            .slab
            .get(idx)
            .map(|span| span.read().drop_ref())
            .unwrap_or(false)
        {
            return;
        }

        // Synchronize only if we are actually removing the span (stolen
        // from std::Arc);
        atomic::fence(Ordering::Acquire);

        let data = this.remove(&self.next, idx);
        // Continue propagating the drop up the span's parent tree, to avoid
        // round-trips through the dispatcher.
        if let Some(parent) = data.and_then(|data| data.parent) {
            self.drop_span(parent)
        }
    }
}

impl Data {
    pub(crate) fn new<N>(metadata: &Metadata) -> Self
    where
        N: for<'a> ::NewVisitor<'a>,
    {
        Self {
            name: metadata.name(),
            parent: Context::<N>::current(),
            ref_count: AtomicUsize::new(1),
            is_empty: true,
        }
    }
}

impl Slot {
    fn new<N>(mut data: Data, attrs: &Attributes, new_visitor: &N) -> Self
    where
        N: for<'a> ::NewVisitor<'a>,
    {
        let mut fields = String::new();
        {
            let mut recorder = new_visitor.make(&mut fields, true);
            attrs.record(&mut recorder);
        }
        if fields.len() != 0 {
            data.is_empty = false;
        }
        Self {
            fields,
            span: State::Full(data),
        }
    }

    fn next(&self) -> Option<usize> {
        match self.span {
            State::Empty(next) => Some(next),
            _ => None,
        }
    }

    fn fill<N>(&mut self, mut data: Data, attrs: &Attributes, new_visitor: &N) -> usize
    where
        N: for<'a> ::NewVisitor<'a>,
    {
        let fields = &mut self.fields;
        {
            let mut recorder = new_visitor.make(fields, true);
            attrs.record(&mut recorder);
        }
        if fields.len() != 0 {
            data.is_empty = false;
        }
        match mem::replace(&mut self.span, State::Full(data)) {
            State::Empty(next) => next,
            State::Full(_) => unreachable!("tried to fill a full slot"),
        }
    }

    fn record<N>(&mut self, fields: &Record, new_visitor: &N)
    where
        N: for<'a> ::NewVisitor<'a>,
    {
        let state = &mut self.span;
        let buf = &mut self.fields;
        match state {
            State::Empty(_) => return,
            State::Full(ref mut data) => {
                {
                    let mut recorder = new_visitor.make(buf, data.is_empty);
                    fields.record(&mut recorder);
                }
                if buf.len() != 0 {
                    data.is_empty = false;
                }
            }
        }
    }

    fn drop_ref(&self) -> bool {
        match self.span {
            State::Full(ref data) => data.ref_count.fetch_sub(1, Ordering::Release) == 1,
            State::Empty(_) => false,
        }
    }
}

impl Slab {
    #[inline]
    fn write_slot(&self, idx: usize) -> Option<RwLockWriteGuard<Slot>> {
        self.slab.get(idx).map(|slot| slot.write())
    }

    #[inline]
    fn read_slot<'a>(&'a self, idx: usize) -> Option<RwLockReadGuard<'a, Slot>> {
        self.slab
            .get(idx)
            .map(|slot| slot.read())
            .and_then(|lock| match lock.span {
                State::Empty(_) => None,
                State::Full(_) => Some(lock),
            })
    }

    /// Remove a span slot from the slab.
    fn remove(&self, next: &AtomicUsize, idx: usize) -> Option<Data> {
        // Again we are essentially implementing a variant of Treiber's stack
        // algorithm to push the removed span's index into the free list.
        loop {
            // Get a snapshot of the current free-list head.
            let head = next.load(Ordering::Relaxed);

            // Empty the data stored at that slot.
            let mut slot = self.slab[idx].write();
            let data = match mem::replace(&mut slot.span, State::Empty(head)) {
                State::Full(data) => data,
                state => {
                    // The slot has already been emptied; leave
                    // everything as it was and return `None`!
                    slot.span = state;
                    return None;
                }
            };

            // Is our snapshot still valid?
            if next.compare_and_swap(head, idx, Ordering::Release) == head {
                // Empty the string but retain the allocated capacity
                // for future spans.
                slot.fields.clear();
                return Some(data);
            }

            atomic::spin_loop_hint();
        }
    }
}
