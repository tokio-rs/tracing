use std::{
    cell::RefCell,
    fmt, io, mem, str,
    sync::{
        atomic::{self, AtomicUsize, Ordering},
        RwLock,
    },
};

pub use tokio_trace_core::Span as Id;
use tokio_trace_core::{dispatcher, field, Metadata};

#[derive(Debug)]
pub struct Span<'a> {
    data: &'a Data,
    fields: &'a str,
}

pub struct Context<'a> {
    lock: &'a RwLock<Slab>,
}

#[derive(Debug)]
pub(crate) struct Slab {
    slab: Vec<Slot>,
    next: usize,
    count: usize,
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
        self.data.name
    }

    pub fn fields(&self) -> &str {
        self.fields
    }

    pub fn parent(&self) -> Option<&Id> {
        self.data.parent.as_ref()
    }

    #[inline]
    pub(crate) fn clone_ref(&self) {
        self.data.ref_count.fetch_add(1, Ordering::Release);
    }

    #[inline]
    pub(crate) fn drop_ref(&self) -> bool {
        if self.data.ref_count.fetch_sub(1, Ordering::Release) != 1 {
            return false;
        }

        // Synchronize only if we are actually removing the span (stolen
        // from std::Arc);
        atomic::fence(Ordering::Acquire);
        true
    }

    #[inline(always)]
    fn with_parent<'store, F, E>(self, my_id: &Id, f: &mut F, store: &'store Slab) -> Result<(), E>
    where
        F: FnMut(&Id, Span) -> Result<(), E>,
    {
        if let Some(parent_id) = self.data.parent.as_ref() {
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
            current.borrow_mut().push(id.clone());
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
                    if let Ok(store) = self.lock.read() {
                        if let Some(span) = store.get(id) {
                            // with_parent uses the call stack to visit the span
                            // stack in reverse order, without having to allocate
                            // a buffer.
                            return span.with_parent(id, &mut f, &store);
                        }
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
                    let spans = self.lock.read().ok()?;
                    if let Some(span) = spans.get(id) {
                        return Some(f((id, span)));
                    }
                }
                None
            })
            .ok()?
    }

    pub(crate) fn new(lock: &'a RwLock<Slab>) -> Self {
        Self { lock }
    }
}

impl Slab {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slab: Vec::with_capacity(capacity),
            count: 0,
            next: 0,
        }
    }

    #[inline(always)]
    fn id_to_idx(id: &Id) -> usize {
        // TODO: :(
        (unsafe { mem::transmute::<_, u64>(id.clone()) }) as usize
    }

    /// Inserts a new span with the given data and fields into the slab,
    /// returning an ID for that span.
    ///
    /// If there are empty slots in the slab previously allocated for spans
    /// which have since been closed, the allocation and span ID of the most
    /// recently emptied span will be reused. Otherwise, a new allocation will
    /// be added to the slab.
    #[inline]
    pub fn new_span<N>(&mut self, span: Data, fields: &field::ValueSet, new_recorder: &N) -> Id
    where
        N: for<'a> ::NewRecorder<'a>,
    {
        self.count += 1;
        let idx = self.next;

        if self.next == self.slab.len() {
            // The next index is the end of the slab, so we need to add a
            // new slot. This allocates an additional string and grows
            // the length of the slab by 1.
            self.slab.push(Slot::new(span));
            self.next += 1;
        } else {
            // Place the new span in an existing slot in the slab.
            self.next = self.slab[self.next].fill(span);
        }

        self.slab[idx].record(fields, new_recorder);

        Id::from_u64(idx as u64)
    }

    /// Returns a `Span` to the span with the specified `id`, if one
    /// currently exists.
    #[inline]
    pub fn get(&self, id: &Id) -> Option<Span> {
        self.slab
            .get(Self::id_to_idx(id))
            .and_then(Slot::as_span_ref)
    }

    /// Records that the span with the given `id` has the given `fields`.
    #[inline]
    pub fn record<N>(&mut self, id: &Id, fields: &field::ValueSet, new_recorder: &N)
    where
        N: for<'a> ::NewRecorder<'a>,
    {
        if let Some(slot) = self.slab.get_mut(Self::id_to_idx(id)) {
            slot.record(fields, new_recorder);
        }
    }

    /// Removes the span with the given `id`, if one exists.
    ///
    /// The allocated span slot will be reused when a new span is created.
    #[inline]
    pub fn remove(&mut self, id: &Id) {
        let idx = Self::id_to_idx(id);
        if self.slab[idx].empty(self.next) {
            self.next = idx;
            self.count -= 1;
        }
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

    fn as_span_ref(&self) -> Option<Span> {
        match self.span {
            State::Full(ref data) => Some(Span {
                data,
                fields: self.fields.as_ref(),
            }),
            _ => None,
        }
    }

    fn empty(&mut self, next: usize) -> bool {
        let mut was_cleared = false;
        match mem::replace(&mut self.span, State::Empty(next)) {
            State::Full(_) => {
                // Reuse the already allocated string for the next span's
                // fields, avoiding an additional allocation.
                self.fields.clear();
                was_cleared = true;
            }
            state => self.span = state,
        };
        was_cleared
    }

    fn fill(&mut self, data: Data) -> usize {
        let span = &mut self.span;
        let buf = &mut self.fields;
        match mem::replace(span, State::Full(data)) {
            State::Empty(next) => next,
            State::Full(_) => unreachable!(),
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
