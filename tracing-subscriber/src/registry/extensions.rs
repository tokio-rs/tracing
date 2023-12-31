// taken from https://github.com/hyperium/http/blob/master/src/extensions.rs.

use std::{
    any::TypeId,
    collections::{hash_map::Entry, HashMap},
    fmt,
    hash::{BuildHasherDefault, Hasher},
    mem, ptr,
};

use crate::registry::extensions::boxed_entry::{Aligned, BoxedEntry, BoxedEntryOp};
use crate::sync::{RwLockReadGuard, RwLockWriteGuard};

#[allow(unreachable_pub)]
mod boxed_entry {
    use std::mem::MaybeUninit;
    use std::ptr;
    use std::ptr::NonNull;

    /// Used to give ourselves one free bit for liveness checks.
    #[repr(align(2))]
    pub struct Aligned<T>(pub T);

    /// Wrapper around an untyped pointer to avoid casting bugs.
    ///
    /// This type holds the pointer to a `Box<Aligned<T>>`.
    #[derive(Copy, Clone)]
    pub struct ExtPtr(NonNull<()>);

    impl ExtPtr {
        fn new<T>(val: T) -> Self {
            let mut this = {
                let ptr = Box::into_raw(Box::new(Aligned(val)));
                Self(unsafe { NonNull::new_unchecked(ptr) }.cast())
            };
            this.set_live(true);
            this
        }

        pub fn typed<T>(self) -> NonNull<Aligned<T>> {
            #[cfg(not(nightly))]
            let ptr = {
                let addr = self.0.as_ptr() as usize;
                let addr = addr & !1; // Zero out the live bit
                addr as *mut _
            };
            #[cfg(nightly)]
            let ptr = self.0.as_ptr().mask(!1).cast();
            unsafe { NonNull::new_unchecked(ptr) }
        }

        pub fn live(&self) -> bool {
            #[cfg(not(nightly))]
            let addr = self.0.as_ptr() as usize;
            #[cfg(nightly)]
            let addr = self.0.as_ptr().addr();
            (addr & 1) == 1
        }

        pub fn set_live(&mut self, live: bool) {
            let update = |addr: usize| if live { addr | 1 } else { addr & !1 };
            #[cfg(not(nightly))]
            let ptr = update(self.0.as_ptr() as usize) as *mut _;
            #[cfg(nightly)]
            let ptr = self.0.as_ptr().map_addr(update);
            self.0 = unsafe { NonNull::new_unchecked(ptr) };
        }
    }

    /// Extension storage
    #[allow(clippy::manual_non_exhaustive)]
    pub struct BoxedEntry {
        pub ptr: ExtPtr,
        pub op_fn: unsafe fn(ExtPtr, BoxedEntryOp),
        _private: (),
    }

    unsafe impl Send for BoxedEntry {}

    unsafe impl Sync for BoxedEntry {}

    pub enum BoxedEntryOp {
        DropLiveValue,
        Drop,
    }

    unsafe fn run_boxed_entry_op<T>(ext: ExtPtr, op: BoxedEntryOp) {
        let ptr = ext.typed::<T>().as_ptr();
        match op {
            BoxedEntryOp::DropLiveValue => unsafe {
                ptr::drop_in_place(ptr);
            },
            BoxedEntryOp::Drop => {
                if ext.live() {
                    unsafe {
                        drop(Box::from_raw(ptr));
                    }
                } else {
                    unsafe {
                        drop(Box::from_raw(ptr.cast::<MaybeUninit<Aligned<T>>>()));
                    }
                }
            }
        }
    }

    impl BoxedEntry {
        pub fn new<T: Send + Sync + 'static>(val: T) -> Self {
            Self {
                ptr: ExtPtr::new(val),
                op_fn: run_boxed_entry_op::<T>,
                _private: (),
            }
        }
    }

    impl Drop for BoxedEntry {
        fn drop(&mut self) {
            unsafe {
                (self.op_fn)(self.ptr, BoxedEntryOp::Drop);
            }
        }
    }
}

