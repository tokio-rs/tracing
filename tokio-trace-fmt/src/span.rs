use std::{
    cell::RefCell,
    mem, str,
    sync::atomic::{self, AtomicUsize, Ordering},
};

use owning_ref::OwningHandle;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
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
            let refcount = data.ref_count.fetch_add(1, Ordering::SeqCst);
            // println!("clone_ref {:?} ({:?})", self.name(), refcount);
        }
    }

    pub(crate) fn drop_ref(&self) -> bool {
        if let State::Full(ref data) = self.lock.span {
            let refcount = data.ref_count.fetch_sub(1, Ordering::SeqCst);
            // println!("drop_ref {:?} ({:?})", self.name(), refcount);
            if refcount == 1 {
                // Synchronize only if we are actually removing the span (stolen
                // from std::Arc);
                // atomic::fence(Ordering::SeqCst);
                return true;
            }
        }
        false
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
            current.borrow_mut().push(id);
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
        let mut span = Some(span);
        loop {
            let head = self.next.load(Ordering::Acquire);
            // println!("try push {:?}; len={:?}", head, len);
            {
                let this = self.inner.read();
                if head < this.slab.len() {
                    match this.slab[head].try_write() {
                        None => {
                            // println!("-> no lock");
                            continue;
                        }
                        Some(mut slot) => {
                            if let Some(next) = slot.next() {
                                if self.next.compare_and_swap(head, next, Ordering::Release) == head
                                {
                                    slot.record(fields, new_recorder);
                                    slot.fill(span.take().unwrap());
                                    // println!("-> filled {:?}", head);
                                    return Id::from_u64(head as u64);
                                } else {
                                    continue;
                                }
                            }
                        }
                    };
                }
            }

            // println!("try grow slab");
            match self.inner.try_write() {
                None => {
                    // println!(" -> no slab lock"),
                }
                Some(mut this) => {
                    let mut slot = Slot::new(span.take().unwrap());
                    slot.record(fields, new_recorder);
                    let len = this.slab.len();
                    this.slab.push(RwLock::new(slot));
                    self.next.store(len + 1, Ordering::Release);
                    // println!("--> pushed {:?}", len + 1);
                    return Id::from_u64(len as u64);
                }
            }
        }
    }

    /// Returns a `Span` to the span with the specified `id`, if one
    /// currently exists.
    #[inline]
    pub fn get(&self, id: &Id) -> Option<Span> {
        let lock = OwningHandle::try_new(self.inner.read(), |slab| {
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
        let slab = self.inner.read();
        let slot = slab.write_slot(id);
        if let Some(mut slot) = slot {
            slot.record(fields, new_recorder);
        }
    }

    /// Removes the span with the given `id`, if one exists.
    ///
    /// The allocated span slot will be reused when a new span is created.
    #[inline]
    pub fn remove(&self, id: &Id) -> Option<Data> {
        // println!("try_remove {:?}", id);
        let next = id.into_u64() as usize;
        atomic::fence(Ordering::SeqCst);
        loop {
            match self.inner.try_read() {
                Some(this) => {
                    let head = self.next.load(Ordering::Relaxed);

                    let mut slot = this.slab[next].write();
                    let data = match mem::replace(&mut slot.span, State::Empty(next)) {
                        State::Full(data) => data,
                        state => {
                            slot.span = state;
                            return None;
                        }
                    };
                    if self.next.compare_and_swap(head, next, Ordering::Release) == head {
                        slot.fields.clear();
                        // println!("removed {:?};", id);
                        return Some(data);
                    }
                }
                None => continue,
            }
        }
    }

    pub fn drop_span(&self, id: Id) {
        if !self.get(&id).map(|span| span.drop_ref()).unwrap_or(false) {
            return;
        }

        let data = self.remove(&id);
        if let Some(parent) = data.and_then(|data| data.parent) {
            self.drop_span(parent)
        }
        atomic::fence(Ordering::SeqCst);
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

impl Slot {
    fn new(data: Data) -> Self {
        Self {
            fields: String::new(),
            span: State::Full(data),
        }
    }

    fn next(&self) -> Option<usize> {
        match self.span {
            State::Empty(next) => Some(next),
            _ => None,
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
            .map(|slot| slot.write())
    }

    #[inline]
    fn read_slot<'a>(&'a self, id: &Id) -> Option<RwLockReadGuard<'a, Slot>> {
        self.slab
            .get(id.into_u64() as usize)
            .map(|slot| slot.read())
            .and_then(|lock| match lock.span {
                State::Empty(_) => None,
                State::Full(_) => Some(lock),
            })
    }
}
