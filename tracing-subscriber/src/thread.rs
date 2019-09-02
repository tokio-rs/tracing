use std::{
    cell::{Cell, UnsafeCell},
    fmt,
    marker::PhantomData,
};
use std::{
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
};

use crossbeam_utils::sync::{ShardedLock, ShardedLockReadGuard};

pub(crate) struct Local<T> {
    inner: ShardedLock<Inner<T>>,
}

type Inner<T> = Vec<Option<UnsafeCell<T>>>;

pub(crate) struct LocalGuard<'a, T> {
    inner: *mut T,
    /// Hold the read guard for the lifetime of this guard. We're not actually
    /// accessing the data behind that guard, but holding the read lock will
    /// keep another thread from reallocating the vec.
    _lock: ShardedLockReadGuard<'a, Inner<T>>,
}

/// Uniquely identifies a thread.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct Id {
    id: usize,
    _not_send: PhantomData<UnsafeCell<()>>,
}

// === impl Local ===

impl<T> Local<T> {
    pub(crate) fn new() -> Self {
        let len = Id::current().as_usize();
        // Preallocate up to the current thread ID, so we don't have to inside
        // the lock.
        let mut data = Vec::with_capacity(len);
        data.resize_with(len, || None);
        Local {
            inner: ShardedLock::new(data),
        }
    }

    /// Returns a `LocalGuard` that dereferences to the local data for the
    /// current thread. If no local data exists, the provided function is
    /// invoked and the result is inserted.
    pub(crate) fn get_or_else<'a>(&'a self, new: impl FnOnce() -> T) -> LocalGuard<'a, T> {
        let i = Id::current().as_usize();
        if let Some(guard) = self.try_get(i) {
            guard
        } else {
            self.new_thread(i, new);
            self.try_get(i).expect("data was just inserted")
        }
    }

    // Returns a `LocalGuard` for the local data for this thread, or `None` if
    // no local data has been inserted.
    fn try_get<'a>(&'a self, i: usize) -> Option<LocalGuard<'a, T>> {
        let lock = try_lock!(self.inner.read(), else return None);
        let slot = lock.get(i)?.as_ref()?;
        let inner = slot.get();
        Some(LocalGuard { inner, _lock: lock })
    }

    #[cold]
    fn new_thread<'a>(&'a self, i: usize, new: impl FnOnce() -> T) {
        let mut lock = try_lock!(self.inner.write());
        let this = &mut *lock;
        this.resize_with(i + 1, || None);
        this[i] = Some(UnsafeCell::new(new()));
    }
}

impl<T: Default> Local<T> {
    /// Returns a `LocalGuard` that dereferences to the local data for the
    /// current thread. If no local data exists, the default value is inserted.
    pub(crate) fn get<'a>(&'a self) -> LocalGuard<'a, T> {
        self.get_or_else(T::default)
    }
}

unsafe impl<T> Sync for Local<T> {}

impl<T: fmt::Debug> fmt::Debug for Local<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = Id::current();
        match self.try_get(id.as_usize()) {
            Some(local) => f
                .debug_struct("Local")
                .field("thread", &id)
                .field("local", &*local)
                .finish(),
            None => f
                .debug_struct("Local")
                .field("thread", &id)
                .field("local", &format_args!("<uninitialized>"))
                .finish(),
        }
    }
}

// === impl LocalGuard ===

impl<'a, T> Deref for LocalGuard<'a, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        unsafe {
            // this is safe, as the `Local` only allows access to each slot
            // from a single thread.
            &*self.inner
        }
    }
}

impl<'a, T> DerefMut for LocalGuard<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe {
            // this is safe, as the `Local` only allows access to each slot
            // from a single thread.
            &mut *self.inner
        }
    }
}

impl<'a, T: fmt::Debug> fmt::Debug for LocalGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.deref().fmt(f)
    }
}

impl<'a, T: fmt::Display> fmt::Display for LocalGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.deref().fmt(f)
    }
}

// === impl Id ===

impl Id {
    pub(crate) fn current() -> Self {
        thread_local! {
            static MY_ID: Cell<Option<Id>> = Cell::new(None);
        }

        MY_ID
            .try_with(|my_id| my_id.get().unwrap_or_else(|| Self::new_thread(my_id)))
            .unwrap_or_else(|_| Self::poisoned())
    }

    pub(crate) fn as_usize(&self) -> usize {
        self.id
    }

    #[cold]
    fn new_thread(local: &Cell<Option<Id>>) -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::AcqRel);
        let tid = Self {
            id,
            _not_send: PhantomData,
        };
        local.set(Some(tid));
        tid
    }

    #[cold]
    fn poisoned() -> Self {
        Self {
            id: std::usize::MAX,
            _not_send: PhantomData,
        }
    }

    /// Returns true if the local thread ID was accessed while unwinding.
    pub(crate) fn is_poisoned(&self) -> bool {
        self.id == std::usize::MAX
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_poisoned() {
            f.debug_tuple("Id")
                .field(&format_args!("<poisoned>"))
                .finish()
        } else {
            f.debug_tuple("Id").field(&self.id).finish()
        }
    }
}
