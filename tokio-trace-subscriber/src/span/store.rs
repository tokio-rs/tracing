use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    fmt, mem, str,
    sync::atomic::{self, AtomicUsize, Ordering},
};

use owning_ref::OwningHandle;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use span::{Attributes, Data, Id, Record, Ref, RefMut, Registry};

/// Stores data associated with currently-active spans.
#[derive(Debug)]
pub struct Store {
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

type RefInner<'a, T> = OwningHandle<RwLockReadGuard<'a, Slab>, T>;

pub struct StoreRef<'a> {
    lock: RefInner<'a, RwLockReadGuard<'a, Slot>>,
}

pub struct StoreRefMut<'a> {
    lock: RefInner<'a, RwLockWriteGuard<'a, Slot>>,
}

#[derive(Debug)]
struct Active {
    parent: Option<Id>,
    name: &'static str,
    ref_count: AtomicUsize,
}

#[derive(Debug)]
struct Slab {
    slab: Vec<RwLock<Slot>>,
}

#[derive(Debug)]
struct Slot {
    data: Data,
    span: State,
}

#[derive(Debug)]
enum State {
    Active(Active),
    Empty(usize),
}

thread_local! {
    static CURRENT: RefCell<Vec<Id>> = RefCell::new(vec![]);
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

    fn make_active(&self, attrs: &Attributes) -> Active {
        let parent = if attrs.is_contextual() {
            with_current(|id| self.clone_span(id))
        } else {
            attrs.parent().map(|id| self.clone_span(id))
        };
        Active {
            name: attrs.metadata().name(),
            parent,
            ref_count: AtomicUsize::from(1),
        }
    }
}

fn with_current<F, T>(f: F) -> Option<T>
where
    F: FnOnce(&Id) -> T,
{
    CURRENT
        .try_with(|stack| stack.borrow().last().map(f))
        .ok()?
}

impl<'a> Registry<'a> for Store {
    type Span = StoreRef<'a>;
    type SpanMut = StoreRefMut<'a>;

    fn current(&self) -> Option<Self::Span> {
        with_current(|id| self.get(id))?
    }

    fn current_mut(&self) -> Option<Self::SpanMut> {
        with_current(|id| self.get_mut(id))?
    }

    /// Inserts a new span with the given data and fields into the slab,
    /// returning an ID for that span.
    ///
    /// If there are empty slots in the slab previously allocated for spans
    /// which have since been closed, the allocation and span ID of the most
    /// recently emptied span will be reused. Otherwise, a new allocation will
    /// be added to the slab.
    fn new_span<F>(&self, attrs: &Attributes, f: F) -> Id
    where
        F: FnOnce(&Attributes, &mut Data),
    {
        let mut to_insert = Some((self.make_active(attrs), f));

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
                                let (span, f) = match to_insert.take() {
                                    Some(x) => x,
                                    None => unreachable!("span will not be inserted twice"),
                                };
                                slot.fill(span, attrs, f);
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

                let (span, f) = match to_insert.take() {
                    Some(x) => x,
                    None => unreachable!("span will not be inserted twice"),
                };
                // Insert the span into a new slot.
                let mut slot = Slot::new(span, attrs, f);
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

    // /// Returns a `Span` to the span with the specified `id`, if one
    // /// currently exists.
    // #[inline]
    // pub fn get(&self, id: &Id) -> Option<Span> {
    //     let lock = OwningHandle::try_new(self.inner.read(), |slab| {
    //         unsafe { &*slab }.read_slot(id_to_idx(id)).ok_or(())
    //     })
    //     .ok()?;
    //     Some(Span { lock })
    // }

    // /// Records that the span with the given `id` has the given `fields`.
    // #[inline]
    // pub fn record<N>(&self, id: &Id, fields: &Record, new_recorder: &N)
    // where
    //     N: for<'a> ::NewVisitor<'a>,
    // {
    //     let slab = self.inner.read();
    //     let slot = slab.write_slot(id_to_idx(id));
    //     if let Some(mut slot) = slot {
    //         slot.record(fields, new_recorder);
    //     }
    // }

    fn clone_span(&self, id: &Id) {
        unimplemented!()
    }

    fn get(&self, id: &Id) -> Option<Self::Span> {
        unimplemented!()
    }

    fn get_mut(&self, id: &Id) -> Option<Self::SpanMut> {
        unimplemented!()
    }

    /// Decrements the reference count of the the span with the given `id`, and
    /// removes the span if it is zero.
    ///
    /// The allocated span slot will be reused when a new span is created.
    fn drop_span(&self, id: Id) {
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

        this.remove(&self.next, idx);
    }
}

impl Slot {
    fn new<F>(mut active: Active, attrs: &Attributes, f: F) -> Self
    where
        F: FnOnce(&Attributes, &mut Data),
    {
        let mut data = Data::new();
        f(attrs, &mut data);
        Self {
            data,
            span: State::Active(active),
        }
    }

