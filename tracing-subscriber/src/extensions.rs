use crate::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault,Hasher};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Default)]
pub struct Directory {
    known: HashMap<TypeId, usize, BuildHasherDefault<IdHasher>>,
}

pub struct Key<T> {
    index: usize,
    ty: PhantomData<fn(T)>,
}

pub struct Extensions {
    values: Vec<RwLock<ExtensionValue>>,
}

type ExtensionValue = Option<Box<dyn Any + Send + Sync>>;

pub use self::imp::*;

/// With TypeIds as keys, there's no need to hash them. They are already hashes
/// themselves, coming from the compiler. The IdHasher holds the u64 of
/// the TypeId, and then returns it, instead of doing any bit fiddling.
#[derive(Default, Debug)]
struct IdHasher(u64);

impl Directory {
    pub fn register<T: Any + Send + Sync>(&mut self) -> Key<T> {
        let index = self
            .known
            .entry(TypeId::of::<T>())
            .or_insert(self.known.len() + 1);
        Key {
            index,
            _ty: PhantomData,
        }
    }

    pub fn extensions(&self) -> Extensions {
        Extensions {
            values: (0..self.len()).map(|_| RwLock::new(None)).collect(),
        }
    }
}

impl Extensions {
    pub fn get<'a, T>(&'a self, key: &Key<T>) -> Extension<'a, T> {
        let guard = self.values[key.index].read().unwrap();
        Extension::from_guard(guard)

    pub fn get_mut<'a, T>(&'a self, key: &Key<T>) -> ExtensionMut<'a, T> {
        let guard = self.values[key.index].write().unwrap();
        ExtensionMut::from_guard(guard)
    }
}

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

#[cfg(feature = "parking_lot")]
mod imp {
    use super::*;
    use parking_lot::{MappedRwLockReadGuard, MappedRwLockWriteGuard};
    pub struct Extension<'a, T> {
        guard: MappedRwLockReadGuard<'a, Option<T>>,
    }

    pub struct ExtensionMut<'a, T> {
        guard: MappedRwLockWriteGuard<'a, Option<T>>,
    }

    impl Extensions {
        pub fn get<'a, T>(&'a self, key: &Key<T>) -> Extension<'a, T> {
            let guard = self.values[key.index].read().unwrap();
            Extension { guard }
        }
    }


    impl<'a, T> Extension<'a, T> {
        #[inline]
        pub(super) fn from_guard(guard: RwLockReadGuard<'a, ExtensionValue>) -> Self {
            
            let guard = RwLockReadGuard::map(|val| val.map(|v| v.downcast_ref::<T>().expect("must downcast")))
            Extension { guard }
        }

    impl<'a, T> ExtensionMut<'a, T> {
        #[inline]
        pub(super) fn from_guard(guard: RwLockWriteGuard<'a, ExtensionValue>) -> Self {
            let guard = RwLockWriteGuard::map(|val| val.map(|v| v.downcast_mut::<T>().expect("must downcast")))
            Extension { guard, _ty: PhantomData }
        }
    }

}

#[cfg(not(feature = "parking_lot"))]
mod imp {
    use super::*;
    pub struct Extension<'a, T> {
        // XXX(eliza): when `parking_lot` is enabled, this should just use a
        // `MappedRwLockReadGuard`.
        guard: RwLockReadGuard<'a, ExtesionValue>,
        _ty: PhantomData<T>,
    }

    pub struct ExtensionMut<'a, T> {
        // XXX(eliza): when `parking_lot` is enabled, this should just use a
        // `MappedRwLockReadGuard`.
        guard: RwLockWriteGuard<'a, ExtesionValue>,
        _ty: PhantomData<T>,
    }

    impl<'a, T> Extension<'a, T> {
        #[inline]
        pub(super) fn from_guard(guard: RwLockReadGuard<'a, ExtensionValue>) -> Self {
            Extension { guard, _ty: PhantomData }
        }

    impl<'a, T> ExtensionMut<'a, T> {
        #[inline]
        pub(super) fn from_guard(guard: RwLockWriteGuard<'a, ExtensionValue>) -> Self {
            Extension { guard, _ty: PhantomData }
        }
    }
}