#[allow(warnings)]
type AnyMap = HashMap<TypeId, BoxedEntry, BuildHasherDefault<IdHasher>>;

/// With TypeIds as keys, there's no need to hash them. They are already hashes
/// themselves, coming from the compiler. The IdHasher holds the u64 of
/// the TypeId, and then returns it, instead of doing any bit fiddling.
#[derive(Default, Debug)]
struct IdHasher(u64);

impl Hasher for IdHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("TypeId calls write_u64");
    }

    #[inline]
    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

/// An immutable, read-only reference to a Span's extensions.
#[derive(Debug)]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub struct Extensions<'a> {
    inner: RwLockReadGuard<'a, ExtensionsInner>,
}

impl<'a> Extensions<'a> {
    #[cfg(feature = "registry")]
    pub(crate) fn new(inner: RwLockReadGuard<'a, ExtensionsInner>) -> Self {
        Self { inner }
    }

    /// Immutably borrows a type previously inserted into this `Extensions`.
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.inner.get::<T>()
    }
}

/// An mutable reference to a Span's extensions.
#[derive(Debug)]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub struct ExtensionsMut<'a> {
    inner: RwLockWriteGuard<'a, ExtensionsInner>,
}

impl<'a> ExtensionsMut<'a> {
    #[cfg(feature = "registry")]
    pub(crate) fn new(inner: RwLockWriteGuard<'a, ExtensionsInner>) -> Self {
        Self { inner }
    }

    /// Insert a type into this `Extensions`.
    ///
    /// Note that extensions are _not_
    /// [subscriber]-specific—they are _span_-specific. This means that
    /// other subscribers can access and mutate extensions that
    /// a different Subscriber recorded. For example, an application might
    /// have a subscriber that records execution timings, alongside a subscriber
    /// that reports spans and events to a distributed
    /// tracing system that requires timestamps for spans.
    /// Ideally, if one subscriber records a timestamp _x_, the other subscriber
    /// should be able to reuse timestamp _x_.
    ///
    /// Therefore, extensions should generally be newtypes, rather than common
    /// types like [`String`](std::string::String), to avoid accidental
    /// cross-`Subscriber` clobbering.
    ///
    /// ## Panics
    ///
    /// If `T` is already present in `Extensions`, then this method will panic.
    ///
    /// [subscriber]: crate::subscribe::Subscribe
    pub fn insert<T: Send + Sync + 'static>(&mut self, val: T) {
        assert!(self.replace(val).is_none())
    }

    /// Replaces an existing `T` into this extensions.
    ///
    /// If `T` is not present, `Option::None` will be returned.
    pub fn replace<T: Send + Sync + 'static>(&mut self, val: T) -> Option<T> {
        self.inner.insert(val)
    }

    /// Get a mutable reference to a type previously inserted on this `ExtensionsMut`.
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.inner.get_mut::<T>()
    }

    /// Remove a type from this `Extensions`.
    ///
    /// If a extension of this type existed, it will be returned.
    pub fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.inner.remove::<T>()
    }
}

/// A type map of span extensions.
///
/// [ExtensionsInner] is used by [Data] to store and
/// span-specific data. A given [Subscriber] can read and write
/// data that it is interested in recording and emitting.
#[derive(Default)]
pub(crate) struct ExtensionsInner {
    map: AnyMap,
}

impl ExtensionsInner {
    /// Create an empty `Extensions`.
    #[cfg(any(test, feature = "registry"))]
    #[inline]
    pub(crate) fn new() -> ExtensionsInner {
        ExtensionsInner {
            map: AnyMap::default(),
        }
    }

    /// Insert a type into this `Extensions`.
    ///
    /// If a extension of this type already existed, it will
    /// be returned.
    pub(crate) fn insert<T: Send + Sync + 'static>(&mut self, val: T) -> Option<T> {
        match self.map.entry(TypeId::of::<T>()) {
            Entry::Occupied(mut entry) => {
                let entry = entry.get_mut();
                let mut ptr = entry.ptr.typed::<T>();
                if entry.ptr.live() {
                    Some(mem::replace(unsafe { ptr.as_mut() }, Aligned(val)).0)
                } else {
                    unsafe {
                        ptr::write(ptr.as_ptr(), Aligned(val));
                    }
                    entry.ptr.set_live(true);
                    None
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(BoxedEntry::new(val));
                None
            }
        }
    }