    fn next(&self) -> Option<usize> {
        match self.span {
            State::Empty(next) => Some(next),
            _ => None,
        }
    }

    fn fill<F>(&mut self, mut active: Active, attrs: &Attributes, f: F) -> usize
    where
        F: FnOnce(&Attributes, &mut Data),
    {
        f(attrs, &mut self.data);
        match mem::replace(&mut self.span, State::Active(active)) {
            State::Empty(next) => next,
            State::Active(_) => unreachable!("tried to fill a full slot"),
        }
    }

    // fn record<N>(&mut self, fields: &Record, f: F)
    // where
    //     F: FnOnce(&Record, &mut Data),
    // {
    //     let state = &mut self.span;
    //     let buf = &mut self.fields;
    //     match state {
    //         State::Empty(_) => return,
    //         State::Active(ref mut data) => {
    //             {
    //                 let mut recorder = new_visitor.make(buf, data.is_empty);
    //                 fields.record(&mut recorder);
    //             }
    //             if buf.len() != 0 {
    //                 data.is_empty = false;
    //             }
    //         }
    //     }
    // }

    fn drop_ref(&self) -> bool {
        match self.span {
            State::Active(ref data) => data.ref_count.fetch_sub(1, Ordering::Release) == 1,
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
                State::Active(_) => Some(lock),
            })
    }

    /// Remove a span slot from the slab.
    fn remove(&self, next: &AtomicUsize, idx: usize) -> Option<Active> {
        // Again we are essentially implementing a variant of Treiber's stack
        // algorithm to push the removed span's index into the free list.
        loop {
            // Get a snapshot of the current free-list head.
            let head = next.load(Ordering::Relaxed);

            // Empty the data stored at that slot.
            let mut slot = self.slab[idx].write();
            let data = match mem::replace(&mut slot.span, State::Empty(head)) {
                State::Active(data) => data,
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
                slot.data.clear();
                return Some(data);
            }

            atomic::spin_loop_hint();
        }
    }
}

impl<'a> Ref<'a> for StoreRef<'a> {
    fn name(&self) -> &'static str {
        unimplemented!();
    }

    fn parent(&self) -> Option<&Id> {
        unimplemented!()
    }
}

impl<'a> Borrow<Data> for StoreRef<'a> {
    fn borrow(&self) -> &Data {
        &self.lock.data
    }
}

impl<'a> Borrow<Data> for StoreRefMut<'a> {
    fn borrow(&self) -> &Data {
        &self.lock.data
    }
}

impl<'a> BorrowMut<Data> for StoreRefMut<'a> {
    fn borrow_mut(&mut self) -> &mut Data {
        &mut self.lock.data
    }
}

impl<'a> Ref<'a> for StoreRefMut<'a> {
    fn name(&self) -> &'static str {
        unimplemented!();
    }

    fn parent(&self) -> Option<&Id> {
        unimplemented!()
    }
}

impl<'a> RefMut<'a> for StoreRefMut<'a> {}
