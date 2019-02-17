use std::{
    cell::RefCell,
    collections::HashMap,
    fmt, io, mem, str,
    sync::{
        atomic::{AtomicUsize, Ordering},
        RwLock,
    },
};

pub use tokio_trace_core::Span as Id;
use tokio_trace_core::dispatcher;


#[derive(Debug)]
pub struct Data {
    parent: Option<Id>,
    pub(crate) name: &'static str,
    pub(crate) fields: String,
    ref_count: AtomicUsize,
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
enum Slot {
    Full(Data),
    Empty(usize),
}

thread_local! {
    static CONTEXT: RefCell<Vec<Id>> = RefCell::new(vec![]);
}

// ===== impl Data =====

impl Data {
    pub(crate) fn new(name: &'static str, fields: String) -> Self {
        Self {
            parent: Context::current(),
            name,
            fields,
            ref_count: AtomicUsize::new(1),
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn fields(&self) -> &str {
        self.fields.as_ref()
    }

    #[inline]
    pub(crate) fn clone_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::Release);
    }

    #[inline]
    pub(crate) fn drop_ref(&self) -> bool {
        self.ref_count.fetch_sub(1, Ordering::AcqRel) == 1
    }

    #[inline(always)]
    fn with_parent<'store, F, E>(&self, my_id: &Id, f: &mut F, store: &'store Slab) -> Result<(), E>
    where
        F: FnMut(&Id, &Data) -> Result<(), E>
    {
        if let Some(parent_id) = self.parent.as_ref() {
            if let Some(parent) = store.get(parent_id) {
                parent.with_parent(parent_id, f, store)?;
            }
        }
        f(my_id, self)
    }
}

impl io::Write for Data {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Hopefully consumers of this struct will only use the `write_fmt`
        // impl, which should be much faster.
        let string = str::from_utf8(buf)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        self.fields.push_str(string);
        Ok(buf.len())
    }

    #[inline]
    fn write_fmt(&mut self, args: fmt::Arguments) -> io::Result<()> {
        use fmt::Write;
        self.fields.write_fmt(args)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}


impl<'a> Context<'a> {
    pub(crate) fn current() -> Option<Id> {
        CONTEXT.try_with(|current| {
            current.borrow()
                .last()
                .map(|id| {
                    dispatcher::with(|subscriber| {
                        subscriber.clone_span(id)
                    })
                })
        }).ok()?
    }

    pub(crate) fn push(id: Id) {
        let _ = CONTEXT.try_with(|current| {
            current.borrow_mut().push(id.clone());
        });
    }

    pub(crate) fn pop() -> Option<Id> {
        CONTEXT.try_with(|current| {
            current.borrow_mut().pop()
        }).ok()?
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
        F: FnMut(&Id, &Data) -> Result<(), E>
    {
        CONTEXT.try_with(|current| {
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
        }).unwrap_or(Ok(()))
    }

    pub fn with_current<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce((&Id, &Data)) -> R,
    {
        // If the lock is poisoned or the thread local has already been
        // destroyed, we might be in the middle of unwinding, so this
        // will just do nothing rather than cause a double panic.
        CONTEXT.try_with(|current| {
            if let Some(id) = current.borrow().last() {
                let spans = self.lock.read().ok()?;
                if let Some(span) = spans.get(id) {
                    return Some(f((id, span)));
                }
            }
            None
        }).ok()?
    }

    pub(crate) fn new(lock: &'a RwLock<Slab>) -> Self {
        Self {
            lock,
        }
    }
}

impl Slab {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

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

    #[inline]
    pub fn insert(&mut self, span: Data) -> Id {
        self.count += 1;
        let id = Id::from_u64(self.next as u64);

        if self.next == self.slab.len() {
            self.slab.push(Slot::Full(span));
            self.next += 1;
            return id;
        }

        match mem::replace(&mut self.slab[self.next], Slot::Full(span)) {
            Slot::Empty(next) => self.next = next,
            Slot::Full(_) => unreachable!("slab.next pointed at full slot!"),
        };

        id
    }

    #[inline(always)]
    pub fn get(&self, id: &Id) -> Option<&Data> {
        // eprintln!("Slab::get: id={:?}; len={:?}; available={:?};", id, self.slab.len(), self.available);
       match self.slab.get(Self::id_to_idx(id))? {
           Slot::Full(span) => Some(span),
           _ => None,
       }
    }

    #[inline]
    pub fn get_mut(&mut self, id: &Id) -> Option<&mut Data> {
        // eprintln!("Slab::get_mut: id={:?}; len={:?}; available={:?};", id, self.slab.len(), self.available);
        match self.slab.get_mut(Self::id_to_idx(id))? {
            Slot::Full(span) => Some(span),
            _ => None,
        }
    }

    #[inline]
    pub fn remove(&mut self, id: &Id) {
        // eprintln!("Slab::remove: id={:?}; len={:?}; available={:?};", id, self.slab.len(), self.available);
        let idx = Self::id_to_idx(id);
        match mem::replace(&mut self.slab[idx], Slot::Empty(self.next)) {
            Slot::Full(_) => {
                self.next = idx;
                self.count -= 1;
            },
            entry =>
                // slot was already emptied, do nothing.
                self.slab[idx] = entry,
        }
    }
}
