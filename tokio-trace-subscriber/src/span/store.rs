use std::{
    cell::RefCell,
    fmt, mem, str,
    sync::atomic::{self, AtomicUsize, Ordering},
};

use owning_ref::OwningHandle;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use ::span::{Registry, Ref, Id};

pub struct Span<'a> {
    lock: OwningHandle<RwLockReadGuard<'a, Slab>, RwLockReadGuard<'a, Slot>>,
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