    /// Get a reference to a type previously inserted on this `Extensions`.
    pub(crate) fn get<T: 'static>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .filter(|boxed| boxed.ptr.live())
            .map(|boxed| &unsafe { boxed.ptr.typed::<T>().as_ref() }.0)
    }

    /// Get a mutable reference to a type previously inserted on this `Extensions`.
    pub(crate) fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.map
            .get_mut(&TypeId::of::<T>())
            .filter(|boxed| boxed.ptr.live())
            .map(|boxed| &mut unsafe { boxed.ptr.typed::<T>().as_mut() }.0)
    }

    /// Remove a type from this `Extensions`.
    ///
    /// If a extension of this type existed, it will be returned.
    pub(crate) fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.map
            .get_mut(&TypeId::of::<T>())
            .filter(|boxed| boxed.ptr.live())
            .map(|boxed| {
                boxed.ptr.set_live(false);
                unsafe { ptr::read(boxed.ptr.typed::<T>().as_ptr()) }.0
            })
    }

    /// Clear the `ExtensionsInner` in-place, dropping any elements in the map but
    /// retaining allocated capacity.
    ///
    /// This permits the hash map allocation to be pooled by the registry so
    /// that future spans will not need to allocate new hashmaps.
    #[cfg(any(test, feature = "registry"))]
    pub(crate) fn clear(&mut self) {
        for boxed in self.map.values_mut().filter(|boxed| boxed.ptr.live()) {
            boxed.ptr.set_live(false);
            unsafe {
                (boxed.op_fn)(boxed.ptr, BoxedEntryOp::DropLiveValue);
            }
        }
    }
}

impl fmt::Debug for ExtensionsInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Extensions")
            .field("len", &self.map.len())
            .field("capacity", &self.map.capacity())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct MyType(i32);

    #[test]
    fn test_extensions() {
        let mut extensions = ExtensionsInner::new();

        extensions.insert(5i32);
        extensions.insert(MyType(10));

        assert_eq!(extensions.get(), Some(&5i32));
        assert_eq!(extensions.get_mut(), Some(&mut 5i32));

        assert_eq!(extensions.remove::<i32>(), Some(5i32));
        assert!(extensions.get::<i32>().is_none());

        assert_eq!(extensions.get::<bool>(), None);
        assert_eq!(extensions.get(), Some(&MyType(10)));
    }

    #[test]
    fn clear_retains_capacity() {
        let mut extensions = ExtensionsInner::new();
        extensions.insert(5i32);
        extensions.insert(MyType(10));
        extensions.insert(true);

        assert_eq!(extensions.map.len(), 3);
        let prev_capacity = extensions.map.capacity();
        extensions.clear();

        assert_eq!(
            extensions.map.len(),
            3,
            "after clear(), extensions map should have length 3"
        );
        assert_eq!(
            extensions.map.capacity(),
            prev_capacity,
            "after clear(), extensions map should retain prior capacity"
        );
    }

    #[test]
    fn clear_drops_elements() {
        use std::sync::Arc;
        struct DropMePlease(Arc<()>);
        struct DropMeTooPlease(Arc<()>);

        let mut extensions = ExtensionsInner::new();
        let val1 = DropMePlease(Arc::new(()));
        let val2 = DropMeTooPlease(Arc::new(()));

        let val1_dropped = Arc::downgrade(&val1.0);
        let val2_dropped = Arc::downgrade(&val2.0);
        extensions.insert(val1);
        extensions.insert(val2);

        assert!(val1_dropped.upgrade().is_some());
        assert!(val2_dropped.upgrade().is_some());

        extensions.clear();
        assert!(
            val1_dropped.upgrade().is_none(),
            "after clear(), val1 should be dropped"
        );
        assert!(
            val2_dropped.upgrade().is_none(),
            "after clear(), val2 should be dropped"
        );
    }
}
